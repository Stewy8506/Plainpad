---
milestone: Antinote 1:1 Parity Reconstruction
version: 1.0.0
updated: 2026-10-13T19:20:30+05:30
---

# Roadmap

> **Current Phase:** 1 - SQLite Storage Engine & Core Data Model 9 Parity
> **Status:** planning

## Must-Haves (from SPEC)

- [ ] SQLite persistence matching Antinote Core Data Model 9 (5 entities, 17 Note properties)
- [ ] 9 Slotted Notes Greek-god system (Hermes through Zeus) with bottom bar and `Ctrl+1..9` hotkeys
- [ ] Natural-language math engine supporting fractions, decimals, scientific notation, trig modes, and 6 conversion domains
- [ ] Natural-language sentence trigger engine (`sum`, `avg`, `math`, `count`, `list`, `check`, `code`, `paste`, `timer`)
- [ ] Timer and Pomodoro subsystem with circular visual progress and native audio chimes (`backtowork`, `countdowndone`, `workdone`)
- [ ] Rich editor capabilities: interactive checkboxes (`- [ ]`), list autocompletion, Antilink URL pills, rounded syntax-highlighted code blocks, and modal Vim mode
- [ ] JavaScript extension runtime executing `antinote-extensions-base-v0.0.1.js` with secure API bridges and multi-provider AI model catalog
- [ ] Cross-platform desktop polish: glassmorphic floating window, global hotkey, system tray, The Void with WebGL shaders, canvas grid textures, and third-party export suite (Obsidian, Bear, Notion, Apple Notes, Custom URL, Markdown, ZIP)

---

## Phases

### Phase 1: SQLite Storage Engine & Core Data Model 9 Parity
**Status:** 🔄 In Progress  
**Objective:** Replace raw file storage with embedded SQLite engine reproducing Antinote's Core Data Model 9 (`Note` with 17 attributes, `ChecklistItem`, `Antilink`, `Attachment`, `SyncState`), soft-delete into The Void, note ordering (`dbIndex`), lock, archive, and pre-restore snapshots.  
**Requirements:** REQ-01, REQ-02, REQ-03, REQ-04  
**Plans:**
- [ ] Plan 1.1: Core Data Model 9 SQLite schema & migrations in `plainpad-core`
- [ ] Plan 1.2: Note lifecycle CRUD, soft delete (The Void), locking, and archiving
- [ ] Plan 1.3: Checklist items, antilinks, attachments relational store & pre-restore snapshot manager

---

### Phase 2: Slotted Notes (Greek Gods System) & Quick Navigation
**Status:** ⬜ Not Started  
**Objective:** Implement the permanent 9-slot carousel named after Greek deities (Hermes, Apollo/Aphrodite, Ares, Cronus, Gaia, Hades, Ouranos, Poseidon, Zeus) with bottom indicator bar, active slot icons, keyboard shortcuts, and slot invariance protections.  
**Depends on:** Phase 1  
**Requirements:** REQ-05, REQ-06, REQ-07  
**Plans:**
- [ ] Plan 2.1: Slotted notes backend store logic, initialization, and slot invariance enforcement
- [ ] Plan 2.2: `SlottedNotesBottomBar` UI component with active deity indicators and hotkeys (`Ctrl+1..9`, `Ctrl+Tab`)

---

### Phase 3: Natural-Language Math & 6-Domain Unit Conversion Engine
**Status:** ⬜ Not Started  
**Objective:** Provide mathematical parsing for fractions, decimals, exponents, scientific notation, trig modes (`degrees` vs `radians`), and complete bidirectional unit conversion across Distance, Area, Mass/Weight, Volume, Temperature, and Currency (160+ currencies with background cache).  
**Depends on:** Phase 1  
**Requirements:** REQ-08, REQ-09, REQ-10, REQ-11  
**Plans:**
- [ ] Plan 3.1: Math tokenizer with mixed fractions, exponents, implicit multiplication, and angle modes
- [ ] Plan 3.2: 5 physical unit conversion domains (Distance, Area, Mass, Volume, Temperature) with regional aliases
- [ ] Plan 3.3: Currency exchange engine with 160+ currencies, cache sync, thousand separators, and decimal precision

---

### Phase 4: Natural-Language Sentence Triggers & AutoPaste
**Status:** ⬜ Not Started  
**Objective:** Sentence-level trigger engine processing `sum`, `avg`, `math`, `count`, `list`, `check`, `code`, `paste`, and `timer` keywords with configurable aliases, master disable toggle, and continuous AutoPaste clipboard capture mode.  
**Depends on:** Phase 1, Phase 3  
**Requirements:** REQ-12, REQ-13, REQ-14  
**Plans:**
- [ ] Plan 4.1: Trigger evaluator for `sum`, `avg`, `math`, `count`, `list`, `check`, `code` with alias mapping
- [ ] Plan 4.2: AutoPaste continuous clipboard ingestion engine with `Esc` termination

---

### Phase 5: Timer, Stopwatch & Pomodoro Subsystem
**Status:** ⬜ Not Started  
**Objective:** Comprehensive inline timer and alarm subsystem handling stopwatch, count-down, and classic/custom Pomodoro cycles, interactive circular progress UI, and native audio alerts (`backtowork.m4a`, `countdowndone.m4a`, `workdone.m4a`).  
**Depends on:** Phase 1  
**Requirements:** REQ-15, REQ-16, REQ-17  
**Plans:**
- [ ] Plan 5.1: Timer backend tick thread, state persistence, and parsing (`timer`, `timer 5`, `pomo`, `p`, `r`, `s`)
- [ ] Plan 5.2: Timer UI pill with animated circular SVG/Canvas progress and native sound alert playback

---

### Phase 6: Rich Editor Capabilities & Modal Vim Mode
**Status:** ⬜ Not Started  
**Objective:** Rich inline editing features: interactive checkboxes (`- [ ]`), list autocompletion, compact Antilink pills, rounded syntax-highlighted code blocks, typography shortcuts, caret vertical nudging, and full modal Vim mode.  
**Depends on:** Phase 1  
**Requirements:** REQ-18, REQ-19, REQ-20, REQ-21, REQ-22  
**Plans:**
- [ ] Plan 6.1: Inline interactive checkboxes, list continuation, Antilink URL shortening, and rounded code blocks
- [ ] Plan 6.2: Modal Vim mode engine (Normal, Insert, Visual, Visual Line) with debounced motion handling
- [ ] Plan 6.3: Typography shortcuts, heading cycle, sub-pixel vertical caret nudging, and RTL layout

---

### Phase 7: JavaScript Extension Runtime & AI Model Catalog
**Status:** ⬜ Not Started  
**Objective:** Sandboxed JS runtime running `antinote-extensions-base-v0.0.1.js`, native secure `callAPI` bridge with credential store, double-colon invocation `::command(...)`, and multi-provider AI model catalog (OpenAI, Anthropic, Ollama).  
**Depends on:** Phase 1  
**Requirements:** REQ-23, REQ-24, REQ-25  
**Plans:**
- [ ] Plan 7.1: Embedded JS runtime executing `antinote-extensions-base-v0.0.1.js` with secure `callAPI` bridge
- [ ] Plan 7.2: AI Model Catalog integration (OpenAI, Anthropic, Ollama) and `::command` editor syntax

---

### Phase 8: Window Chrome, Void Shaders, Grid Overlays & Export Suite
**Status:** ⬜ Not Started  
**Objective:** Translucent glass floating window, global hotkey (`Ctrl+Space`), system tray, WebGL Void shaders (`voidRipple`, `voidDistortionSlow`, `voidDistortionBurst`), canvas grid textures, search modal, and third-party export suite (Markdown, Obsidian, Bear, Notion, Apple Notes, Custom URL, ZIP).  
**Depends on:** Phase 1, Phase 2, Phase 6  
**Requirements:** REQ-26, REQ-27, REQ-28, REQ-29, REQ-30  
**Plans:**
- [ ] Plan 8.1: Window translucency, global summon hotkey, system tray icon, and background canvas grid textures
- [ ] Plan 8.2: The Void WebGL distortion shaders and empty confirmation dialog
- [ ] Plan 8.3: Search engine (`FindAndReplaceManager`) and third-party export suite (Obsidian, Bear, Notion, Apple Notes, URL template, ZIP)

---

## Progress Summary

| Phase | Description | Status | Plans | Complete |
|---|---|---|---|---|
| **Phase 1** | SQLite Storage & Core Data Model 9 | 🔄 In Progress | 0/3 | 0% |
| **Phase 2** | Slotted Notes (Greek Gods) | ⬜ Not Started | 0/2 | 0% |
| **Phase 3** | Math & 6-Domain Unit Conversions | ⬜ Not Started | 0/3 | 0% |
| **Phase 4** | Sentence Triggers & AutoPaste | ⬜ Not Started | 0/2 | 0% |
| **Phase 5** | Timer, Stopwatch & Pomodoro | ⬜ Not Started | 0/2 | 0% |
| **Phase 6** | Rich Editor & Modal Vim Mode | ⬜ Not Started | 0/3 | 0% |
| **Phase 7** | JS Extension Runtime & AI Catalog | ⬜ Not Started | 0/2 | 0% |
| **Phase 8** | Window Chrome, Shaders & Export Suite | ⬜ Not Started | 0/3 | 0% |

---

## Timeline

| Phase | Started | Completed | Duration |
|---|---|---|---|
| Phase 1 | 2026-10-13 | — | — |
| Phase 2 | — | — | — |
| Phase 3 | — | — | — |
| Phase 4 | — | — | — |
| Phase 5 | — | — | — |
| Phase 6 | — | — | — |
| Phase 7 | — | — | — |
| Phase 8 | — | — | — |
