# Contributing to Plainpad

Thank you for your interest in contributing! Plainpad is a young project and we welcome all kinds of help — from bug reports to feature implementations.

## Development Setup

### Prerequisites

- **Rust** stable (1.77+) via [rustup](https://rustup.rs/)
- **Node.js** 18+ (for the UI build)
- **Platform-specific:**
  - **Windows 10/11:** WebView2 runtime (pre-installed)
  - **Linux:** `webkit2gtk-4.1`, `libayatana-appindicator3-1`, `librsvg2-dev`

### First-Time Setup

```bash
git clone https://github.com/your-username/plainpad.git
cd plainpad
npm install
npm run build:ui
cargo build --workspace
cargo test --workspace
```

## Code Structure

| Directory | What lives here |
|-----------|----------------|
| `crates/plainpad-core/` | Pure Rust core — no shell or platform dependencies |
| `src-tauri/` | Tauri 2 desktop shell (window, tray, hotkey, IPC) |
| `ui/` | No-framework TypeScript frontend |
| `scripts/` | Build-time helpers |

### The One Rule

> **`plainpad-core` stays pure Rust + serde, zero shell dependencies.**

This crate is shared by the desktop app, the future CLI, and the future Flutter mobile shell (via FFI). If your change adds a dependency on Tauri, a windowing library, or anything platform-specific, it belongs in `src-tauri/`, not in `plainpad-core`.

## Pull Request Checklist

- [ ] `cargo test --workspace` passes
- [ ] `cargo clippy --workspace -- -D warnings` has no warnings
- [ ] `npm run typecheck` passes (if you touched `ui/`)
- [ ] New features include tests
- [ ] Commit messages are clear and descriptive

## Coding Guidelines

- **Rust:** follow `cargo clippy` and `rustfmt` defaults
- **TypeScript:** strict mode, no `any` types
- **Comments:** preserve all existing comments and docstrings unrelated to your change
- **Privacy:** no network calls without an explicit opt-in switch (PRD S-1/S-2)

## Reporting Bugs

Please open an issue with:
1. Your OS and version
2. Steps to reproduce
3. Expected vs actual behaviour
4. Any relevant log output

## Feature Requests

Check the [PRD](Plainpad%20PRD%20a%20natural-language%20scratchpad%20for%20Windows%20and%20Linux.md) first — your idea might already be planned! If not, open an issue describing the use case and how it fits Plainpad's minimalist philosophy.

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
