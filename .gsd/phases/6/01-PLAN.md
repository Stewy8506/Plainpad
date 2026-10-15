---
phase: 6
plan: 1
wave: 1
gap_closure: false
depends_on: []
files_modified:
  - ui/src/list_manager.ts
  - ui/src/rich_elements.ts
  - ui/src/main.ts
  - ui/styles.css
  - ui/src/settings.ts
  - ui/settings.html
  - crates/plainpad-core/src/intent.rs
  - crates/plainpad-core/src/store/relational.rs
  - crates/plainpad-core/tests/rich_editor_relational_tests.rs
autonomous: true
must_haves:
  truths:
    - "Typing or having `- [ ] ` or `- [x] ` or `[ ] ` or `[x] ` renders an interactive checkbox in the editor overlay"
    - "Clicking an overlay checkbox toggles `- [ ]` ↔ `- [x]` (and `[ ]` ↔ `[x]`) in plaintext, preserving indentation and triggering note save"
    - "Pressing Enter on bullet lists (`- `, `* `, `+ `) automatically continues the bullet on the new line with matching indentation"
    - "Pressing Enter on numbered lists (`1. `, `2. `) automatically increments the number (`2. `, `3. `) on the new line"
    - "Pressing Enter on an empty bullet or numbered list line terminates the list by removing the marker"
    - "URLs in note text are automatically detected and rendered as compact Antilink domain pills (`github.com/...`) while keeping full destination URLs intact in note plaintext"
    - "Clicking an Antilink pill opens the full destination URL in the default browser"
    - "Fenced code blocks (``` ... ```) render with rounded corner backgrounds and a Copy Code button that copies the block body to the clipboard"
    - "Checklist items and Antilinks synchronize into SQLite relational tables `checklist_items` and `antilinks` on note save"
  artifacts:
    - "ui/src/list_manager.ts"
    - "ui/src/rich_elements.ts"
    - "crates/plainpad-core/tests/rich_editor_relational_tests.rs"
  key_links:
    - "main.ts delegates Enter key handling and list continuation to list_manager.ts"
    - "main.ts delegates URL shortening and code block rendering to rich_elements.ts"
    - "settings.ts exposes inlineChecklistCheckboxesEnabled, inlineChecklistBulletsEnabled, inlineNumberedListsEnabled, disableLinkShortening, and disableLinkHighlighting"
---

# Plan 6.1: Inline Interactive Checkboxes, List Auto-Continuation, Antilinks, and Rounded Code Blocks

<objective>
Implement rich inline editor capabilities: interactive checkboxes (`- [ ]`, `- [x]`, `[ ]`, `[x]`) with bidirectional plaintext and relational database synchronization, list auto-continuation (`- `, `* `, `1. ` on Enter, terminating on blank Enter), compact Antilink pills with underlying destination retention and skip lists, and rounded syntax-highlighted code blocks with a "Copy Code" button.

Purpose: Antinote provides a seamless writing experience where plaintext markdown markers immediately render interactive visual components (checkboxes, pills, code blocks) without breaking plaintext editing or losing data.
Output: `ui/src/list_manager.ts`, `ui/src/rich_elements.ts`, enhanced `main.ts`, updated CSS styling, settings toggles, and integration test suite `rich_editor_relational_tests.rs`.
</objective>

<context>
Load these files for context:
- .gsd/SPEC.md (§7 Editor Capabilities & Rich Inline Elements, §11 Preferences)
- .gsd/findings.md (§15 Editor Controls & Typography Tweaks, §18 Checklist)
- .gsd/REQUIREMENTS.md (REQ-18, REQ-19, REQ-20, REQ-21)
- ui/src/main.ts
- ui/styles.css
- crates/plainpad-core/src/intent.rs
- crates/plainpad-core/src/store/relational.rs
</context>

<tasks>

<task type="auto">
  <name>Implement interactive inline checkboxes with bidirectional plaintext and relational synchronization</name>
  <files>
    ui/src/list_manager.ts
    ui/src/main.ts
    ui/styles.css
    crates/plainpad-core/src/intent.rs
  </files>
  <action>
    Create `ui/src/list_manager.ts` and update `main.ts`:
    1. Support Markdown checkbox markers: `- [ ] `, `- [x] `, `- [X] `, `* [ ] `, `* [x] `, `* [X] `, `[ ] `, `[x] `, `[X] `.
    2. In `crates/plainpad-core/src/intent.rs`, ensure line scanner annotates checklist items for all supported markers (`- [ ]`, `- [x]`, `* [ ]`, `* [x]`, `[ ]`, `[x]`).
    3. In `ui/src/list_manager.ts`, implement `toggleCheckboxLine(text: string, lineIndex: number): { nextText: string; isChecked: boolean }`:
       - Parse line at `lineIndex`. Detect leading indent and marker type.
       - If `- [ ] `, toggle to `- [x] `; if `- [x] ` or `- [X] `, toggle to `- [ ] `.
       - If `* [ ] `, toggle to `* [x] `; if `* [x] `, toggle to `* [ ] `.
       - If `[ ] `, toggle to `[x] `; if `[x] `, toggle to `[ ] `.
       - Preserve exact indentation and all text following the marker.
    4. In `ui/src/main.ts`, update `renderAnnotations()`:
       - When `anno.kind === 'checklist'`, check `inlineChecklistCheckboxesEnabled` setting (default true). If enabled, create `.anno-checkbox` element positioned over the checkbox glyph.
       - Attach click handler that calls `toggleCheckboxLine()`, updates `editor.value`, preserves cursor position/selection, schedules note save, and triggers re-analysis.
       - Checkbox rendering styles in `ui/styles.css`: 15x15px rounded-square or circle, subtle border `rgba(255,255,255,0.22)`, background transparent, animated checkmark SVG or glyph with accent fill `#0A84FF` on `.checked`, and optional slight text dimming for completed task items.
    
    AVOID: Modifying textarea value in a way that resets cursor to the start of the document.
    USE: Cache and restore `selectionStart` and `selectionEnd` offset relative to the toggle position.
  </action>
  <verify>
    npm run typecheck
  </verify>
  <done>
    Clicking overlay checkboxes toggles `- [ ]` ↔ `- [x]` in editor value, preserves indentation, updates note state, and respects `inlineChecklistCheckboxesEnabled`.
  </done>
</task>

<task type="auto">
  <name>Implement list auto-continuation and termination on Enter</name>
  <files>
    ui/src/list_manager.ts
    ui/src/main.ts
  </files>
  <action>
    In `ui/src/list_manager.ts`, implement list continuation handler `handleListEnterKey(editor: HTMLTextAreaElement, options: ListOptions): boolean`:
    1. Read `cursorPos = editor.selectionStart`. Extract current line up to cursor, and full current line.
    2. Check options: `options.bulletListsEnabled` (from `inlineChecklistBulletsEnabled`, default true) and `options.numberedListsEnabled` (from `inlineNumberedListsEnabled`, default true).
    3. Detect marker patterns:
       - Checkbox bullet: `^(\s*)([-*]\s+\[[ xX]\]\s+)(.*)$` or `^(\s*)(\[[ xX]\]\s+)(.*)$`
       - Bullet list: `^(\s*)([-*+]\s+)(.*)$`
       - Numbered list: `^(\s*)(\d+)([.)]\s+)(.*)$`
    4. Handle Termination (Blank item on Enter):
       - If the line content after the list marker is empty (i.e. `trim().length === 0`), pressing `Enter` must terminate the list!
       - Action: Delete the marker from the current line, leaving an empty line with cursor at the beginning, without inserting a new bullet.
    5. Handle Continuation (Non-empty item on Enter):
       - If checkbox item: insert `\n` + indent + `- [ ] ` (uncheck on next item).
       - If bullet item: insert `\n` + indent + matching bullet marker (e.g. `- `, `* `, `+ `).
       - If numbered item: parse integer `num = parseInt(digits)`, increment `num + 1`, insert `\n` + indent + `${num + 1}${delimiter} `.
    6. Update editor:
       - Use `document.execCommand('insertText', false, text)` where supported for undo stack preservation, or splice into `editor.value` and set `selectionStart = selectionEnd = newPos`.
       - Trigger `input` event and auto-resize.
    7. In `ui/src/main.ts`, attach `handleListEnterKey` inside the `editor.addEventListener('keydown', ...)` listener before default Enter propagation.
    
    AVOID: Breaking standard Shift+Enter or Ctrl+Enter behaviors.
    USE: Only intercept plain Enter key when cursor is within a list line.
  </action>
  <verify>
    npm run typecheck && npm run build:ui
  </verify>
  <done>
    Pressing Enter on bullet or numbered lists auto-continues the sequence; pressing Enter on an empty list line terminates the list by clearing the marker.
  </done>
</task>

<task type="auto">
  <name>Implement compact Antilink pills, rounded code blocks with Copy button, and relational sync</name>
  <files>
    ui/src/rich_elements.ts
    ui/src/main.ts
    ui/styles.css
    ui/src/settings.ts
    ui/settings.html
    crates/plainpad-core/src/store/relational.rs
    crates/plainpad-core/tests/rich_editor_relational_tests.rs
  </files>
  <action>
    Create `ui/src/rich_elements.ts` and add tests:
    1. **Smart Antilinks:**
       - Detect URLs in line text using regex: `https?:\/\/[^\s<>"'{}|\\^`]+`.
       - Compute shortened domain pill display: e.g. `https://github.com/rust-lang/rust` → `github.com/...`, `https://news.ycombinator.com/item?id=123` → `news.ycombinator.com/...`.
       - Skip list / settings: honor `disableLinkShortening` (if true, show full URL) and `disableLinkHighlighting` (if true, omit pill container).
       - In overlay, render `.antilink-pill` element with domain favicon/link icon, shortened label, and tooltip with full destination URL.
       - Clicking pill calls `window.open(url, '_blank')` or Tauri shell open.
    2. **Rounded Code Blocks (`RoundedCodeLayoutManager`):**
       - Scan text for fenced code blocks delimited by triple backticks: ```` ```[a-z0-9_-]*\n[\s\S]*?\n``` ````.
       - In overlay/backdrop, render `.rounded-code-block` with rounded corners (`border-radius: 8px`), dark translucent background (`rgba(20, 24, 32, 0.75)`), border `1px solid rgba(255,255,255,0.1)`, and top-right "Copy Code" button.
       - Language badge (e.g. `rust`, `typescript`, `python`) displayed subtly in code block header.
       - Clicking "Copy Code" extracts the code body, writes to `navigator.clipboard.writeText(code)`, and updates button label to "Copied!" for 1.8s.
    3. **Relational Database Synchronization:**
       - When note is saved, extract all checklist items (`- [ ]`, `- [x]`) and Antilinks from text.
       - Call `replace_checklist_items` and `replace_antilinks` in `plainpad-core` store to keep Core Data Model 9 relational entities synchronized with plaintext note content.
    4. **Settings UI Integration:**
       - In `settings.html` and `settings.ts`, wire toggles for `inlineChecklistCheckboxesEnabled`, `inlineChecklistBulletsEnabled`, `inlineNumberedListsEnabled`, `disableLinkShortening`, and `disableLinkHighlighting`.
    5. **Tests:**
       - Add integration test in `crates/plainpad-core/tests/rich_editor_relational_tests.rs` verifying checklist items and antilinks insertion, replacement, and query.
  </action>
  <verify>
    cargo test -p plainpad-core --test rich_editor_relational_tests && npm run typecheck
  </verify>
  <done>
    URLs render as compact clickable pills; code blocks render with rounded corners and functional Copy Code button; checklist and antilink records are synchronized in SQLite store.
  </done>
</task>

</tasks>

## Must-Haves
After all tasks complete, verify:
- [ ] `- [ ] ` and `- [x] ` render interactive overlay checkboxes that toggle plaintext state on click
- [ ] Enter key auto-increments numbered lists (`1. ` -> `2. `) and continues bullet lists (`- `)
- [ ] Pressing Enter on an empty list bullet removes the bullet and terminates the list
- [ ] URLs render as compact Antilink domain pills without mutating underlying plaintext
- [ ] Fenced code blocks display rounded background container and Copy Code button
- [ ] Settings toggles for checklist, bullets, numbers, and link shortening persist and take effect

## Success Criteria
- [ ] All tasks verified passing
- [ ] `npm run typecheck` passes with zero errors
- [ ] `cargo test -p plainpad-core --test rich_editor_relational_tests` passes with 100% green tests
- [ ] No regressions in existing tests
