# Plainpad — Potential Future Features & Architecture Roadmap

> **Architectural Philosophy:**  
> Plainpad is built on a modular, high-performance workspace architecture. The core rule for feature placement is:
> 
> * **Inside `plainpad-core`:** Any capability that directly parses, inspects, evaluates, or formats text in real-time under Plainpad's strict **< 30 ms latency budget**. Core must remain lightweight, cross-platform, non-I/O-blocking, and dependency-lean.
> * **In Separate Workspace Crates (`crates/*`):** Any capability involving background network I/O, heavy third-party dependencies, cryptographic primitives, native OS daemons, external protocols (LSP, CalDAV, WebDAV), or optional standalone binaries (CLI).

---

## Architecture Blueprint: The Multi-Crate Workspace

```
Plainpad/
├── Cargo.toml                     # Top-level workspace definition
├── crates/
│   ├── plainpad-core/             # The brain: grammar, AST, real-time intent analysis (<30ms)
│   ├── plainpad-sync/             # [Future] E2EE sync, CRDTs (Yrs), P2P & cloud replication
│   ├── plainpad-lsp/              # [Future] Language Server Protocol for VS Code / Neovim
│   ├── plainpad-cli/              # [Future] Terminal companion & batch pipeline evaluator
│   └── plainpad-calendar/         # [Future] Native OS calendar integration & CalDAV bridge
├── src-tauri/                     # Native desktop shell (Tauri 2.0, WebKitGTK / WebView2)
└── ui/                            # Fast TypeScript/CSS front-end
```

---

## Group A: Features That Belong INSIDE `plainpad-core`

These features directly inspect, evaluate, or format note text in real-time as the user types, feeding into Plainpad's instant AST analysis pipeline.

### 1. Smart Markdown Tables with Formulas (`=sum`, `=avg`, `=B2*C2`)

#### Concept
Typing a standard Markdown table triggers tabular AST recognition. When formula expressions like `=sum`, `=avg`, `=count`, or column-based operations appear in the final row or column, Plainpad evaluates and formats the result inline.

#### User Experience
```markdown
| Item     | Cost   | Qty | Subtotal |
| Coffee   | 4.50   | 2   | =B1*C1   |  <- auto-evaluates to $9.00
| Lunch    | 12.00  | 1   | 12.00    |
| Total    | =sum   |     |          |  <- auto-evaluates to $21.00
```

#### Why in `plainpad-core`?
* Directly integrates into Plainpad's existing line scanner and math parser.
* Uses the in-memory AST without requiring external network calls or disk operations.
* Preserves the pure plain-text Markdown structure while giving scratchpad spreadsheet powers.

---

### 2. Natural Language Dates & Task Reminders

#### Concept
Detects temporal expressions like `"meet Sarah next Friday at 3pm"` or `"call landlord tomorrow at 10am"`. The phrase is annotated with `kind: "event"`, displays a calendar badge overlay, and generates an in-memory `.ics` standard iCalendar payload or task reminder payload.

#### User Experience
```markdown
dentist appointment next Tuesday at 2pm    [ 📅 Add to Calendar ]
remind me to pay rent in 3 days            [ ⏰ Set Reminder ]
```

#### Why in `plainpad-core`?
* Extends the existing `trigger_evaluator` and tokenizer that already powers timers (`timer 25m`).
* Zero I/O overhead: parsing natural language date offsets runs deterministically in microseconds.
* Produces portable iCalendar RFC 5545 strings in-memory for the UI or OS to consume.

---

### 3. Dimensional Unit Arithmetic & Compound Conversions

#### Concept
Expands the unit conversion engine to handle complex multi-step unit expressions and compound velocity/power/energy math directly in prose.

#### User Experience
```markdown
65 mph for 2 hours and 15 mins in km       = 233.35 km
1200 watts for 4 hours in kwh              = 4.8 kWh
4.8 kWh at $0.18 per kwh                   = $0.86
```

#### Why in `plainpad-core`?
* Extends `crates/plainpad-core/src/units.rs` and Fend integration.
* Completely self-contained mathematical grammar and affine conversion matrices.

---

### 4. Bi-Directional WikiLinks & Backlink Graph Indexer

#### Concept
Parses `[[Note Title]]` or `[[Target Note|Custom Alias]]` syntax in real-time, validating link validity against the SQLite notes database, reporting dangling links, and maintaining an in-memory graph of incoming and outgoing references.

#### User Experience
```markdown
See research notes in [[Q3 Financial Summary]]
Action item discussed with [[Sarah Miller|Sarah]]
```

#### Why in `plainpad-core`?
* Extends the relational entity extraction engine (`crates/plainpad-core/src/relational.rs`) that already extracts checklists and antilinks.
* Real-time link validation and auto-suggestion directly inform overlay badges.

---

### 5. Cross-Note Transclusion & Dynamic Block References

#### Concept
Allows referencing calculations or lists from other notes using transclusion syntax `!include(Note Title:total)` or `!embed(Shopping List)`.

#### User Experience
```markdown
Monthly Expenses:
Rent: 1500
Utilities: 220
Groceries: !include(Groceries Note:total)  <- embeds live evaluated sum
Total: =sum
```

#### Why in `plainpad-core`?
* Requires relational access to the SQLite storage layer already managed by `plainpad-core::db`.
* Evaluated during the AST resolution phase so all sums and variables remain mathematically coherent.

---

## Group B: Features That Belong in SEPARATE Workspace Crates

These features provide advanced infrastructure, external platform bridges, or heavy network/cryptographic systems. Isolating them into independent crates prevents `plainpad-core` from bloating and keeps UI typing latency near 0ms.

---

### 1. `crates/plainpad-sync` — End-to-End Encrypted Cloud & P2P Sync Engine

#### Purpose
Provides zero-knowledge, encrypted multi-device synchronization over local P2P networks (mDNS/WebRTC) or cloud object stores (S3, WebDAV, Cloudflare R2).

#### Technical Stack
* **CRDT Engine:** `yrs` (Yjs Rust port) or `automerge` for conflict-free concurrent editing.
* **Cryptography:** `ring` or `sodiumoxide` for XChaCha20-Poly1305 authenticated encryption and Argon2id key derivation.
* **Networking:** `tokio`, `webrtc`, or lightweight HTTP/WebSocket clients.

#### Why a separate crate?
* Network I/O and async handshakes must never block the synchronous editor keystroke loop.
* Pulling in cryptographic suites and WebRTC protocols increases binary footprint and compile times, which should not affect someone just running core tests.

---

### 2. `crates/plainpad-lsp` — Language Server Protocol Companion

#### Purpose
Turns Plainpad's core intelligence into a standard Language Server Protocol (LSP) daemon. Any developer using **VS Code**, **Neovim**, **Zed**, **Emacs**, or **Helix** can configure `plainpad-lsp` on `.txt` or `.md` files to get Plainpad's real-time math evaluation, currency conversion, smart tables, and timer highlights directly in their code editor.

#### Technical Stack
* `tower-lsp` or `lsp-types` over standard I/O (stdio).
* Consumes `plainpad-core` as a library dependency, passing incoming file buffers to `plainpad_core::intent::analyze()`.

#### Why a separate crate?
* Pure headless protocol adapter. It has zero UI dependencies and serves an entirely different distribution channel (editor plugin stores, package managers).

---

### 3. `crates/plainpad-cli` — Terminal & Scripting Companion

#### Purpose
A command-line companion binary (`plainpad`) that allows UNIX pipeline workflows, terminal evaluations, and batch conversions.

#### Technical Stack
* `clap` for command-line argument parsing and shell autocompletion generation.
* `colored` / `crossterm` for terminal rendering.

#### Example Commands
```bash
# Evaluate an inline math expression or conversion instantly
plainpad eval "200 inr to cad"
# Output: C$2.97

# Evaluate math inside an entire markdown document
cat budget.md | plainpad eval --in-place

# Start a background timer from your terminal
plainpad timer 25m --name "Code Review"

# Export notes to Obsidian or Markdown bundle
plainpad export --format obsidian --out ~/Documents/Vault
```

#### Why a separate crate?
* Compiles into a dedicated standalone executable (`/usr/local/bin/plainpad`).
* Does not require Tauri, WebKitGTK, or GUI dependencies.

---

### 4. `crates/plainpad-calendar` — Native Calendar & Reminders Integration

#### Purpose
A native OS integration crate that communicates with macOS EventKit, Linux Evolution Data Server (EDS), Windows Calendar API, or CalDAV/Google Calendar servers.

#### Technical Stack
* Platform-specific native bridges: `objc2` for Apple EventKit, `zbus` for Linux D-Bus/EDS, native Windows runtime bindings.
* CalDAV client for Nextcloud, Fastmail, and self-hosted calendar backends.

#### Why a separate crate?
* Interfacing with OS calendar services requires platform-specific system headers, permissions management, and asynchronous D-Bus/COM/Obj-C calls.
* Keeping it in a satellite crate keeps `plainpad-core` 100% pure Rust and fully OS-agnostic.

---

### 5. `crates/plainpad-ocr` — Offline Document & Receipt Scanner

#### Purpose
Allows dropping an image of a receipt, whiteboard, or handwritten note onto Plainpad to automatically transcribe text, identify currency amounts, and populate an editable Plainpad note.

#### Technical Stack
* `tract` or `ort` (ONNX Runtime) for lightweight offline OCR neural networks.
* Image decoding and normalization via `image` crate.

#### Why a separate crate?
* Vision and machine learning models are multi-megabyte dependencies that require substantial CPU/GPU compute during processing.
* Belongs as an optional or lazy-loaded crate so the core editor remains lightweight and snappy.

---

## Architectural Decision Matrix

| Dimension | Inside `plainpad-core` | Separate Crate (`crates/*`) |
|---|---|---|
| **Execution Timing** | Synchronous or bounded (<30ms) | Asynchronous or background worker |
| **Dependencies** | Minimal (math grammar, regex, serde, sqlite) | Heavy (network, WebRTC, ONNX, D-Bus, LSP) |
| **I/O Access** | In-memory strings + local SQLite database | Network sockets, external processes, platform APIs |
| **Interface** | Internal Rust API & WebAssembly/IPC exports | CLI commands, LSP JSON-RPC, or sync daemons |
| **Impact on Editor Latency** | Direct (must stay sub-frame) | Zero (completely decoupled from keystrokes) |

---

## Summary Roadmap

1. **Immediate Next Steps in `plainpad-core`:**
   - Implement **Smart Markdown Tables** with `=sum` and `=avg` column aggregators.
   - Implement **Natural Language Dates** with `.ics` export badges.
2. **First Satellite Crate Candidate:**
   - Spin up `crates/plainpad-cli` as a lightweight CLI tool to expose `plainpad-core` algorithms to terminal power users.
3. **Ecosystem & Cloud:**
   - Introduce `crates/plainpad-sync` with Yrs CRDTs for secure multi-device synchronization.
