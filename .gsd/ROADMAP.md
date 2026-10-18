---
milestone: Antinote 1:1 Parity Reconstruction
version: 1.0.0
updated: 2026-10-13T20:14:00+05:30
---

# Roadmap

> **Current Phase:** 5 - Timer, Stopwatch & Pomodoro Subsystem
> **Status:** ready for planning

## Must-Haves (from SPEC)

- [x] SQLite persistence matching Antinote Core Data Model 9 (5 entities, 17 Note properties)
- [x] 9 Slotted Notes Greek-god system (Hermes through Zeus) with bottom bar and `Ctrl+1..9` hotkeys
- [x] Natural-language math engine supporting fractions, decimals, scientific notation, trig modes, and 6 conversion domains
- [x] Natural-language sentence trigger engine (`sum`, `avg`, `math`, `count`, `list`, `check`, `code`, `paste`, `timer`)
- [x] Timer and Pomodoro subsystem with circular visual progress and native audio chimes (`backtowork`, `countdowndone`, `workdone`)
- [x] Rich editor capabilities: interactive checkboxes (`- [ ]`), list autocompletion, Antilink URL pills, rounded syntax-highlighted code blocks, and modal Vim mode
- [ ] JavaScript extension runtime executing `antinote-extensions-base-v0.0.1.js` with secure API bridges and multi-provider AI model catalog
- [ ] Cross-platform desktop polish: glassmorphic floating window, global hotkey, system tray, The Void with WebGL shaders, canvas grid textures, and third-party export suite (Obsidian, Bear, Notion, Apple Notes, Custom URL, Markdown, ZIP)

---

## Phases

### Phase 1: SQLite Storage Engine & Core Data Model 9 Parity
**Status:** ✅ Complete  
**Objective:** Replace raw file storage with embedded SQLite engine reproducing Antinote's Core Data Model 9 (`Note` with 17 attributes, `ChecklistItem`, `Antilink`, `Attachment`, `SyncState`), soft-delete into The Void, note ordering (`dbIndex`), lock, archive, and pre-restore snapshots.  
**Requirements:** REQ-01, REQ-02, REQ-03, REQ-04  
**Plans:**
- [x] Plan 1.1: Core Data Model 9 SQLite schema & migrations in `plainpad-core`
- [x] Plan 1.2: Note lifecycle CRUD, soft delete (The Void), locking, and archiving
- [x] Plan 1.3: Checklist items, antilinks, attachments relational store & pre-restore snapshot manager

---

### Phase 2: Slotted Notes (Greek Gods System) & Quick Navigation
**Status:** ✅ Complete  
**Objective:** Implement the permanent 9-slot carousel named after Greek deities (Hermes, Apollo/Aphrodite, Ares, Cronus, Gaia, Hades, Ouranos, Poseidon, Zeus) with bottom indicator bar, active slot icons, keyboard shortcuts, and slot invariance protections.  
**Depends on:** Phase 1  
**Requirements:** REQ-05, REQ-06, REQ-07  
**Plans:**
- [x] Plan 2.1: Slotted notes backend store logic, initialization, and slot invariance enforcement
- [x] Plan 2.2: `SlottedNotesBottomBar` UI component with active deity indicators and hotkeys (`Ctrl+1..9`, `Ctrl+Tab`)

---

### Phase 3: Natural-Language Math & 6-Domain Unit Conversion Engine
**Status:** ✅ Complete  
**Objective:** Provide mathematical parsing for fractions, decimals, exponents, scientific notation, trig modes (`degrees` vs `radians`), and complete bidirectional unit conversion across Distance, Area, Mass/Weight, Volume, Temperature, and Currency (160+ currencies with background cache).  
**Depends on:** Phase 1  
**Requirements:** REQ-08, REQ-09, REQ-10, REQ-11  
**Plans:**
- [x] Plan 3.1: Math tokenizer with mixed fractions, exponents, implicit multiplication, and angle modes
- [x] Plan 3.2: 5 physical unit conversion domains (Distance, Area, Mass, Volume, Temperature) with regional aliases
- [x] Plan 3.3: Currency exchange engine with 160+ currencies, cache sync, thousand separators, and decimal precision

---

### Phase 4: Natural-Language Sentence Triggers & AutoPaste
**Status:** ✅ Complete  
**Objective:** Sentence-level trigger engine processing `sum`, `avg`, `math`, `count`, `list`, `check`, `code`, `paste`, and `timer` keywords with configurable aliases, master disable toggle, and continuous AutoPaste clipboard capture mode.  
**Depends on:** Phase 1, Phase 3  
**Requirements:** REQ-12, REQ-13, REQ-14  
**Plans:**
- [x] Plan 4.1: Trigger evaluator for `sum`, `avg`, `math`, `count`, `list`, `check`, `code` with alias mapping
- [x] Plan 4.2: AutoPaste continuous clipboard ingestion engine with `Esc` termination

---

### Phase 5: Timer, Stopwatch & Pomodoro Subsystem
**Status:** ✅ Complete  
**Objective:** Comprehensive inline timer and alarm subsystem handling stopwatch, count-down, and classic/custom Pomodoro cycles, interactive circular progress UI, and native audio alerts (`backtowork.m4a`, `countdowndone.m4a`, `workdone.m4a`).  
**Depends on:** Phase 1  
**Requirements:** REQ-15, REQ-16, REQ-17  
**Plans:**
- [x] Plan 5.1: Timer backend tick thread, state persistence, and parsing (`timer`, `timer 5`, `pomo`, `p`, `r`, `s`)
- [x] Plan 5.2: Timer UI pill with animated circular SVG/Canvas progress and native sound alert playback
- [x] Plan 5.3: Gap Closure: Timer reactivity on pause/stop and window hide/close pause integration

---

### Phase 6: Rich Editor Capabilities & Modal Vim Mode
**Status:** ✅ Complete  
**Objective:** Rich inline editing features: interactive checkboxes (`- [ ]`), list autocompletion, compact Antilink pills, rounded syntax-highlighted code blocks, typography shortcuts, caret vertical nudging, and full modal Vim mode.  
**Depends on:** Phase 1  
**Requirements:** REQ-18, REQ-19, REQ-20, REQ-21, REQ-22  
**Plans:**
- [x] Plan 6.1: Inline interactive checkboxes, list continuation, Antilink URL shortening, and rounded code blocks
- [x] Plan 6.2: Modal Vim mode engine (Normal, Insert, Visual, Visual Line) with debounced motion handling
- [x] Plan 6.3: Typography shortcuts, heading cycle, sub-pixel vertical caret nudging, and RTL layout

---

### Phase 7: JavaScript Extension Runtime & AI Model Catalog
**Status:** ✅ Complete  
**Objective:** Sandboxed JS runtime running `antinote-extensions-base-v0.0.1.js`, native secure `callAPI` bridge with credential store, double-colon invocation `::command(...)`, and multi-provider AI model catalog (OpenAI, Anthropic, Ollama).  
**Depends on:** Phase 1  
**Requirements:** REQ-23, REQ-24, REQ-25  
**Plans:**
- [x] Plan 7.1: Embedded JS runtime executing `antinote-extensions-base-v0.0.1.js` with secure `callAPI` bridge
- [x] Plan 7.2: AI Model Catalog integration (OpenAI, Anthropic, Ollama) and `::command` editor syntax

---

### Phase 8: Window Chrome, Void Shaders, Grid Overlays & Export Suite
**Status:** ✅ Complete  
**Objective:** Translucent glass floating window, global hotkey (`Ctrl+Space`), system tray, WebGL Void shaders (`voidRipple`, `voidDistortionSlow`, `voidDistortionBurst`), canvas grid textures, search modal, and third-party export suite (Markdown, Obsidian, Bear, Notion, Apple Notes, Custom URL, ZIP).  
**Depends on:** Phase 1, Phase 2, Phase 6  
**Requirements:** REQ-26, REQ-27, REQ-28, REQ-29, REQ-30  
**Plans:**
- [x] Plan 8.1: Window translucency, global summon hotkey, system tray icon, and background canvas grid textures
- [x] Plan 8.2: The Void WebGL distortion shaders and empty confirmation dialog
- [x] Plan 8.3: Search engine (`FindAndReplaceManager`) and third-party export suite (Obsidian, Bear, Notion, Apple Notes, URL template, ZIP)
- [x] Plan 8.4: Gap Closure: Balanced HTML tags and export modal styling

---

### Phase 9: Editor Calculation UX Polish
**Status:** ✅ Complete  
**Objective:** Suppress trivial identity number calculations (`20` -> no `= 20`), eliminate duplicate equals signs on manual typing (` =` stepping), and enable visual selection synchronization and rich clipboard copy for calculation answers.  
**Depends on:** Phase 3, Phase 6  
**Requirements:** UX-01, UX-02, UX-03  
**Plans:**
- [x] Plan 9.1: Trivial identity suppression, ghost equals absorption, and rich clipboard copy interceptor

---

### Phase 10: Pomodoro Break Screen & Fullscreen Takeover
**Status:** ✅ Complete  
**Objective:** High-fidelity Antinote break screen with mint-green orbit line-art graphic, rotating Wildlife Fact-File catalog, live interactive break countdown, "Sneak a peek" button with persistent "No-Peek Streak" gamification in SQLite, and cross-platform desktop blur / fullscreen takeover window.  
**Depends on:** Phase 5, Phase 8  
**Requirements:** REQ-31  
**Plans:**
- [x] Plan 10.1: Fact-File catalog, SQLite No-Peek streak engine, and high-fidelity Break Time view
- [x] Plan 10.2: Cross-platform fullscreen takeover window management and "Sneak a peek" unblur mechanics

---

## Progress Summary

| Phase | Description | Status | Plans | Complete |
|---|---|---|---|---|
| **Phase 1** | SQLite Storage & Core Data Model 9 | ✅ Complete | 3/3 | 100% |
| **Phase 2** | Slotted Notes (Greek Gods) | ✅ Complete | 2/2 | 100% |
| **Phase 3** | Math & 6-Domain Unit Conversions | ✅ Complete | 3/3 | 100% |
| **Phase 4** | Sentence Triggers & AutoPaste | ✅ Complete | 2/2 | 100% |
| **Phase 5** | Timer, Stopwatch & Pomodoro | ✅ Complete | 3/3 | 100% |
| **Phase 6** | Rich Editor & Modal Vim Mode | ✅ Complete | 3/3 | 100% |
| **Phase 7** | JS Extension Runtime & AI Catalog | ✅ Complete | 2/2 | 100% |
| **Phase 8** | Window Chrome, Shaders & Export Suite | ✅ Complete | 4/4 | 100% |
| **Phase 9** | Editor Calculation UX Polish | ✅ Complete | 1/1 | 100% |
| **Phase 10** | Pomodoro Break Screen & Fullscreen Takeover | ✅ Complete | 2/2 | 100% |

---

## Timeline

| Phase | Started | Completed | Duration |
|---|---|---|---|
| Phase 1 | 2026-10-13 | 2026-10-13 | ~45m |
| Phase 2 | 2026-10-13 | 2026-10-13 | ~30m |
| Phase 3 | 2026-10-13 | 2026-10-13 | ~45m |
| Phase 4 | 2026-10-13 | 2026-10-15 | ~40m |
| Phase 5 | 2026-10-15 | 2026-10-15 | ~45m |
| Phase 6 | 2026-10-15 | 2026-10-15 | ~50m |
| Phase 7 | 2026-10-15 | 2026-10-15 | ~40m |
| Phase 8 | 2026-10-16 | 2026-10-16 | ~1h |
| Phase 9 | 2026-10-17 | 2026-10-17 | ~45m |
| Phase 10 | 2026-10-18 | 2026-10-18 | ~30m |

