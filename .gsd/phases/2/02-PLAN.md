---
phase: 2
plan: 2
wave: 2
depends_on: ["01"]
files_modified:
  - ui/src/slotted.ts
  - ui/src/main.ts
  - ui/index.html
  - ui/styles.css
autonomous: true
gap_closure: false
must_haves:
  truths:
    - "Bottom bar renders 9 permanent indicator pills for Hermes, Apollo, Ares, Cronus, Gaia, Hades, Ouranos, Poseidon, and Zeus"
    - "Active indicator pill highlights the currently selected slotted note"
    - "Ctrl+1 through Ctrl+9 keyboard shortcuts switch directly to corresponding slotted notes"
    - "Ctrl+Tab and Ctrl+Shift+Tab cycle sequentially between slotted notes"
    - "Slotted notes are protected against empty-note auto-deletion and deletion button triggers"
  artifacts:
    - "ui/src/slotted.ts"
    - "ui/src/main.ts"
    - "ui/index.html"
    - "ui/styles.css"
---

# Plan 2.2: SlottedNotesBottomBar UI Component with Active Deity Indicators and Hotkeys

<objective>
Implement the frontend `SlottedNotesBottomBar` indicator bar and `SlottedNoteIconView` components displaying 9 Greek deity indicators (Hermes through Zeus). Wire instant keyboard navigation (`Ctrl+1..9`, `Ctrl+Tab`, `Ctrl+Shift+Tab`), click-to-switch handlers, active slot highlighting, and front-end deletion protections for slotted notes.

Purpose: Antinote features a persistent bottom indicator bar showing the 9 slotted notes with active state feedback and instant hotkeys, allowing rapid scratchpad switching while guaranteeing slot permanence.
Output: `ui/src/slotted.ts`, integrated bottom bar in `ui/index.html` and `ui/styles.css`, and navigation/invariance event handling in `ui/src/main.ts`.
</objective>

<context>
Load these files for context:
- .gsd/SPEC.md (§3 The 9 Slotted Notes Architecture (Greek Gods System))
- .gsd/findings.md (§4 The 9 Slotted Notes Architecture)
- ui/index.html
- ui/styles.css
- ui/src/main.ts
- .gsd/phases/2/01-PLAN.md
</context>

<tasks>

<task type="auto">
  <name>Create SlottedNotesBottomBar component, SlottedNoteIconView pill renderers, and CSS styling</name>
  <files>
    ui/src/slotted.ts
    ui/index.html
    ui/styles.css
  </files>
  <action>
    1. Create `ui/src/slotted.ts` exporting:
       - `DeitySlot` interface:
         ```ts
         export interface DeitySlot {
           index: number;
           name: string;
           shortcut: string;
         }
         ```
       - `DEITY_SLOTS`: The 9 Greek deities in order:
         - 0: Hermes (`Ctrl+1`)
         - 1: Apollo (`Ctrl+2`)
         - 2: Ares (`Ctrl+3`)
         - 3: Cronus (`Ctrl+4`)
         - 4: Gaia (`Ctrl+5`)
         - 5: Hades (`Ctrl+6`)
         - 6: Ouranos (`Ctrl+7`)
         - 7: Poseidon (`Ctrl+8`)
         - 8: Zeus (`Ctrl+9`)
       - `SlottedNotesBottomBar` class:
         - Constructor accepts `container: HTMLElement` and `onSelectSlot: (index: number) => void`.
         - `render()` method:
           - Empties container and renders 9 pill elements with class `.slotted-pill`.
           - Sets attributes `data-slot="0..8"`, `title="${name} (${shortcut})"`.
           - Renders an inner indicator dot/glyph (`.slotted-pill-dot`).
           - Attaches `click` event listener to each pill invoking `onSelectSlot(index)`.
         - `setActiveSlot(slotIndex: number | null)`:
           - Toggles `.active` class: adds `.active` to pill whose `data-slot` matches `slotIndex`, removes `.active` from all other pills.
           - If `slotIndex === null` or out of bounds (such as when viewing an unslotted scratch note), clears `.active` from all pills.
         - `getActiveSlot(): number | null`: Returns currently active slot index.
    2. In `ui/index.html`:
       - Insert `<div id="slotted-bar" class="slotted-bar"></div>` in the bottom area (centered alongside or integrated into `#bottom-bar`).
    3. In `ui/styles.css`:
       - Add layout and glassmorphism styling for `.slotted-bar`:
         - Centered flex row, subtle gap between pills, positioned cleanly without overlapping bottom chrome buttons.
       - Add styling for `.slotted-pill` and `.slotted-pill-dot`:
         - Semi-transparent interactive pill (subtle dot/pill in default state).
         - Hover state: increased brightness, subtle hover scale/lift.
         - Active state (`.slotted-pill.active`): prominent glow and accent color indicating the current open deity scratchpad.

    AVOID: using fixed non-responsive widths or breaking existing editor padding.
    USE: clean semantic CSS classes and variables matching Plainpad's dark/light translucent glass themes.
  </action>
  <verify>npm run typecheck && npm run build:ui</verify>
  <done>TypeScript compiles ui/src/slotted.ts to ui/dist/slotted.js without errors and #slotted-bar renders 9 deity indicator pills.</done>
</task>

<task type="auto">
  <name>Implement slot navigation hotkeys (Ctrl+1..9, Ctrl+Tab), IPC integration, and deletion protections</name>
  <files>
    ui/src/main.ts
    ui/src/slotted.ts
  </files>
  <action>
    1. In `ui/src/main.ts`:
       - Import `SlottedNotesBottomBar`, `DEITY_SLOTS` from `./slotted.js`.
       - Update `NoteMeta` interface to match Core Data Model 9 fields from `SqliteStore`:
         ```ts
         interface NoteMeta {
           id: string;
           title: string;
           last_modified: string;
           db_index: number;
           is_slotted: boolean;
           slot_index: number;
           is_locked: boolean;
           is_archived: boolean;
           is_private: boolean;
           is_tutorial: boolean;
           soft_deleted: boolean;
           deleted_at: string | null;
         }
         ```
       - Update `Note` interface: `{ id: string; content: string; is_slotted: boolean; slot_index: number; ... }` and ensure text editor uses `note.content`.
       - Initialize `SlottedNotesBottomBar` with callback `(slotIndex) => switchToSlot(slotIndex)` targeting `#slotted-bar`.
       - Implement `switchToSlot(slotIndex: number)`:
         - Await `saveCurrentNote()` before switching to prevent data loss.
         - Load slotted note from backend via `invoke<Note>('get_slotted_note', { slot: slotIndex })`.
         - Update editor value, update `currentId`, `currentText`, update `noteIndex` display (e.g. `Hermes (1/9)`), and call `slottedBar.setActiveSlot(slotIndex)`.
       - In `loadNote(index)`:
         - When loading a note from the active notes stack, call `slottedBar.setActiveSlot(meta.is_slotted ? meta.slot_index : null)`.
    2. Implement Hotkeys:
       - Direct slot jump `Ctrl+1` through `Ctrl+9` (or `Cmd+1..9`):
         - Intercept `e.ctrlKey || e.metaKey`, match `e.key` in `'1'..'9'`.
         - Call `e.preventDefault()`, calculate `slot = parseInt(e.key) - 1`, call `switchToSlot(slot)`.
       - Cycle navigation `Ctrl+Tab` / `Ctrl+Shift+Tab`:
         - Intercept `(e.ctrlKey || e.metaKey) && e.key === 'Tab'`.
         - Call `e.preventDefault()`.
         - If currently viewing a slotted note with `slotIndex >= 0`:
           - If `e.shiftKey`, previous slot: `(slotIndex + 8) % 9`.
           - Else, next slot: `(slotIndex + 1) % 9`.
           - Call `switchToSlot(targetSlot)`.
         - If currently viewing an unslotted note: jump to slot 0 (`Hermes`).
    3. Enforce Slotted Note Protections in Editor UI:
       - In `saveCurrentNote()`:
         - Fix empty note auto-delete: if `text.trim() === ''` AND `currentNoteIsSlotted`:
           - DO NOT invoke `delete_note`. Slotted notes are permanent scratchpads and remain active even when empty!
           - Save the empty text to backend `save_note`.
       - In `deleteCurrentNote()` / `btnDelete`:
         - If current note is slotted (`is_slotted == true`):
           - Block deletion; do NOT call `delete_note`.
           - Provide user feedback (brief status bar message: "Slotted notes cannot be deleted" per findings §4).
       - In `btnPin`:
         - Disable or hide pin action on slotted notes since their position is permanently fixed in the slot carousel.

    AVOID: browser tab switching stealing Ctrl+Tab without preventDefault().
    USE: unified switchToSlot routine that guarantees state consistency and syncs with the bottom indicator bar.
  </action>
  <verify>npm run build:ui && cargo check --manifest-path src-tauri/Cargo.toml</verify>
  <done>Ctrl+1..9 jumps to slots 0..8, Ctrl+Tab cycles slots, indicator bar reflects active slot, empty slotted notes persist, and delete actions are prevented on slotted notes.</done>
</task>

</tasks>

<verification>
After all tasks, verify:
- [ ] Bottom bar displays 9 deity indicators (Hermes through Zeus) with tooltips
- [ ] Active indicator highlights the currently viewed slotted note
- [ ] Ctrl+1 through Ctrl+9 navigate directly to slots 0 through 8
- [ ] Ctrl+Tab and Ctrl+Shift+Tab cycle sequentially between slotted notes
- [ ] Slotted notes cannot be deleted via the delete button and empty content does not auto-delete them
- [ ] Unslotted scratch notes function normally and unhighlight the slotted indicators
</verification>

<success_criteria>
- [ ] All tasks verified via build and typecheck
- [ ] Must-haves confirmed
</success_criteria>
