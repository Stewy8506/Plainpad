---
phase: 6
plan: 3
wave: 3
gap_closure: false
depends_on:
  - "6.1"
  - "6.2"
files_modified:
  - ui/src/typography.ts
  - ui/src/main.ts
  - ui/styles.css
  - ui/src/settings.ts
  - ui/settings.html
  - ui/index.html
  - crates/plainpad-core/tests/typography_rtl_tests.rs
autonomous: true
must_haves:
  truths:
    - "Heading cycle shortcut (Ctrl+H / Cmd+H) cycles current line prefix through Body → # (H1) → ## (H2) → ### (H3) → Body"
    - "Markdown formatting shortcuts wrap selections or insert empty tokens: Bold (** Ctrl+B), Italic (* Ctrl+I), Strikethrough (~~ Ctrl+Shift+X), Underline (<u> Ctrl+U)"
    - "Toggle Read-Only mode (Ctrl+Shift+R or settings) prevents accidental editing on the current note and displays a lock icon indicator"
    - "Sub-pixel vertical caret nudging (_caretVerticalNudgeRaw) adjusts baseline cursor alignment for custom fonts via CSS variables"
    - "Caret vertical nudging for lists (_caretVerticalNudgeForListsRaw) applies distinct baseline compensation when cursor is on a checklist or bullet list item"
    - "Right-To-Left text orientation (_forceRTLStorage / forceRTL) toggles dir='rtl' and flips editor text alignment and annotation overlay coordinates"
  artifacts:
    - "ui/src/typography.ts"
    - "crates/plainpad-core/tests/typography_rtl_tests.rs"
  key_links:
    - "main.ts delegates typography shortcut keydowns and caret position detection to typography.ts"
    - "typography.ts adjusts CSS custom properties --caret-vertical-nudge and --caret-vertical-nudge-list on the editor container"
    - "settings.ts exposes controls for _caretVerticalNudgeRaw, _caretVerticalNudgeForListsRaw, and forceRTL"
---

# Plan 6.3: Typography Shortcuts, Heading Cycle, Sub-Pixel Caret Nudging, and RTL Layout

<objective>
Implement in-editor typography shortcuts (Heading cycle `#`, `##`, `###`, Body; Bold `**`, Italic `*`, Strikethrough `~~`, Underline, Toggle Read-Only), sub-pixel vertical caret nudging (`_caretVerticalNudgeRaw`, `_caretVerticalNudgeForListsRaw`) for custom fonts and list items, and Right-to-Left (RTL) text orientation (`_forceRTLStorage` / `forceRTL`) with mirrored overlay coordinates.

Purpose: Antinote provides precise typography control, seamless Markdown styling shortcuts, baseline caret tuning for custom monospace/sans fonts, and native RTL writing direction support.
Output: `ui/src/typography.ts`, updated `main.ts`, new CSS variables and styles, settings controls in `settings.html`, and automated test suite `typography_rtl_tests.rs`.
</objective>

<context>
Load these files for context:
- .gsd/SPEC.md (§7 Editor Capabilities & Rich Inline Elements, §11 Preferences)
- .gsd/findings.md (§15 Editor Controls & Typography Tweaks, §18 Checklist)
- .gsd/REQUIREMENTS.md (REQ-28)
- ui/src/main.ts
- ui/styles.css
- ui/src/settings.ts
- ui/settings.html
- ui/index.html
</context>

<tasks>

<task type="auto">
  <name>Implement in-editor typography shortcuts, heading cycle, and read-only mode</name>
  <files>
    ui/src/typography.ts
    ui/src/main.ts
    ui/styles.css
    ui/index.html
  </files>
  <action>
    Create `ui/src/typography.ts` and integrate into `main.ts`:
    1. **Heading Cycle (`cycleHeadingLevel(text: string, cursorPos: number)`):**
       - Triggered by `Ctrl+H` (or `Cmd+H` on macOS).
       - Locate line containing `cursorPos`.
       - Check line prefix:
         - If starts with `### `, replace with `` (strip prefix to return to Body text).
         - If starts with `## `, replace with `### `.
         - If starts with `# `, replace with `## `.
         - If no heading prefix, prepend `# `.
       - Preserve relative cursor position within the line text.
    2. **Inline Formatting Shortcuts:**
       - Bold (`Ctrl+B` / `Cmd+B`): wrap selection with `**`...`**`. If already wrapped, unwrap. If selection is empty, insert `****` and place cursor between asterisks.
       - Italic (`Ctrl+I` / `Cmd+I`): wrap selection with `*`...`*`. If selection is empty, insert `**` and place cursor between.
       - Strikethrough (`Ctrl+Shift+X` or `Ctrl+Alt+S`): wrap selection with `~~`...`~~`.
       - Underline (`Ctrl+U` / `Cmd+U`): wrap selection with `<u>`...`</u>`.
    3. **Toggle Read-Only Mode:**
       - Triggered by `Ctrl+Shift+R` or settings toggle.
       - When active: set `editor.readOnly = true`, show discreet `#readonly-badge` lock icon in the chrome bar, and disallow typing/cutting.
       - Pressing `Ctrl+Shift+R` again sets `editor.readOnly = false` and removes the lock badge.
    4. In `ui/src/main.ts`, hook these shortcuts into the editor `keydown` listener, preventing default browser actions (such as browser bookmark or underline) when Plainpad shortcuts match.
  </action>
  <verify>
    npm run typecheck && npm run build:ui
  </verify>
  <done>
    Heading cycle toggles Body → # → ## → ### → Body; Bold, Italic, Strikethrough, and Underline shortcuts format selections cleanly; Read-Only mode locks editor and displays lock indicator.
  </done>
</task>

<task type="auto">
  <name>Implement sub-pixel vertical caret nudging engine for custom fonts and lists</name>
  <files>
    ui/src/typography.ts
    ui/src/main.ts
    ui/styles.css
    ui/src/settings.ts
    ui/settings.html
  </files>
  <action>
    In `ui/src/typography.ts`, implement caret vertical nudging:
    1. **CSS Variables & Baseline Adjustment:**
       - Define `--caret-vertical-nudge: 0px` and `--caret-vertical-nudge-list: 0px` in `ui/styles.css` applied to `#editor-container`, `#editor`, `#editor-mirror`, and `#editor-backdrop`.
       - Support preferences:
         - `_caretVerticalNudgeRaw`: base cursor baseline offset in pixels (range `-5.0px` to `+5.0px`, step `0.25px`, default `0px`).
         - `_caretVerticalNudgeForListsRaw`: extra vertical baseline offset when cursor is on a checklist or bullet list item (range `-5.0px` to `+5.0px`, default `0px`).
    2. **Dynamic List Line Detection:**
       - On editor selection change / input / cursor motion:
         - Inspect line containing cursor.
         - If line matches list or checklist pattern (`- [ ]`, `- [x]`, `- `, `* `, `1. `): apply `--caret-vertical-nudge-list` offset so the caret aligns precisely with bullet and checkbox centers.
         - If ordinary body line: clear list offset.
    3. **Settings Controls:**
       - In `settings.html` (Appearance / Typography section), add range sliders and number badges for "Caret Baseline Nudge (`_caretVerticalNudgeRaw`)" and "List Caret Nudge (`_caretVerticalNudgeForListsRaw`)".
       - In `settings.ts`, bind changes to immediately update editor CSS variables and persist values to `localStorage` and `AppSettings`.
  </action>
  <verify>
    npm run typecheck && npm run build:ui
  </verify>
  <done>
    Caret vertical position responds to _caretVerticalNudgeRaw and dynamically applies _caretVerticalNudgeForListsRaw on list lines; settings controls adjust baseline alignment in real-time.
  </done>
</task>

<task type="auto">
  <name>Implement Right-to-Left (RTL) layout, mirrored overlay coordinates, and test suite</name>
  <files>
    ui/src/main.ts
    ui/styles.css
    ui/src/settings.ts
    ui/settings.html
    crates/plainpad-core/tests/typography_rtl_tests.rs
  </files>
  <action>
    Implement Right-To-Left (RTL) layout support and comprehensive testing:
    1. **RTL Writing Direction (`_forceRTLStorage` / `forceRTL`):**
       - Preference `forceRTL` (default `false`).
       - Hotkey toggle: `Ctrl+Shift+L` or `Ctrl+Shift+D` to toggle RTL mode instantly.
       - When `forceRTL` is true:
         - Set `dir="rtl"` on `#editor`, `#editor-backdrop`, `#editor-mirror`, and `#editor-container`.
         - Apply `direction: rtl; text-align: right;` in `ui/styles.css`.
    2. **Mirrored Annotation Overlay Calculations:**
       - In `main.ts` `renderAnnotations()`:
         - In LTR mode: annotation chips and checkboxes are positioned using `left: ${calc}px`.
         - In RTL mode: calculate horizontal positioning from the right margin (`right: ${calc}px`), ensuring math chips, trigger badges, and Antilink pills align naturally with RTL text flow without overlapping Persian, Arabic, or Hebrew characters.
    3. **Settings UI:**
       - In `settings.html` (under Appearance / Editor), add toggle "Right-to-Left (RTL) Layout (`_forceRTLStorage`)".
       - In `settings.ts`, wire toggle to persist and broadcast `forceRTL` state.
    4. **Automated Test Suite:**
       - Create `crates/plainpad-core/tests/typography_rtl_tests.rs` testing heading cycle transitions, formatting tag insertion, RTL preference persistence, and checklist/nudge configuration values.
  </action>
  <verify>
    cargo test -p plainpad-core --test typography_rtl_tests && npm run typecheck
  </verify>
  <done>
    forceRTL properly switches editor text direction and mirrors overlay annotations; typography shortcuts and caret nudging settings persist; automated tests pass with 100% success.
  </done>
</task>

</tasks>

## Must-Haves
After all tasks complete, verify:
- [ ] Heading cycle shortcut (Ctrl+H) cycles current line through Body → # → ## → ### → Body
- [ ] Bold (Ctrl+B), Italic (Ctrl+I), Strikethrough (Ctrl+Shift+X), and Underline (Ctrl+U) wrap text selections correctly
- [ ] Read-Only mode (Ctrl+Shift+R) locks editor from modifications and displays lock badge
- [ ] Caret baseline nudges vertically via _caretVerticalNudgeRaw and dynamically on list lines via _caretVerticalNudgeForListsRaw
- [ ] RTL mode (forceRTL) sets text direction to right-to-left and aligns overlay annotations to the right edge
- [ ] Settings controls for caret nudging and RTL persist across sessions

## Success Criteria
- [ ] All tasks verified passing
- [ ] `npm run typecheck` and `npm run build:ui` pass with zero errors
- [ ] `cargo test -p plainpad-core --test typography_rtl_tests` passes with 100% green tests
- [ ] No regressions in existing tests
