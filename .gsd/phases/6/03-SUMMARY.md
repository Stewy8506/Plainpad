# Phase 6 Plan 3: Typography Shortcuts, Heading Cycle, Sub-Pixel Caret Nudging, and RTL Layout Summary

## Implementation Overview

In this plan, we implemented comprehensive in-editor typography shortcuts, baseline caret nudging with list compensation, and native Right-to-Left (RTL) writing direction with mirrored overlay coordinates matching Antinote 1:1 parity.

### 1. In-Editor Typography & Formatting Shortcuts (`ui/src/typography.ts`, `ui/src/main.ts`)
- **Heading Cycle (`cycleHeadingLevel`):**
  - Triggered by `Ctrl+H` / `Cmd+H`.
  - Cycles current line prefix seamlessly: `Body` → `# ` (H1) → `## ` (H2) → `### ` (H3) → `Body` (prefix stripped).
  - Accurately tracks and preserves the relative cursor position within the edited line text across heading changes.
- **Inline Markdown Formatting (`applyInlineFormat`):**
  - **Bold (`Ctrl+B` / `Cmd+B`):** Wraps selection with `**`...`**`. If already wrapped (internally or directly enclosing), unwraps. If selection is empty, inserts `****` and positions caret between asterisks.
  - **Italic (`Ctrl+I` / `Cmd+I`):** Wraps selection with `*`...`*`. Unwraps if previously styled; inserts `**` when selection is collapsed.
  - **Strikethrough (`Ctrl+Shift+X` or `Ctrl+Alt+S`):** Wraps selection with `~~`...`~~`. Unwraps or inserts `~~~~`.
  - **Underline (`Ctrl+U` / `Cmd+U`):** Wraps selection with `<u>`...`</u>`.
- **Read-Only Mode:**
  - Toggle via `Ctrl+Shift+R` or settings switch.
  - Sets `editor.readOnly = true`, displays `#readonly-badge` lock indicator in top chrome bar, and disallows text modifying keystrokes while maintaining navigation and copy capability.

### 2. Sub-Pixel Vertical Caret Nudging Engine (`ui/src/typography.ts`, `ui/styles.css`, `ui/src/settings.ts`, `ui/settings.html`)
- **CSS Variables & Custom Font Alignment:**
  - Introduced `--caret-vertical-nudge`, `--caret-vertical-nudge-list`, and `--caret-vertical-nudge-applied-list` on `#editor-container`, `#editor`, `#editor-backdrop`, and `#editor-mirror`.
  - Padding top dynamically computes `calc(var(--pad-y) + var(--caret-vertical-nudge, 0px) + var(--caret-vertical-nudge-applied-list, 0px))` ensuring perfect baseline alignment across monospace fonts.
- **Dynamic List Line Detection (`CaretNudgeController`):**
  - Tracks editor selection, input, and click events.
  - Inspects active line: if cursor rests on a checklist or bullet/numbered list (`- [ ]`, `- [x]`, `* [ ]`, `[ ]`, `- `, `* `, `+ `, `1. `), applies `--caret-vertical-nudge-applied-list` to center the cursor with bullet points and checkbox boxes.
  - Clears extra list offset when cursor navigates back to ordinary body lines.
- **Settings Controls:**
  - Added range sliders and real-time pixel badges for `_caretVerticalNudgeRaw` and `_caretVerticalNudgeForListsRaw` (`-5.0px` to `+5.0px`, step `0.25px`) in `settings.html` and `settings.ts`.

### 3. Right-to-Left (RTL) Writing Direction & Mirrored Coordinates (`ui/src/main.ts`, `ui/styles.css`, `ui/src/settings.ts`, `ui/settings.html`)
- **Writing Direction (`forceRTL` / `_forceRTLStorage`):**
  - Hotkey toggle via `Ctrl+Shift+L` or settings switch.
  - Sets `dir="rtl"` on editor, mirror, backdrop, and editor-container, and applies `.rtl-mode` styling with `direction: rtl; text-align: right;` and mirrored padding (`padding-right: var(--pad-x); padding-left: 120px;`).
- **Mirrored Overlay Positioning:**
  - Updated `renderOverlay()` to dynamically compute horizontal offsets using `right: ${offset}px` (and clear `left`) when RTL is active.
  - Mirrored placement applied to math calculation chips, currency conversion chips, countdown/stopwatch timer chips, checklist checkboxes, and Antilink pills.

### 4. Rust Backend & Automated Test Suite (`crates/plainpad-core/src/typography.rs`, `crates/plainpad-core/tests/typography_rtl_tests.rs`)
- Implemented `typography.rs` module in `plainpad-core` supporting heading cycling, inline formatting, list line detection, `CaretNudgeConfig`, and `RtlConfig`.
- Added 5 comprehensive automated tests in `typography_rtl_tests.rs`:
  - `test_heading_cycle_transitions` (single line and multiline cursor tracking)
  - `test_inline_formatting_shortcuts` (bold, italic, strike, underline, empty, outer unwrap)
  - `test_list_line_detection` (all checklist, bullet, and numbered list variants)
  - `test_caret_nudge_config_and_active_calculation` (base + list nudge calculations and JSON serialization)
  - `test_rtl_config_and_persistence` (RTL configuration roundtrip)

## Verification Proof
- `cargo test -p plainpad-core --test typography_rtl_tests`: 5 passed; 0 failed; 0 warnings.
- `cargo test`: 138 passed across all 13 test suites project-wide (0 regressions).
- `npm run typecheck`: clean TypeScript check with zero errors.
- `npm run build:ui`: clean build, assets copied to `ui/dist`.
