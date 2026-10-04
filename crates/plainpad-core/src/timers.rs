//! Timers: countdowns started from plain phrases, persisted to disk so they
//! survive window close AND app restart (PRD R-2). A watcher thread marks due
//! timers and emits events; timers that expired while the app was away fire
//! immediately on load.

use crate::now_ms;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
pub struct Timer {
    pub id: u64,
    pub name: String,
    pub due_ms: u64,
    pub duration_ms: u64,
    pub fired: bool,
}

#[derive(serde::Serialize, Clone, Debug)]
pub struct FiredTimer {
    pub id: u64,
    pub name: String,
    /// True when the timer expired while the app was not running.
    pub overdue: bool,
}

struct Inner {
    timers: Vec<Timer>,
    next_id: u64,
}

/// Persistent timer engine. `FileStore` supplies the JSON sidecar path.
pub struct TimerEngine {
    state: Arc<Mutex<Inner>>,
    path: PathBuf,
    stop: Arc<std::sync::atomic::AtomicBool>,
}

impl TimerEngine {
    /// Load persisted timers; anything already due but unfired is reported as
    /// fired immediately ("overdue") so a reminder is never lost.
    pub fn load(path: &PathBuf) -> (Self, Vec<FiredTimer>) {
        let timers: Vec<Timer> = std::fs::read(path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default();
        let now = now_ms();
        let mut overdue = Vec::new();
        let mut timers: Vec<Timer> = timers
            .into_iter()
            .map(|mut t| {
                if !t.fired && t.due_ms <= now {
                    t.fired = true;
                    overdue.push(FiredTimer { id: t.id, name: t.name.clone(), overdue: true });
                }
                t
            })
            .collect();
        let next_id = timers.iter().map(|t| t.id).max().unwrap_or(0) + 1;
        let engine = Self {
            state: Arc::new(Mutex::new(Inner { timers: std::mem::take(&mut timers), next_id })),
            path: path.clone(),
            stop: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        };
        engine.persist();
        (engine, overdue)
    }

    fn persist(&self) {
        let guard = self.state.lock().unwrap();
        if let Ok(bytes) = serde_json::to_vec_pretty(&guard.timers) {
            let tmp = self.path.with_extension(format!("tmp{}", std::process::id()));
            if std::fs::write(&tmp, &bytes).is_ok() {
                std::fs::rename(&tmp, &self.path).ok();
            }
        }
    }

    /// Start a named countdown; returns the timer id.
    pub fn start(&self, name: &str, duration_ms: u64) -> u64 {
        let now = now_ms();
        let id = {
            let mut g = self.state.lock().unwrap();
            let id = g.next_id;
            g.next_id += 1;
            g.timers.push(Timer {
                id,
                name: name.to_string(),
                due_ms: now + duration_ms,
                duration_ms,
                fired: false,
            });
            id
        };
        self.persist();
        id
    }

    pub fn cancel(&self, id: u64) -> bool {
        let removed = {
            let mut g = self.state.lock().unwrap();
            let before = g.timers.len();
            g.timers.retain(|t| t.id != id);
            before != g.timers.len()
        };
        self.persist();
        removed
    }

    pub fn list(&self) -> Vec<Timer> {
        self.state.lock().unwrap().timers.clone()
    }

    /// Drain timers that became due since the last poll (watcher thread body:
    /// poll every ~500 ms; shell turns results into OS notifications).
    pub fn poll_due(&self) -> Vec<FiredTimer> {
        let now = now_ms();
        let mut fired = Vec::new();
        {
            let mut g = self.state.lock().unwrap();
            for t in g.timers.iter_mut() {
                if !t.fired && t.due_ms <= now {
                    t.fired = true;
                    fired.push(FiredTimer { id: t.id, name: t.name.clone(), overdue: false });
                }
            }
            g.timers.retain(|t| !(t.fired && now.saturating_sub(t.due_ms) > 60 * 60 * 1000));
        }
        if !fired.is_empty() {
            self.persist();
        }
        fired
    }

    /// Spawn the watcher thread that polls for due timers. `on_fire` is called
    /// on that thread; the shell bridges it to UI/OS notifications.
    pub fn spawn_watcher(&self, on_fire: impl Fn(FiredTimer) + Send + 'static) {
        let engine = Self {
            state: Arc::clone(&self.state),
            path: self.path.clone(),
            stop: Arc::clone(&self.stop),
        };
        std::thread::spawn(move || {
            while !engine.stop.load(std::sync::atomic::Ordering::Relaxed) {
                for f in engine.poll_due() {
                    on_fire(f);
                }
                std::thread::sleep(Duration::from_millis(500));
            }
        });
    }

    pub fn shutdown(&self) {
        self.stop.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Parse plain-English timer phrases:
///   `timer 25 min`, `timer 10 minutes pasta`, `remind me in 10 minutes`,
///   `timer 1h 30m`, `timer 90s`
/// Returns (name, duration_ms) or None when the line is not a timer phrase.
pub fn parse_timer_phrase(line: &str) -> Option<(String, u64)> {
    let t = line.trim();
    let lower = t.to_lowercase();
    let rest = lower
        .strip_prefix("timer")
        .or_else(|| lower.strip_prefix("remind me in"))
        .or_else(|| lower.strip_prefix("remind in"))?;
    if rest.trim().is_empty() {
        return None;
    }
    // Scan tokens: durations may be fused ("25min", "1h") or split ("25 min",
    // "10 minutes", "5"), colon format ("3:30"), or "pomo".
    let tokens: Vec<&str> = rest.split_whitespace().collect();
    let mut ms_total: u64 = 0;
    let mut name_parts: Vec<String> = Vec::new();
    let mut found_any = false;
    let mut pending_number: Option<u64> = None;
    for token in tokens {
        if token == "pomo" || token == "pomodoro" {
            ms_total += 25 * 60_000;
            found_any = true;
            name_parts.push("Pomodoro".to_string());
            continue;
        }
        // Support "3:30" (3 mins 30 secs) or "1:30:00"
        if let Some((m_str, s_str)) = token.split_once(':') {
            if let (Ok(m), Ok(s)) = (m_str.parse::<u64>(), s_str.parse::<u64>()) {
                ms_total += (m * 60 + s) * 1000;
                found_any = true;
                continue;
            }
        }

        if let Some(v) = parse_duration_token(token) {
            ms_total += v;
            found_any = true;
            pending_number = None;
        } else if token.chars().all(|c| c.is_ascii_digit()) && !token.is_empty() {
            pending_number = token.parse::<u64>().ok();
        } else if let Some(n) = pending_number.take() {
            // Try fusing the pending number with this unit word: "10" + "minutes".
            match parse_duration_token(&format!("{n}{token}")) {
                Some(v) => {
                    ms_total += v;
                    found_any = true;
                }
                None => {
                    name_parts.push(n.to_string());
                    name_parts.push(token.to_string());
                }
            }
        } else {
            name_parts.push(token.to_string());
        }
    }
    if let Some(n) = pending_number {
        if !found_any {
            // "timer 5" -> 5 minutes by default
            ms_total += n * 60_000;
            found_any = true;
        } else {
            name_parts.push(n.to_string());
        }
    }
    if !found_any || ms_total == 0 {
        return None;
    }
    let name = name_parts.join(" ").trim().to_string();
    Some((if name.is_empty() { "Timer".to_string() } else { name }, ms_total))
}

/// Detect phrases that stop or cancel running timers (Antinote: "timer s", "timer stop")
pub fn is_timer_stop_phrase(line: &str) -> bool {
    let t = line.trim().to_lowercase();
    t == "timer s"
        || t == "timer stop"
        || t == "timer cancel"
        || t == "stop timer"
        || t == "cancel timer"
}

fn parse_duration_token(token: &str) -> Option<u64> {
    let t = token.trim().trim_end_matches(['.', ',']);
    let split = t.find(|c: char| c.is_alphabetic())?;
    let (num, unit) = t.split_at(split);
    let n: u64 = num.trim().parse().ok()?;
    let ms = match unit.trim().to_lowercase().as_str() {
        "s" | "sec" | "secs" | "second" | "seconds" => n * 1000,
        "m" | "min" | "mins" | "minute" | "minutes" => n * 60_000,
        "h" | "hr" | "hrs" | "hour" | "hours" => n * 3_600_000,
        "d" | "day" | "days" => n * 86_400_000,
        _ => return None,
    };
    Some(ms)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmppath(tag: &str) -> PathBuf {
        std::env::temp_dir().join(format!("plainpad-timers-{tag}-{}.json", std::process::id()))
    }

    #[test]
    fn phrase_parsing() {
        assert_eq!(parse_timer_phrase("timer 25 min").unwrap(), ("Timer".into(), 25 * 60_000));
        assert_eq!(
            parse_timer_phrase("remind me in 10 minutes").unwrap(),
            ("Timer".into(), 10 * 60_000)
        );
        assert_eq!(
            parse_timer_phrase("timer 1h 30m pasta water").unwrap(),
            ("pasta water".into(), 90 * 60_000)
        );
        assert_eq!(parse_timer_phrase("timer 5").unwrap(), ("Timer".into(), 5 * 60_000));
        assert_eq!(parse_timer_phrase("timer 3:30").unwrap(), ("Timer".into(), 210_000));
        assert_eq!(parse_timer_phrase("timer pomo").unwrap(), ("Pomodoro".into(), 25 * 60_000));
        assert!(parse_timer_phrase("buy milk").is_none());
        assert!(parse_timer_phrase("timer").is_none());
    }

    #[test]
    fn start_poll_cancel_and_persistence() {
        let path = tmppath("cycle");
        let _ = std::fs::remove_file(&path);
        let (engine, overdue) = TimerEngine::load(&path);
        assert!(overdue.is_empty());
        let id = engine.start("tea", 50);
        assert!(engine.list().len() == 1);
        assert!(engine.poll_due().is_empty()); // not due yet
        std::thread::sleep(Duration::from_millis(70));
        let fired = engine.poll_due();
        assert_eq!(fired.len(), 1);
        assert_eq!(fired[0].name, "tea");
        assert!(engine.cancel(id));
        assert!(engine.list().is_empty());
        engine.shutdown();
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn timers_survive_restart_and_fire_overdue() {
        let path = tmppath("restart");
        let _ = std::fs::remove_file(&path);
        let (engine, _) = TimerEngine::load(&path);
        engine.start("short", 20);
        // Simulate restart: drop, wait past expiry, reload.
        engine.shutdown();
        drop(engine);
        std::thread::sleep(Duration::from_millis(40));
        let (engine2, overdue) = TimerEngine::load(&path);
        assert_eq!(overdue.len(), 1);
        assert_eq!(overdue[0].name, "short");
        assert!(overdue[0].overdue);
        engine2.shutdown();
        let _ = std::fs::remove_file(&path);
    }
}
