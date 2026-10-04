//! DB-less sync (owner requirement): devices are interconnected in real time,
//! changes propagate as they happen, and offline devices catch up on reconnect.
//! No server-side database: peers exchange manifests and file payloads directly.
//!
//! Two transports share one reconcile rule:
//! - **LAN:** UDP broadcast discovery → TCP JSON-lines sessions.
//! - **Folder:** any synced folder (Syncthing/Dropbox/Obsidian vault) reconciled
//!   two-way on a poll; gives internet sync through whatever the user already uses.
//!
//! Merge rule everywhere: newer mtime wins; equal mtime + different hash is kept
//! as a `.conflict.<device>.<epoch>` copy — nothing is silently dropped.

use std::collections::BTreeMap;
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream, UdpSocket};
use std::path::{Path, PathBuf};

pub const DISCOVERY_MAGIC: &str = "PLAINPAD1";
pub const DISCOVERY_PORT: u16 = 50815;
pub const TCP_PORT_DEFAULT: u16 = 50816;

/// name → (content_hash, mtime_ms)
pub type Manifest = BTreeMap<String, (u64, u64)>;
pub type DeviceId = String;

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

pub fn content_hash(bytes: &[u8]) -> u64 {
    fnv1a64(bytes)
}

// ---------------------------------------------------------------------------
// Shared reconcile primitives (also used by the folder transport)
// ---------------------------------------------------------------------------

/// Decide what to do with a remote file vs a local one.
#[derive(Debug, PartialEq)]
pub enum MergeAction {
    KeepLocal,
    TakeRemote,
    Conflict,
}

pub fn decide(local: Option<(u64, u64)>, remote: (u64, u64)) -> MergeAction {
    match local {
        None => MergeAction::TakeRemote,
        Some((lh, lm)) => {
            if lh == remote.0 {
                MergeAction::KeepLocal
            } else if remote.1 > lm {
                MergeAction::TakeRemote
            } else if remote.1 < lm {
                MergeAction::KeepLocal
            } else {
                MergeAction::Conflict
            }
        }
    }
}

/// Diff two manifests: what we should send (we are newer / they lack it) and
/// what we should request (they are newer / we lack it).
pub fn diff_manifests(local: &Manifest, remote: &Manifest) -> (Vec<String>, Vec<String>) {
    let mut send = Vec::new();
    let mut want = Vec::new();
    for (name, r) in remote {
        match local.get(name) {
            Some(l) => match decide(Some(*l), *r) {
                MergeAction::TakeRemote => want.push(name.clone()),
                MergeAction::KeepLocal => send.push(name.clone()),
                MergeAction::Conflict => want.push(name.clone()), // remote copy arrives as conflict file
            },
            None => want.push(name.clone()),
        }
    }
    for (name, l) in local {
        if !remote.contains_key(name) {
            send.push(name.clone());
        } else if let Some(r) = remote.get(name) {
            if decide(Some(*l), *r) == MergeAction::KeepLocal {
                // already queued in the loop above
            }
        }
    }
    (send, want)
}

// ---------------------------------------------------------------------------
// Folder transport: two-way reconcile against any synced directory
// ---------------------------------------------------------------------------

pub struct FolderSync {
    pub dir: PathBuf,
    pub device: DeviceId,
}

impl FolderSync {
    /// One reconcile pass: pull newer/missing files in, push local ones out,
    /// keep conflicts as copies. Returns the names that changed locally.
    pub fn reconcile(&self, store_dir: &Path) -> std::io::Result<Vec<String>> {
        fs::create_dir_all(&self.dir)?;
        let remote = self.scan(&self.dir)?;
        let local = self.scan(store_dir)?;
        let mut changed = Vec::new();

        for name in remote.keys() {
            let (send, want) = diff_manifests(&local, &remote);
            let _ = send;
            if want.contains(name) && self.pull_one(name, store_dir)? {
                changed.push(name.clone());
            }
        }
        let local2 = self.scan(store_dir)?;
        for name in local2.keys() {
            let remote_now = self.scan(&self.dir)?;
            let (send, _want) = diff_manifests(&local2, &remote_now);
            if send.contains(name) && self.push_one(name, store_dir)? {
                changed.push(name.clone());
            }
        }
        Ok(changed)
    }

    fn scan(&self, dir: &Path) -> std::io::Result<Manifest> {
        let mut m = Manifest::new();
        if !dir.exists() {
            return Ok(m);
        }
        for e in fs::read_dir(dir)? {
            let p = e?.path();
            if p.is_file() && p.extension().is_some_and(|x| x == "txt") {
                let name = p.file_name().unwrap().to_string_lossy().to_string();
                let bytes = fs::read(&p)?;
                let mtime = fs::metadata(&p)?
                    .modified()?
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                m.insert(name, (content_hash(&bytes), mtime));
            }
        }
        Ok(m)
    }

    fn pull_one(&self, name: &str, store_dir: &Path) -> std::io::Result<bool> {
        let src = self.dir.join(name);
        let dst = store_dir.join(name);
        let incoming = fs::read(&src)?;
        let action = match fs::read(&dst) {
            Ok(local_bytes) => decide(
                Some((content_hash(&local_bytes), mtime_of(&dst))),
                (content_hash(&incoming), mtime_of(&src)),
            ),
            Err(_) => MergeAction::TakeRemote,
        };
        match action {
            MergeAction::KeepLocal => Ok(false),
            MergeAction::TakeRemote => {
                write_atomic(&dst, &incoming)?;
                Ok(true)
            }
            MergeAction::Conflict => {
                let conflict = format!(
                    "{}.conflict.{}.{}.txt",
                    name.trim_end_matches(".txt"),
                    sanitize(&self.device),
                    mtime_of(&src)
                );
                write_atomic(&store_dir.join(conflict), &incoming)?;
                Ok(true)
            }
        }
    }

    fn push_one(&self, name: &str, store_dir: &Path) -> std::io::Result<bool> {
        let src = store_dir.join(name);
        let dst = self.dir.join(name);
        let bytes = fs::read(&src)?;
        match fs::read(&dst) {
            Ok(remote_bytes) => {
                let action = decide(
                    Some((content_hash(&remote_bytes), mtime_of(&dst))),
                    (content_hash(&bytes), mtime_of(&src)),
                );
                match action {
                    MergeAction::KeepLocal => {
                        write_atomic(&dst, &bytes)?;
                        Ok(true)
                    }
                    MergeAction::TakeRemote => Ok(false), // folder already has newer
                    MergeAction::Conflict => {
                        let conflict = format!(
                            "{}.conflict.{}.{}.txt",
                            name.trim_end_matches(".txt"),
                            sanitize(&self.device),
                            mtime_of(&src)
                        );
                        write_atomic(&self.dir.join(conflict), &bytes)?;
                        Ok(true)
                    }
                }
            }
            Err(_) => {
                write_atomic(&dst, &bytes)?;
                Ok(true)
            }
        }
    }
}

fn mtime_of(p: &Path) -> u64 {
    fs::metadata(p)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension(format!("tmp{}", std::process::id()));
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path)
}

fn sanitize(s: &str) -> String {
    s.chars().map(|c| if c.is_ascii_alphanumeric() || c == '-' { c } else { '_' }).collect()
}

// ---------------------------------------------------------------------------
// LAN transport: UDP discovery + TCP JSON-lines sessions
// ---------------------------------------------------------------------------

#[derive(serde::Serialize, serde::Deserialize)]
struct Hello {
    device: DeviceId,
    manifest: Manifest,
}

#[derive(serde::Serialize, serde::Deserialize)]
#[allow(dead_code)]
struct Request {
    names: Vec<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Payload {
    name: String,
    b64: String,
    mtime: u64,
}

/// Serve one connection: exchange HELLOs, send what they want, take what we want.
fn serve_session(stream: TcpStream, store_dir: PathBuf, device: DeviceId) -> std::io::Result<()> {
    let mut writer = stream.try_clone()?;
    let mut reader = BufReader::new(stream);

    let local_manifest = scan_dir(&store_dir)?;
    let hello = Hello { device, manifest: local_manifest };
    writeln!(writer, "{}", serde_json::to_string(&hello).unwrap_or_default())?;

    let mut line = String::new();
    if reader.read_line(&mut line)? == 0 {
        return Ok(());
    }
    let their: Hello = serde_json::from_str(line.trim()).unwrap_or(Hello {
        device: "unknown".into(),
        manifest: Manifest::new(),
    });

    let (send, want) = diff_manifests(&scan_dir(&store_dir)?, &their.manifest);

    // Send requested files first.
    for name in &send {
        if let Ok(bytes) = fs::read(store_dir.join(name)) {
            let p = Payload { name: name.clone(), b64: b64_encode(&bytes), mtime: mtime_of(&store_dir.join(name)) };
            writeln!(writer, "{}", serde_json::to_string(&p).unwrap_or_default())?;
        }
    }
    writeln!(writer)?;
    writer.flush()?;

    // Receive files they push (they computed their own `send` from our hello).
    let _ = want;
    loop {
        line.clear();
        if reader.read_line(&mut line)? == 0 {
            break;
        }
        let t = line.trim();
        if t.is_empty() {
            break;
        }
        if let Ok(p) = serde_json::from_str::<Payload>(t) {
            apply_incoming(&store_dir, &p, &their.device);
        }
    }
    Ok(())
}

fn apply_incoming(store_dir: &Path, p: &Payload, from_device: &str) {
    let Ok(bytes) = b64_decode(&p.b64) else { return };
    let dst = store_dir.join(&p.name);
    let action = match fs::read(&dst) {
        Ok(local_bytes) => decide(
            Some((content_hash(&local_bytes), mtime_of(&dst))),
            (content_hash(&bytes), p.mtime),
        ),
        Err(_) => MergeAction::TakeRemote,
    };
    match action {
        MergeAction::KeepLocal => {}
        MergeAction::TakeRemote => {
            write_atomic(&dst, &bytes).ok();
        }
        MergeAction::Conflict => {
            let conflict = format!(
                "{}.conflict.{}.{}.txt",
                p.name.trim_end_matches(".txt"),
                sanitize(from_device),
                p.mtime
            );
            write_atomic(&store_dir.join(conflict), &bytes).ok();
        }
    }
}

fn scan_dir(dir: &Path) -> std::io::Result<Manifest> {
    FolderSync { dir: dir.to_path_buf(), device: String::new() }.scan(dir)
}

/// Minimal standard base64 (RFC 4648) — avoids an extra dependency in the shell.
pub fn b64_encode(data: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [chunk[0], *chunk.get(1).unwrap_or(&0), *chunk.get(2).unwrap_or(&0)];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

pub fn b64_decode(s: &str) -> Result<Vec<u8>, String> {
    fn val(c: u8) -> Result<u32, String> {
        match c {
            b'A'..=b'Z' => Ok((c - b'A') as u32),
            b'a'..=b'z' => Ok((c - b'a' + 26) as u32),
            b'0'..=b'9' => Ok((c - b'0' + 52) as u32),
            b'+' => Ok(62),
            b'/' => Ok(63),
            _ => Err("bad base64".into()),
        }
    }
    let bytes: Vec<u8> = s.bytes().filter(|b| !b.is_ascii_whitespace() && *b != b'=').collect();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    for chunk in bytes.chunks(4) {
        let mut n: u32 = 0;
        for (i, c) in chunk.iter().enumerate() {
            n |= val(*c)? << (18 - 6 * i);
        }
        out.push((n >> 16) as u8);
        if chunk.len() > 2 {
            out.push((n >> 8) as u8);
        }
        if chunk.len() > 3 {
            out.push(n as u8);
        }
    }
    Ok(out)
}

/// Background LAN sync: listens for peers and announces this device.
/// Threads are detached by design; the desktop shell keeps one per app lifetime.
pub struct LanSyncHandle {
    pub tcp_port: u16,
}

/// Start LAN sync: TCP listener for sessions + UDP discovery (beacon + respond).
pub fn start_lan_sync(
    store_dir: PathBuf,
    device: DeviceId,
    tcp_port: u16,
) -> std::io::Result<LanSyncHandle> {
    fs::create_dir_all(&store_dir)?;

    // TCP listener: accept peer sessions forever.
    let listener = TcpListener::bind(("0.0.0.0", tcp_port))?;
    let dir = store_dir.clone();
    let dev = device.clone();
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            let Ok(stream) = stream else { continue };
            let dir = dir.clone();
            let dev = dev.clone();
            std::thread::spawn(move || {
                let _ = serve_session(stream, dir, dev);
            });
        }
    });

    // UDP discovery: announce every loop (~1.5 s) and connect to beacons we hear.
    let sock = UdpSocket::bind(("0.0.0.0", DISCOVERY_PORT))?;
    sock.set_read_timeout(Some(std::time::Duration::from_millis(1500))).ok();
    sock.set_broadcast(true).ok();
    let dir = store_dir.clone();
    std::thread::spawn(move || {
        let mut buf = [0u8; 256];
        loop {
            match sock.recv_from(&mut buf) {
                Ok((n, addr)) => {
                    let msg = String::from_utf8_lossy(&buf[..n]);
                    let parts: Vec<&str> = msg.trim().split('|').collect();
                    if parts.len() == 3 && parts[0] == DISCOVERY_MAGIC {
                        let their_port: u16 = parts[1].parse().unwrap_or(0);
                        let their_id = parts[2].to_string();
                        if their_id != device && their_port > 0 {
                            let dir = dir.clone();
                            let dev = device.clone();
                            let ip = addr.ip();
                            std::thread::spawn(move || {
                                let target = format!("{ip}:{their_port}");
                                if let Ok(stream) = TcpStream::connect_timeout(
                                    &target.parse().unwrap(),
                                    std::time::Duration::from_secs(2),
                                ) {
                                    let _ = serve_session(stream, dir, dev);
                                }
                            });
                        }
                    }
                }
                Err(_) => { /* timeout: fall through to announcing */ }
            }
            let beacon = format!("{DISCOVERY_MAGIC}|{tcp_port}|{device}");
            let _ = sock.send_to(beacon.as_bytes(), ("255.255.255.255", DISCOVERY_PORT));
            let _ = sock.send_to(beacon.as_bytes(), ("127.0.0.255", DISCOVERY_PORT));
        }
    });

    Ok(LanSyncHandle { tcp_port })
}

/// One-shot client session to a known peer (used on manual refresh and tests).
pub fn connect_to_peer(store_dir: PathBuf, device: DeviceId, addr: &str) -> std::io::Result<()> {
    let stream = TcpStream::connect_timeout(
        &addr.parse().map_err(|_| std::io::Error::other("bad addr"))?,
        std::time::Duration::from_secs(3),
    )?;
    serve_session(stream, store_dir, device)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("plainpad-sync-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn b64_roundtrip_matches_known_vectors() {
        assert_eq!(b64_encode(b""), "");
        assert_eq!(b64_encode(b"f"), "Zg==");
        assert_eq!(b64_encode(b"fo"), "Zm8=");
        assert_eq!(b64_encode(b"foo"), "Zm9v");
        assert_eq!(b64_encode(b"foobar"), "Zm9vYmFy");
        assert_eq!(b64_decode("Zm9vYmFy").unwrap(), b"foobar");
        assert_eq!(b64_decode(&b64_encode(&[0u8, 255, 10, 33])).unwrap(), vec![0u8, 255, 10, 33]);
    }

    #[test]
    fn merge_decisions() {
        assert_eq!(decide(None, (1, 5)), MergeAction::TakeRemote);
        assert_eq!(decide(Some((1, 5)), (1, 5)), MergeAction::KeepLocal); // same hash
        assert_eq!(decide(Some((1, 9)), (2, 5)), MergeAction::KeepLocal); // local newer
        assert_eq!(decide(Some((1, 5)), (2, 9)), MergeAction::TakeRemote);
        assert_eq!(decide(Some((1, 7)), (2, 7)), MergeAction::Conflict); // same time, diff content
        assert_eq!(decide(Some((1, 8)), (1, 8)), MergeAction::KeepLocal); // equal, same hash
    }

    #[test]
    fn folder_sync_pull_push_and_conflict_copy() {
        let store = tmp("folder-store");
        let vault = tmp("folder-vault");

        // Remote has a newer file we lack → pull.
        fs::write(vault.join("a.txt"), "from laptop").unwrap();
        let f = FolderSync { dir: vault.clone(), device: "phone".into() };
        let changed = f.reconcile(&store).unwrap();
        assert!(changed.contains(&"a.txt".to_string()));
        assert_eq!(fs::read_to_string(store.join("a.txt")).unwrap(), "from laptop");

        // Local has a file the folder lacks → push.
        fs::write(store.join("b.txt"), "from desktop").unwrap();
        f.reconcile(&store).unwrap();
        assert_eq!(fs::read_to_string(vault.join("b.txt")).unwrap(), "from desktop");

        // Same mtime, different content → conflict copy on the store side.
        let t = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
        set_mtime(&vault.join("a.txt"), t);
        set_mtime(&store.join("a.txt"), t);
        fs::write(store.join("a.txt"), "edited locally").unwrap();
        set_mtime(&store.join("a.txt"), t);
        f.reconcile(&store).unwrap();
        let entries: Vec<_> = fs::read_dir(&store)
            .unwrap()
            .filter_map(|e| e.ok())
            .map(|e| e.file_name().to_string_lossy().to_string())
            .collect();
        assert!(entries.iter().any(|n| n.contains(".conflict.")), "entries: {entries:?}");

        let _ = fs::remove_dir_all(&store);
        let _ = fs::remove_dir_all(&vault);
    }

    fn set_mtime(p: &Path, t: std::time::SystemTime) {
        use std::io::Write;
        let mut f = fs::OpenOptions::new().write(true).open(p).unwrap();
        f.set_modified(t).unwrap();
        f.flush().ok();
    }

    #[test]
    fn lan_sync_two_instances_on_loopback() {
        let store1 = tmp("lan1");
        let store2 = tmp("lan2");
        fs::write(store1.join("note1.txt"), "hello from 1").unwrap();

        // Device one runs the full LAN stack (UDP discovery + TCP server).
        let _h1 = start_lan_sync(store1.clone(), "device-one".into(), 50826).unwrap();
        // Device two runs a TCP server only (UDP port would clash within the
        // same process — on real devices each machine binds it once).
        let listener2 = TcpListener::bind(("0.0.0.0", 50827)).unwrap();
        {
            let dir = store2.clone();
            std::thread::spawn(move || {
                for stream in listener2.incoming() {
                    let Ok(stream) = stream else { continue };
                    let dir = dir.clone();
                    std::thread::spawn(move || {
                        let _ = serve_session(stream, dir, "device-two".into());
                    });
                }
            });
        }

        // Explicit session one → two (production does this via UDP discovery).
        connect_to_peer(store1.clone(), "device-one".into(), "127.0.0.1:50827").unwrap();
        // Sessions are symmetric: device two now has note1, device one any files
        // two already had. Push one from two and sync back for the reverse dir.
        fs::write(store2.join("note2.txt"), "hello from 2").unwrap();
        connect_to_peer(store1.clone(), "device-one".into(), "127.0.0.1:50827").unwrap();

        assert_eq!(fs::read_to_string(store2.join("note1.txt")).unwrap(), "hello from 1");
        assert_eq!(fs::read_to_string(store1.join("note2.txt")).unwrap(), "hello from 2");
        let _ = fs::remove_dir_all(&store1);
        let _ = fs::remove_dir_all(&store2);
    }
}
