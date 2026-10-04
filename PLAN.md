# Plainpad — Implementation Plan (v2.1)

Status: **build in progress** (see §6 Progress). v2 revised v1 after two owner decisions
(**mobile is a target platform**, **no database anywhere**); v2.1 records the owner's
**Flutter-for-mobile** choice and the **UI design language** from the reference shots.
Source: `Plainpad PRD a natural-language scratchpad for Windows and Linux.md` (Draft v0.1).

---

## 1. Decisions changed in v2 / v2.1

| Area | v1 plan | Final decision | Why |
| --- | --- | --- | --- |
| Storage | SQLite (WAL) per PRD N-2 | **Plain files**: one `.txt` per note + JSON sidecars for settings/timers/geometry/index | PRD principle 3 taken literally; folder is inspectable, portable, and *is* an Obsidian-compatible vault; no DB to install, migrate or corrupt. PRD S-5 (documented open format) is now trivially satisfied — the format *is* files. |
| Sync | E2EE sync via user backend (P2) | **Built-in, DB-less, from day one**: LAN peer sync (UDP discovery + TCP, stateless) + any-synced-folder mode; LWW with conflict copies; no accounts, no server-side storage | Owner requirement. A dumb stateless relay (optional, later) can bridge internet sync; payloads are E2E-encrypted (S-4 rule preserved). |
| Mobile | non-goal (PRD); later: Tauri mobile | **Flutter app later, same Rust core via FFI** (flutter_rust_bridge); same sync protocol — the protocol lives in `plainpad-core`, so there is no Dart reimplementation to drift | Owner decision. Flutter owns the excellent mobile UX; Tauri owns desktop where hotkey/tray/popup matter. Kept honest by one rule: **`plainpad-core` stays pure Rust + serde, zero shell dependencies** — its serializable API surface doubles as the FFI contract. |
| Shell | Tauri 2 | **Tauri 2 (stable, not v3 alpha)** — desktop only | Main-codebase path for Win/Linux; desktop features (tray, hotkey) are desktop-only plugins anyway. |
| UI | generic minimal editor | **Design language from reference shots** (§5) | Owner-provided visual direction. |

## 2. What this session builds (M0 + P0 slice of M1)

| In scope now (P0) | Deferred (P1/P2) |
| --- | --- |
| W-1/2/3 hotkey, always-on-top popup, tray, Esc hide, geometry memory | W-4 Wayland portal (behind `hotkey` seam + docs), W-5/6 docked/fullscreen |
| N-1..4 note stack **as files**, autosave, empty discard, 30-day trash | N-5..7 expiry, search, templates |
| L-1..3 per-line intents + confidence, explain/accept/dismiss/undo, plain mode | L-4/5 palette, L-6 locale packs |
| M-1..4, M-8, M-9 math via **fend-core (MIT)**, variables, %, units, block stats | M-5/6 currency/dates; M-7/10/11 |
| T-1/4 checklists + natural bullets | T-2/3/5 nesting, sink, date chips |
| C-1 code detection (language tag + monospace when code-dominant; per-token highlight later) | C-2..6 |
| R-1/2 timers, notification, survive window hide **and app restart** | R-3 reminders |
| H-3 paste strips formatting | H-1/2/4/5 |
| E-1 export txt/Markdown + copy-as-Markdown (PDF via `window.print()`) | E-2..5 |
| **Sync v1:** folder mode (Syncthing/Dropbox/Obsidian vault) + **LAN real-time peer sync**, LWW + conflict copies, all offline-first | Stateless E2E relay for internet sync, CRDT upgrade |
| S-1/2/5 no telemetry, no DB, documented file format | S-3/4 locks, relay |
| A-1 light/dark following OS + design-language styling | A-2..4 |
| Mobile-ready core (no shell deps) | Flutter shell (next milestone) |

Measurable checks this session: intent latency < 30 ms (bench test), autosave < 500 ms
(atomic write per save call), all tests/clippy/typecheck green, two-instance LAN sync
integration test passes on loopback.

## 3. Architecture

```
plainpad/                       cargo workspace
├── crates/plainpad-core/       PURE Rust (serde only) — shared by desktop, CLI, Flutter mobile
│   ├── store.rs                FileStore: notes/*.txt, trash/, state/*.json, index rebuild
│   ├── intent.rs               per-line intent engine (rules → confidence → why)
│   ├── mathwrap.rs             fend-backed eval + rewrites (labels, %, units, bases) + fallbacks
│   ├── timers.rs               timer engine, persisted, fire events, survives restart
│   ├── sync.rs                 manifest reconcile + TCP/JSON change push + UDP discovery + folder mode
│   └── export.rs               txt / markdown
├── src-tauri/                  desktop shell ONLY: window, tray, hotkey, notifications, IPC
├── ui/                         no-framework TS: textarea + mirror + overlay editor
├── scripts/                    gen-icons.mjs (offline icon/PNG/ICO writer), copy-ui.mjs
└── PLAN.md / README.md
```

**Data on disk (the whole "database"):**

```
<appdata>/plainpad/
├── notes/<id>.txt              one file per note; line 1 = title; [ ]/[x] = checklist state
├── trash/<id>.txt + trash/meta.json   {id, deleted_at, name} → 30-day purge on launch
├── state/index.json            order + {pinned, plain} per id — rebuildable from notes/
├── state/settings.json         hotkey, theme, plain_mode_global, sync config, network switches
├── state/timers.json           persisted timers (survive restart; expired-while-away fire on load)
└── state/geometry.json         last window x/y/w/h
```

**Sync protocol (stateless, DB-less):**
- **Discovery:** UDP broadcast every 3 s on LAN: `PLAINPAD1|<tcp_port>|<device_id>`.
- **Session:** TCP JSON-lines: `HELLO{device, manifest{name→(hash64, mtime)}}` → each side
  requests what it lacks/outdated → then live `CHANGE{name, b64, mtime}` pushes.
- **Merge rule:** newer mtime wins; equal mtime + different hash → incoming saved as
  `<name>.conflict.<device>.<epoch>.txt` (never silently dropped). Hash = FNV-1a 64 (stable).
- **Folder mode:** same manifest/LWW logic applied two-way against any synced folder,
  polled every 2 s — gives internet sync via whatever the user already uses (Syncthing,
  Dropbox, iCloud, an Obsidian vault), still zero Plainpad infrastructure.

## 4. Editor model

`<textarea>` is the single editable surface (text = truth); a hidden mirror measures
wrapped line boxes; an absolutely-positioned overlay paints right-aligned math results,
suggestion chips (Tab accepts), checkbox squares (click = text rewrite `[ ] `⇄`[x] `),
stat lines, title band, code tint. When ≥60 % of lines are code the textarea switches to
monospace. Limitation noted: per-token syntax colouring needs the M2 editor upgrade.

## 5. UI design language (from owner's reference shots)

Frameless rounded popup floating over the desktop; everything else is hover-revealed:

- **Window:** dark near-black card (`#17191d`-ish), 14–18 px corner radius, generous
  padding, subtle border; light mode mirrors it (near-white card).
- **Typography:** monospace-first (JetBrains Mono / Cascadia / ui-monospace fallback);
  ~15 px base, roomy line-height (~1.6).
- **Mode word:** a bare first line like `math` or `code: bash` renders as a small muted
  colored label (sage green for `math`, coral for `code`) — this is the PRD's explicit
  mode-forcing line, styled.
- **Results:** coral/salmon (`#ff6b5e`-ish) — inline `= result` in accent color; variables
  in muted teal; labels stay normal text. Numbers with thousands separators.
- **Chrome:** two slim hover bars — top bar (note nav: index `2/7`, prev/next, pin,
  plain-mode toggle) revealed when the pointer is in the upper strip; bottom bar (new note,
  delete, export, settings, sync status) revealed at the lower strip. Auto-fade when the
  pointer leaves. Keyboard-only users: bars also appear while Ctrl is held / on focus.
- **Scroll:** thin minimal scrollbar in accent-muted tone.

## 6. Progress

- [x] Research: fend-core MIT chosen (numbat rejected: default features fetch network
  rates — violates S-1/S-2); Tauri 2 stable; plugin set pinned; fend API confirmed
  (`fend_core::evaluate` + `Context::disable_rng`).
- [x] Plan v2.1 (this file).
- [x] Scaffold: workspace Cargo.toml, core + tauri crates, tauri.conf.json (frameless,
  runtime-created window, `ui/dist`), capability, tsconfig, package.json, copy-ui script.
- [x] Offline icon generator (coral rounded square + white P) → icon.ico/png verified.
- [ ] `plainpad-core`: lib.rs done (Note/NoteMeta/TrashEntry, id gen, titles); **next:
  store.rs → mathwrap.rs → intent.rs → timers.rs → sync.rs → export.rs**, each unit-tested
  (sync gets a two-instance loopback integration test).
- [ ] Tauri shell: window/tray/hotkey/geometry/notifications + IPC commands.
- [ ] UI: editor overlay, chips, checklists, plain mode, design-language styling,
  hover chrome bars, settings, timers.
- [ ] Verify: `cargo test`, `cargo clippy -D warnings`, `tsc --noEmit`,
  `cargo build`, launch smoke test.

## 7. Build order (unchanged) & Risks

1. ✅ Research → 2. ✅ Plan → 3. ✅ Scaffold → 4. core crate → 5. Tauri shell → 6. UI → 7. Verify.

- **Wayland hotkey** — untestable on this Windows box; isolated in the shell's hotkey
  seam with the portal path documented in README for M0-Linux week.
- **Sync merge conflicts** — LWW + conflict copies is honest but lossy for *simultaneous*
  edits of the same note; acceptable for a scratchpad (append-heavy, rarely concurrent).
  CRDT (Automerge) is the P2 upgrade path; sync lives in one swappable module.
- **Idle memory** — WebView2 is shared-runtime; measure after slice, fall back to lighter
  UI layer only if measured above PRD target.
- **Intent unpredictability** — confidence bands + `why` + one-key revert + plain mode are
  built in this slice, not deferred.
