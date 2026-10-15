---
milestone: Antinote 1:1 Parity Reconstruction
updated: 2026-10-13T19:20:00+05:30
---

# Requirements

## Overview

Comprehensive requirements derived directly from `.gsd/SPEC.md` and reverse engineering evidence in `.gsd/findings.md` to ensure 100% functional, visual, and behavioral parity with Antinote v2.1.0 (`com.chabomakers.Antinote`).

---

## Functional Requirements

| ID | Requirement | Source | Phase | Status |
|---|---|---|---|---|
| **REQ-01** | **Core Data Model 9 Schema**: SQLite relational schema matching the 5 Core Data entities (`notes`, `checklist_items`, `antilinks`, `attachments`, `sync_state`) and all 17 `Note` fields. | SPEC §2 | 1 | Complete |
| **REQ-02** | **Note State & Lifecycle**: Full CRUD with soft delete into "The Void" (`deletedAt`), locking (`isLocked`), archiving (`isArchived`), and biometric/password gating flag (`isPrivate`). | SPEC §2, §9 | 1 | Complete |
| **REQ-03** | **Checklist & Antilink Entities**: Relational support with cascade deletion for inline checklist spans and shortened URL metadata. | SPEC §2 | 1 | Complete |
| **REQ-04** | **Pre-Restore Safety Snapshots**: Pre-restore snapshotting and sync isolation to prevent data loss during restore or recovery operations. | SPEC §9, findings §16 | 1 | Complete |
| **REQ-05** | **9 Slotted Notes Architecture**: Permanent 9-slot carousel named after Greek deities (Hermes, Apollo/Aphrodite, Ares, Cronus, Gaia, Hades, Ouranos, Poseidon, Zeus). | SPEC §3 | 2 | Complete |
| **REQ-06** | **Slotted Notes Bottom Bar UI**: Persistent bottom indicator bar with 9 dot/pill icons, active slot indication, and tooltips. | SPEC §3 | 2 | Complete |
| **REQ-07** | **Slot Hotkeys & Invariance Rules**: `Ctrl+1..9` direct navigation, `Ctrl+Tab` cycling, and protection against batch-deletion or slot displacement. | SPEC §3 | 2 | Complete |
| **REQ-08** | **Math Grammar & Parsing**: Support for decimals, mixed fractions (`1 1/2`), scientific notation (`1e6`), exponents (`2^8`, `2**8`), implicit multiplication (`4(12+3)`), and algebraic precedence. | SPEC §4.1 | 3 | Complete |
| **REQ-09** | **Trig & Angle Modes**: Configurable `degrees` vs `radians` modes and trigonometric/hyperbolic functions (`sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `sinh`, `cosh`, `tanh`, `sqrt`, `cuberoot`). | SPEC §4.1 | 3 | Complete |
| **REQ-10** | **5 Physical Conversion Domains**: Comprehensive bidirectional conversions for Distance, Area (including `cuerda`, `dunam`, `bigha`), Mass (including `stones`, long ton), Volume, and Temperature. | SPEC §4.2 | 3 | Complete |
| **REQ-11** | **Currency Conversion & Formatting**: 160+ fiat currencies and top cryptocurrencies, background rates cache, thousands separators, and configurable decimal precision. | SPEC §4.2 | 3 | Complete |
| **REQ-12** | **Natural-Language Sentence Triggers**: Evaluation of `sum`, `avg`, `math`, `count`, `list`, `check`, `code`, `paste`, and `timer` keywords at paragraph and list scope. | SPEC §5 | 4 | Complete |
| **REQ-13** | **Trigger Configuration**: Master toggle `disableAllKeywords`, `allowArgumentsAfterKeyword`, and custom trigger alias mappings. | SPEC §5 | 4 | Complete |
| **REQ-14** | **AutoPaste & Clipboard Ingestion**: Clipboard monitoring mode appending copied text continuously until dismissed with `Esc`. | SPEC §5, findings §6 | 4 | Complete |
| **REQ-15** | **Timer & Pomodoro Parsing**: Parse `timer` (stopwatch), `timer 5` (countdown), `timer 3:30`, `timer pomo` (25m/5m), `timer 57 11`, and controls (`timer p`, `timer r`, `timer s`). | SPEC §6.1 | 5 | Complete |
| **REQ-16** | **Timer UI & Circular Visuals**: Animated circular progress pill (`circleDiameter`, `circleSpacing`), remaining time display, and work/break phase indicators. | SPEC §6.2 | 5 | Complete |
| **REQ-17** | **Audio Alerts & Modal Overlays**: Shipped audio chimes (`backtowork.m4a`, `countdowndone.m4a`, `workdone.m4a`) and optional full-screen Pomodoro alert overlay. | SPEC §6.2 | 5 | Complete |
| **REQ-18** | **Interactive Inline Checkboxes**: Render `- [ ]` and `- [x]` as clickable checkboxes with bidirectional plaintext synchronization. | SPEC §7 | 6 | Pending |
| **REQ-19** | **List Continuation & Formatting**: Auto-completion for bullet lists (`- `, `* `) and numbered lists (`1. `) on Enter, terminating on blank Enter. | SPEC §7 | 6 | Pending |
| **REQ-20** | **Smart Antilinks**: URL auto-detection with compact domain pill display while maintaining underlying destination URL and opt-out skip list. | SPEC §7 | 6 | Pending |
| **REQ-21** | **Rounded Code Blocks**: Fenced code block rendering with rounded corner backgrounds, syntax highlighting, and "Copy Code" action button. | SPEC §7 | 6 | Pending |
| **REQ-22** | **Modal Vim Mode**: Normal, Insert, Visual, and Visual Line modes with standard motions (`h,j,k,l,w,b,e,0,$,gg,G`) and actions (`d,y,p,u,x,c,r,i,a,o,O`). | SPEC §7 | 6 | Pending |
| **REQ-23** | **JavaScript Extension Runtime**: Sandboxed JS engine executing `antinote-extensions-base-v0.0.1.js` classes (`Extension`, `Command`, `Preference`, `ReturnObject`). | SPEC §8.1 | 7 | Pending |
| **REQ-24** | **Secure Native API Bridge**: `callAPI` function injecting credentials from secure storage and scoping text access (`none`, `line`, `full`). | SPEC §8.1 | 7 | Pending |
| **REQ-25** | **AI Model Catalog & Command Syntax**: Invocation syntax `::commandName(...)` and catalog support for OpenAI, Anthropic, and local Ollama (`localhost:11434`). | SPEC §8.2 | 7 | Pending |
| **REQ-26** | **Floating Glass Window & Hotkey**: Translucent backdrop blur, global summon shortcut (`Ctrl+Space`), system tray icon/menu, and always-on-top pin persistence. | SPEC §9, §11 | 8 | Pending |
| **REQ-27** | **The Void Shaders**: WebGL/Canvas fragment shaders replicating `voidRipple`, `voidDistortionSlow`, and `voidDistortionBurst` with permanent purge confirmation. | SPEC §9 | 8 | Pending |
| **REQ-28** | **Canvas Textures & Typography**: Background overlays (`dots`, `smallGrid`, `bigGrid`), sub-pixel vertical caret nudging, and RTL text layout. | SPEC §7, §9, §11 | 8 | Pending |
| **REQ-29** | **Find & Replace Engine**: Search modal supporting `Contains`, `Starts With`, `Ends With`, `Exact Match`, case sensitivity, and highlighted match counts. | SPEC §9 | 8 | Pending |
| **REQ-30** | **Third-Party Export Suite**: Direct export to Markdown, Plaintext, Obsidian (vault paths), Bear, Apple Notes, Notion, Custom URL templates, and bulk ZIP. | SPEC §10 | 8 | Pending |

---

## Non-Functional Requirements

| ID | Requirement | Category | Phase | Status |
|---|---|---|---|---|
| **NFR-01** | Sub-15ms trigger and math evaluation latency on standard scratchpad notes. | Performance | 3, 4 | Pending |
| **NFR-02** | Zero UI freeze during background currency exchange rate synchronization. | Responsiveness | 3 | Pending |
| **NFR-03** | Local-first offline capability: all core features function without internet connectivity. | Reliability | All | Pending |
| **NFR-04** | Cross-platform compatibility across Linux (X11 & Wayland), Windows 10/11, and macOS. | Portability | All | Pending |

---

## Traceability Matrix

| Requirement | Phase | Target Module | Status |
|---|---|---|---|
| REQ-01 .. REQ-04 | Phase 1 | `crates/plainpad-core/src/store.rs`, `src-tauri` | Complete |
| REQ-05 .. REQ-07 | Phase 2 | `ui/src/slotted.ts`, `crates/plainpad-core/src/store.rs` | Complete |
| REQ-08 .. REQ-11 | Phase 3 | `crates/plainpad-core/src/mathwrap.rs`, `currency.rs` | Complete |
| REQ-12 .. REQ-14 | Phase 4 | `crates/plainpad-core/src/intent.rs`, `ui/src/editor.ts` | Complete |
| REQ-15 .. REQ-17 | Phase 5 | `crates/plainpad-core/src/timers.rs`, `ui/src/timer.ts` | Complete |
| REQ-18 .. REQ-22 | Phase 6 | `ui/src/editor.ts`, `ui/src/vim.ts` | Planned |
| REQ-23 .. REQ-25 | Phase 7 | `crates/plainpad-core/src/extensions.rs`, `ui/src/extensions.ts` | Planned |
| REQ-26 .. REQ-30 | Phase 8 | `ui/src/shaders.ts`, `export.rs`, `src-tauri/src/main.rs` | Planned |
