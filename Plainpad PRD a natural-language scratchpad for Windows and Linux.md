# Plainpad: Product Requirements Document

**Working title:** Plainpad (placeholder; run a trademark check before using) **Status:** Draft v0.1 **Date:** 4 Oct 2026 **Platforms:** Windows 10/11, Linux (X11 and Wayland)

---

## 1. Summary

Plainpad is a minimalist, keyboard-first scratchpad that opens instantly over whatever you are doing. You type plain English and it understands: sums become live math, task phrases become checklists, pasted code becomes highlighted code, and times become timers. There is no Markdown to learn and no toolbar to manage.

**One-line pitch:** The scratchpad where you just write, and the app works out the rest.

## 2. Background and opportunity

Research (Oct 2026) on the closest products:

- **Antinote (macOS, $5):** the reference product. Global hotkey, inline natural-language math, conversions, timers, OCR, AutoPaste, JavaScript extensions, encrypted iCloud sync. macOS only; no Windows or Linux version is planned.
- **LinNote (Linux, MIT):** broad feature set but early (v1.0.0, one maintainer). Math needs an `=` suffix and slash-commands. GNOME global hotkeys need extra setup. Linux only.
- **EasyNotes (Windows, MIT):** hotkey, tray, note stack, checklists, themes. No math, conversions, timers, OCR or export. Windows only.

**Gaps:** no product offers the full Antinote-class feature set on Windows and Linux, none has a reliable hotkey and popup on Wayland and GNOME, and none lets people drive features with plain sentences instead of syntax (slash commands, mode keywords or Markdown).

## 3. Problem statement

People jot down sums, phone numbers, to-dos, snippets and reminders all day. Existing options force a choice: a full notes app (too heavy, pollutes your system), Notepad-style tools (no intelligence), or syntax-driven tools (Markdown, slash-commands) that must be learned and remembered. Windows and Linux users also lack a polished option comparable to Antinote.

## 4. Target users

| Persona | Needs | Why Plainpad |
| --- | --- | --- |
| **Student / engineer (primary)** | Quick calculations, unit and base conversions, code snippets, timers | Engineer-aware math, code detection, runs on their OS |
| **Developer / power user** | Keyboard-only, scriptable, privacy-conscious | CLI, extensions, local-first, no telemetry |
| **Everyday knowledge worker** | Park a number, list or draft without ceremony | Zero syntax, nothing to set up |

Non-target: people who want a permanent knowledge base (Obsidian, Notion).

## 5. Goals and non-goals

### Goals

1. Summon, type and dismiss in under a second, from any app, on Windows and on Linux (including Wayland).
2. Replace Markdown and slash-commands with plain-language understanding that is predictable and reversible.
3. Cover the everyday scratchpad feature set: math, conversions, lists, code, timers, OCR, clipboard capture, export.
4. Stay minimal: one window, no folders, no tags, no formatting toolbar.
5. Local-first and private by default.

### Non-goals (v1)

- Permanent knowledge management, folders, backlinks, tags.
- Rich text, fonts, colours, embedded media.
- Real-time collaboration or accounts.
- Mobile apps (future consideration).
- Always-on background AI reading notes.

## 6. Success metrics

Targets are assumptions to validate after launch.

| Metric | Target |
| --- | --- |
| Hotkey to first keystroke accepted (p95) | < 150 ms |
| Cold start to usable | < 500 ms |
| Idle memory (tray resident) | < 120 MB (stretch: < 80 MB) |
| Installer size | < 30 MB |
| Intent-detection latency per keystroke | < 30 ms |
| Auto-applied transforms undone by the user | < 5% |
| Wayland hotkey success on GNOME, KDE, Sway/Hyprland | Works with documented setup on all three |
| Adoption (open source) | 1,000 GitHub stars within 6 months of launch |
| Retention proxy | 40% of installs active in week 4 |

## 7. Product principles

1. **Suggest, don't surprise.** Anything the app infers is visible, explainable and reversible with one key. Only unambiguous patterns apply automatically.
2. **Deterministic first.** Core understanding runs on local parsers and rules. No network, no model required.
3. **Plain text is the source of truth.** What you type is what is stored. Structure is a view over text, so you can always edit it as text.
4. **Notes are disposable by default**, with a one-key way to keep or export.
5. **Speed is a feature.** If it feels slower than Notepad, it has failed.
6. **Quiet UI.** No chrome that is not needed right now.

## 8. Core concept: the natural-language layer

### 8.1 What it does

As the user types, an **intent engine** classifies each line or block and attaches a lightweight view (a result, a checkbox, a code style). The underlying text never changes unless the user accepts a transformation.

### 8.2 Pipeline

1. **Tokenise** each line and its context (neighbouring lines, note history).
2. **Detect** candidates with fast deterministic parsers: math expressions, quantities with units, currencies, dates and times, durations, list-like phrases, code-like text.
3. **Score confidence.**
   - High confidence: render a live result or style inline (for example, `45 * 12` shows `540`).
   - Medium confidence: show a small suggestion chip the user accepts with Tab or ignores.
   - Low confidence: do nothing.
4. **Explain and undo.** Every inference has a hover or keyboard 'why' and Esc or Ctrl+Z reverts it, and a per-note 'plain mode' disables inference entirely.
5. **Optional local LLM** (off by default, user-supplied model such as Ollama) handles fuzzy cases and the natural-language command palette. The app must be fully functional without it.

### 8.3 Natural-language examples

| User types | Plainpad does |
| --- | --- |
| `rent 12000 + food 4500 + transport 1800` | Shows the total inline, keeps the labels |
| `rent = 12000` then `rent * 12` | Reactive variables; changing `rent` updates dependants |
| `20% of 850`, `850 + 15%` | Percentage math |
| `5 km in miles`, `72 f to c` | Unit conversion |
| `100 usd in inr` | Currency conversion with cached rates (network opt-in) |
| `3pm IST in PST` | Timezone conversion |
| `friday + 10 days` | Date arithmetic |
| `0xFF & 0x0F`, `255 in binary` | Programmer math and base conversion |
| `buy milk, eggs, bread` | Suggests converting to a checklist |
| `todo call dentist` | Creates a checkbox item (explicit trigger) |
| `timer 25 min` or `remind me in 10 minutes` | Starts a timer or reminder |
| Pasting a function | Detects language, applies monospace and highlighting |
| Pasting comma-separated rows | Suggests formatting as a table |
| Ctrl+K then `sort these lines` | Natural-language command palette runs the transform |

An explicit `list`, `math` or `code` word at the top of a note forces that mode, for users who prefer being deliberate.

### 8.4 Replacing Markdown

- **Title:** the first line is the note title. No `#` needed.
- **Lists:** natural phrases, a leading dash or bullet, or `todo`; Tab nests items.
- **Emphasis:** not supported in notes (plain text). Export converts structure to Markdown on request.
- **Code:** detected on paste or typing; triple backticks are accepted but never required.
- **Links:** shortened for display, expand on click.

## 9. Functional requirements

Priority: **P0** = MVP, **P1** = v1.0, **P2** = later.

### 9.1 Capture and window

| ID | Requirement | Pri |
| --- | --- | --- |
| W-1 | Global hotkey (default Alt+A, customisable) shows or hides the window | P0 |
| W-2 | Always-on-top popup window; hides on Esc; remembers position and size | P0 |
| W-3 | Tray icon with Show, Hide, Quit; optional launch at login | P0 |
| W-4 | Wayland support via xdg-desktop-portal GlobalShortcuts, with documented compositor fallbacks (GNOME, KDE, Sway, Hyprland) | P0 |
| W-5 | Display modes: popup, docked panel, tray-only | P1 |
| W-6 | Show over fullscreen apps where the OS allows | P1 |

### 9.2 Notes model

| ID | Requirement | Pri |
| --- | --- | --- |
| N-1 | Stack of notes; keyboard and gesture navigation; new note when moving past the newest | P0 |
| N-2 | Autosave on every change; local SQLite storage | P0 |
| N-3 | Empty notes are discarded automatically | P0 |
| N-4 | Recoverable trash (30 days) | P0 |
| N-5 | Per-note expiry with a pin to keep | P1 |
| N-6 | Search across all notes (Ctrl+F in note, global search overlay) | P1 |
| N-7 | Note templates (to-do, standup, 1:1, meeting notes) plus user-defined templates with date, time and cursor placeholders, inserted by plain phrase | P2 |

### 9.3 Natural-language engine

| ID | Requirement | Pri |
| --- | --- | --- |
| L-1 | Per-line intent detection with confidence scoring and inline chips | P0 |
| L-2 | Explain ('why'), accept, dismiss and undo for every inference | P0 |
| L-3 | Per-note and global 'plain mode' toggle | P0 |
| L-4 | Natural-language command palette (Ctrl+K) with a deterministic grammar for core commands | P1 |
| L-5 | Optional local LLM fallback (Ollama or compatible) for the palette and fuzzy intents | P2 |
| L-6 | Rule and locale packs (date formats, decimal separators, languages) | P1 |

### 9.4 Math and conversions

| ID | Requirement | Pri |
| --- | --- | --- |
| M-1 | Inline arithmetic with descriptive text on the same line | P0 |
| M-2 | Named variables with reactive recalculation | P0 |
| M-3 | Percentages, powers, roots, common functions | P0 |
| M-4 | Unit conversion (length, mass, temperature, volume, area, speed, time, data) | P0 |
| M-5 | Currency and crypto conversion; cached rates; network call opt-in | P1 |
| M-6 | Date and time arithmetic, timezone conversion | P1 |
| M-7 | Engineering math: SI prefixes, hex, binary, octal, bitwise, dB, Ohm's law style quantities | P1 |
| M-8 | Sum, average, min and max over a block; count of items, lines, words and characters, with a comment marker (//) to exclude a line from the count | P0 |
| M-9 | Evaluate with a proven Rust math engine (evaluate `fend` or `Numbat` licences and fit before adopting) | P0 |
| M-10 | Finance calculators in plain language: loan and mortgage payments with amortisation schedule, present and future value, payment per period | P1 |
| M-11 | Advanced finance: NPV, IRR, and FIRE (financial independence) planning | P2 |

### 9.5 Lists and tasks

| ID | Requirement | Pri |
| --- | --- | --- |
| T-1 | Checklists with clickable and keyboard toggling | P0 |
| T-2 | Nested items with Tab and Shift+Tab | P1 |
| T-3 | Checked items can sink to the bottom or auto-remove (setting) | P1 |
| T-4 | Bullets and numbered lists from natural phrasing | P0 |
| T-5 | Date chips on tasks ('tomorrow 5pm') with optional OS notification | P2 |

### 9.6 Code and text tools

| ID | Requirement | Pri |
| --- | --- | --- |
| C-1 | Code detection on paste and typing; monospace plus syntax highlighting for 20+ languages | P0 |
| C-2 | Manual language override | P1 |
| C-3 | Inline text transforms: case, sort, dedupe, trim, join/split lines; keep or remove lines containing given text; keep or remove text between delimiters | P1 |
| C-4 | Data tools: JSON format/minify, CSV to table, base64, URL encode/decode, hashes, UUID, regex tester | P1 |
| C-5 | Diff of two pasted blocks | P2 |
| C-6 | Insert helpers: dates with day or business-day offsets, random numbers and strings, dice rolls | P2 |

### 9.7 Timers and reminders

| ID | Requirement | Pri |
| --- | --- | --- |
| R-1 | Stopwatch, countdown and pomodoro from plain phrases; timers can be named and shown fullscreen | P0 |
| R-2 | OS notification and sound on completion; timers survive window close | P0 |
| R-3 | 'Remind me' with a time expression | P1 |

### 9.8 Capture helpers

| ID | Requirement | Pri |
| --- | --- | --- |
| H-1 | AutoPaste: collect everything copied into the current note as plain text | P1 |
| H-2 | Region screenshot OCR hotkey (Windows OCR API; Tesseract on Linux) | P1 |
| H-3 | Paste strips formatting by default | P0 |
| H-4 | Link shortening with readable endings | P1 |
| H-5 | Clipboard history and paste queue | P2 |

### 9.9 Export, automation and extensibility

| ID | Requirement | Pri |
| --- | --- | --- |
| E-1 | Export to txt, Markdown, PDF; copy as Markdown | P0 |
| E-2 | Export or sync to a folder (for example an Obsidian vault) | P1 |
| E-3 | CLI: `plainpad add`, pipe stdin, `plainpad get` | P1 |
| E-4 | URL or protocol handler and local API | P2 |
| E-5 | Extensions: sandboxed (WASM or JS) commands with declared permissions and a data-access declaration | P2 |

### 9.10 Sync, privacy and security

| ID | Requirement | Pri |
| --- | --- | --- |
| S-1 | No account, no telemetry, no crash reporting by default | P0 |
| S-2 | Settings page lists every possible network call, each with its own switch | P0 |
| S-3 | Per-note lock with a master password and recovery key | P1 |
| S-4 | Optional end-to-end encrypted sync via user-supplied backend (synced folder, WebDAV, S3) | P2 |
| S-5 | Local database is open SQLite; documented schema | P0 |

### 9.11 Appearance

| ID | Requirement | Pri |
| --- | --- | --- |
| A-1 | Light and dark themes following the OS | P0 |
| A-2 | Custom theme import (JSON) | P1 |
| A-3 | Adjustable font and size; monospace for code only | P1 |
| A-4 | Translucent window where the OS supports it | P2 |

## 10. Non-functional requirements

- **Performance:** meet the targets in section 6; typing latency unaffected by intent detection (run off the input path).
- **Reliability:** no data loss on crash or power loss (write-ahead logging, autosave within 500 ms).
- **Accessibility:** full keyboard operation, screen reader labels, respect OS contrast and reduced-motion settings.
- **Internationalisation:** UI strings externalised; locale-aware number, date and currency parsing.
- **Compatibility:** Windows 10 and 11 (x64, ARM64 as stretch); major Linux distributions on X11 and Wayland.
- **Packaging:** Windows MSI/NSIS and winget; Linux AppImage, Flatpak, .deb and AUR; auto-update optional and off by default for privacy.
- **Licensing:** open source (MIT or Apache-2.0) for the core; audit all dependencies.

## 11. UX overview

- **Single window.** A text area, a thin title line (first line of the note) and a note counter. Nothing else is visible by default.
- **Inline results.** Math results and chips appear right-aligned on the same line in muted colour.
- **Chips.** Small and dismissible: 'Make checklist (Tab)', 'Table (Tab)', 'Start timer (Enter)'.
- **Command palette.** Ctrl+K, one input line, plain-English commands with live preview.
- **Settings.** One page, grouped: Hotkey, Appearance, Intelligence, Privacy, Export.
- **First run.** A three-line example note that demonstrates math, a list and a timer; no wizard beyond hotkey choice.

## 12. Technical approach (recommendation)

| Area | Recommendation | Reason |
| --- | --- | --- |
| Shell | Tauri 2 | Small binaries, one codebase for both OSes, mature tray and shortcut plugins |
| Core | Rust library: storage, intent engine, math, timers | Speed, safety, reuse in CLI |
| UI | Minimal web UI (lightweight editor component) | Fast iteration; keep dependencies small |
| Storage | SQLite with WAL | Reliable, inspectable, easy sync later |
| Math | Adopt or wrap an existing Rust unit-aware engine | Avoids writing a parser from scratch |
| OCR | Windows OCR API; Tesseract on Linux | Local, no uploads |
| Wayland | xdg-desktop-portal GlobalShortcuts plus compositor-specific fallbacks | Only standard route |

**Prototype first:** global hotkey, always-on-top popup and clipboard watching on Wayland in week one; if these fail the Linux scope must change.

## 13. Release plan

Estimates assume one developer part-time and should be revisited after the week-one prototype.

| Milestone | Scope | Estimate |
| --- | --- | --- |
| **M0 Prototype** | Hotkey, popup, tray on Windows plus GNOME, KDE and Sway | 1-2 weeks |
| **M1 MVP (P0)** | Note stack, autosave, trash, math, lists, code detection, timers, paste cleanup, export, plain mode | 6-8 weeks |
| **M2 v1.0 (P1)** | Conversions, dates, engineering math, command palette, OCR, AutoPaste, search, lock, CLI, themes | 6-8 weeks |
| **M3 Beta** | Packaging for all targets, docs site, accessibility pass, performance tuning | 3-4 weeks |
| **M4 v1.x (P2)** | Extensions, E2EE sync, optional local LLM, notifications for task dates | Ongoing |

## 14. Risks and mitigations

| Risk | Impact | Mitigation |
| --- | --- | --- |
| Natural-language inference feels unpredictable | Users lose trust | Suggest-don't-surprise, confidence thresholds, explain and undo, plain mode |
| Wayland hotkeys or fullscreen overlay unreliable | Linux unusable on some desktops | Week-one prototype; portal API plus documented per-compositor fallbacks; scope Linux support by tested desktops |
| Idle memory too high with a webview | Misses minimalism promise | Measure early; consider lighter UI layer if the target is missed |
| Competitor catch-up (LinNote, Antinote platform expansion) | Lower differentiation | Ship reliability and the natural-language layer first; stay open source |
| Currency or rate API dependency | Privacy conflict, outages | Opt-in network, cached rates, swappable providers |
| Scope creep (the 'and more' list) | Never ships | Hold to P0 for MVP; every new feature must pass 'does it keep the app minimal?' |
| Trademark or clone concerns | Legal or reputational | Original name, icons and themes; do not reuse any competitor assets or branding |

## 15. Open questions

1. Final name and branding.
2. Open source from day one, or closed beta first?
3. Free forever, or a paid tier later (sync, extensions marketplace)?
4. Is a macOS version in scope given Antinote exists there?
5. Which languages and locales matter first for parsing?
6. How much of the natural-language command palette needs a model versus a fixed grammar?
7. Is a lighter UI layer than a webview worth the added effort if memory targets are missed?

## 16. Appendix: competitive reference

| Capability | Antinote | LinNote | EasyNotes | Plainpad target |
| --- | --- | --- | --- | --- |
| Windows | No | No | Yes | Yes |
| Linux | No | Yes | No | Yes |
| Natural-language math | Yes | Partial | No | Yes |
| Syntax-free interaction (no modes, slash-commands or Markdown) | Partial | No | No | Yes |
| Extensions | Yes | No | No | P2 |
| Sync | iCloud | No | No | P2 (E2EE, user-supplied backend) |
| Reliable Wayland hotkey | n/a | Partial (GNOME) | n/a | P0 |
