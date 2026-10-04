<div align="center">

# ✏️ Plainpad

**The scratchpad where you just write, and the app works out the rest.**

A minimalist, keyboard-first scratchpad that opens instantly over whatever you're doing.
Type plain English and it understands: sums become live math, task phrases become checklists,
pasted code becomes highlighted, and times become timers. No Markdown to learn, no toolbar to manage.

[![License: MIT](https://img.shields.io/badge/License-MIT-coral.svg)](LICENSE)
[![Built with Tauri](https://img.shields.io/badge/Built_with-Tauri_2-blue.svg)](https://v2.tauri.app)
[![Rust](https://img.shields.io/badge/Core-Rust-orange.svg)](https://www.rust-lang.org)

</div>

---

## ✨ Features

| Feature | Description |
|---------|-------------|
| ⚡ **Instant popup** | Global hotkey (default `Alt+A`) summons and hides in under 150 ms |
| 🧮 **Natural-language math** | `rent 12000 + food 4500` → shows total inline, keeps your labels |
| 📐 **Reactive variables** | `rent = 12000` then `rent * 12` → updates automatically |
| 🔄 **Unit conversions** | `5 km in miles`, `72°F to °C`, `100 usd in inr` |
| 🔢 **Programmer math** | `0xFF & 0x0F`, `255 in binary`, bitwise ops |
| ✅ **Smart checklists** | Type `todo call dentist` or `buy milk, eggs, bread` |
| ⏱️ **Timers** | `timer 25 min`, `remind me in 10 minutes` — survives app restart |
| 💻 **Code detection** | Paste code → auto-detected language, monospace rendering |
| 📊 **Block stats** | Sum, average, min, max over lines (use `//` to exclude) |
| 📋 **Paste cleanup** | Strips formatting by default — plain text in, plain text stored |
| 📤 **Export** | Copy as Markdown, export as `.txt` or `.md` |
| 🔄 **LAN sync** | Zero-config peer-to-peer sync over your local network |
| 📁 **Folder sync** | Point at any synced folder (Dropbox, Syncthing, Obsidian vault) |
| 🔒 **Privacy-first** | No account, no telemetry, no crash reporting. Local-only by default |

## 🖼️ Design

Inspired by [Antinote](https://antinote.app) — a dark frameless floating card with:
- Monospace typography (JetBrains Mono / Cascadia / system fallback)
- Coral/salmon accent for results (`= 540`)
- Muted teal for variables
- Hover-revealed chrome bars (top: navigation, bottom: actions)
- Light and dark themes following your OS

## 🚀 Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) (stable, 1.77+)
- [Node.js](https://nodejs.org/) (18+)
- **Windows:** WebView2 (pre-installed on Windows 10/11)
- **Linux:** `webkit2gtk-4.1`, `libayatana-appindicator3-1` (see [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/))

### Build & Run

```bash
# Install Node dependencies
npm install

# Build the UI
npm run build:ui

# Run in development mode
cargo run -p plainpad

# Run tests
cargo test --workspace

# Clippy lint
cargo clippy --workspace -- -D warnings
```

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Alt+A` | Show / hide Plainpad |
| `Esc` | Hide the window |
| `Ctrl+N` | New note |
| `Ctrl+←` / `Ctrl+→` | Previous / next note |
| `Ctrl+Shift+D` | Delete note |
| `Ctrl+Shift+E` | Export note |
| `Ctrl+P` | Toggle plain mode (disable intent detection) |
| `Tab` | Accept a suggestion chip |
| `Ctrl+Z` | Undo (including reverting inferences) |

## 🏗️ Architecture

```
plainpad/
├── crates/plainpad-core/     Pure Rust — shared by desktop, CLI, mobile
│   ├── lib.rs                Note, NoteMeta, TrashEntry, ID generation
│   ├── store.rs              FileStore: atomic writes, CRUD, trash, settings
│   ├── mathwrap.rs           fend-backed eval, variables, %, units, bitwise
│   ├── intent.rs             Per-line intent engine with confidence scoring
│   ├── timers.rs             Persistent countdown timers with watcher thread
│   ├── sync.rs               LAN peer sync (UDP+TCP) + folder sync
│   └── export.rs             txt / Markdown export
├── src-tauri/                Desktop shell: window, tray, hotkey, IPC
├── ui/                       No-framework TypeScript: textarea + overlay editor
└── scripts/                  Build helpers (icon gen, UI copy)
```

### Data on Disk (No Database)

Notes are plain `.txt` files — inspectable, portable, and Obsidian-vault compatible:

```
<appdata>/plainpad/
├── notes/<id>.txt              One file per note (line 1 = title)
├── trash/<id>.txt              Recoverable trash (30-day retention)
└── state/
    ├── index.json              Note order and per-note flags
    ├── settings.json           Hotkey, theme, sync config
    ├── timers.json             Persisted timers
    └── geometry.json           Window position and size
```

### Sync Protocol

- **LAN:** UDP broadcast discovery → TCP JSON-lines manifest exchange → file payloads
- **Folder:** Two-way reconcile against any synced directory on a 2s poll
- **Merge rule:** Newer `mtime` wins; same time + different content → conflict copy (never silently dropped)

## 🛣️ Roadmap

| Milestone | Status |
|-----------|--------|
| M0: Prototype (hotkey, popup, tray) | 🔨 In progress |
| M1: MVP (math, lists, timers, export) | 🔨 Core complete, shell + UI in progress |
| M2: v1.0 (conversions, OCR, CLI, themes) | ⏳ Planned |
| M3: Beta (packaging, docs, a11y) | ⏳ Planned |
| M4: v1.x (extensions, E2EE, local LLM) | ⏳ Future |

## 🤝 Contributing

Contributions are welcome! Please read [CONTRIBUTING.md](CONTRIBUTING.md) before submitting a PR.

1. Fork the repository
2. Create your feature branch (`git checkout -b feat/amazing-feature`)
3. Ensure tests pass (`cargo test --workspace`)
4. Ensure no lint warnings (`cargo clippy --workspace -- -D warnings`)
5. Submit a Pull Request

## 📄 License

MIT — see [LICENSE](LICENSE) for details.

## 🙏 Acknowledgements

- [fend](https://github.com/printfn/fend) — the unit-aware math engine powering Plainpad's calculations (MIT)
- [Tauri](https://tauri.app) — the framework for building tiny, fast desktop apps
- [Antinote](https://antinote.app) — the macOS app that inspired Plainpad's UX vision
