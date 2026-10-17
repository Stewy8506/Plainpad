---
phase: 9
plan: 1
wave: 1
gap_closure: false
depends_on: []
files_modified:
  - crates/plainpad-core/src/intent.rs
  - crates/plainpad-core/tests/trigger_evaluator_tests.rs
  - ui/src/main.ts
  - ui/styles.css
  - ui/style.css
autonomous: true
must_haves:
  truths:
    - "Typing a bare standalone number (e.g. '20', '1234.5') does NOT produce a trivial identity math annotation ('= 20')"
    - "Expressions with operations, functions, variables, units, or currency conversions ('20 + 5', '20 inr to usd') continue to evaluate accurately"
    - "Typing ' =' or pressing 'Tab' after a calculated expression steps into or cleanly adopts the ghost answer without duplicate equals signs ('= =')"
    - "Dragging across lines with active calculation results visually highlights the ghost answers with the selection styling"
    - "Copying selected text containing active calculations captures the full expression with the answer ('20 inr to usd = $0.21') into the clipboard"
  artifacts:
    - crates/plainpad-core/src/intent.rs
    - crates/plainpad-core/tests/trigger_evaluator_tests.rs
    - ui/src/main.ts
    - ui/styles.css
  key_links:
    - "crates/plainpad-core/src/intent.rs filters out bare numeric literals and trivial identity evaluations where input equals output"
    - "ui/src/main.ts incorporates hasTrailingEquals into annoKey so changing trailing equals state immediately updates prefix without duplicate equals"
    - "ui/src/main.ts selection and copy event listeners synchronize selection highlight and inject autocompleted values into the clipboardData buffer"
---

# Plan 9.1: Trivial Identity Suppression, Ghost Equals Absorption, and Rich Clipboard Copy Interceptor

<objective>
Refine the mathematical and conversion interaction model in Plainpad:
1. Suppress trivial identity calculations: Typing a standalone number (e.g. `20`, `100`, `0.5`) must not show `= 20`. Calculations should trigger only when an operator, function, conversion domain, variable, or explicit arithmetic relationship exists.
2. Ghost answer adoption & duplicate equals fix: Eliminate the duplicate equals bug (`= = $0.21`) by incorporating trailing equals state into the overlay cache key, and allow pressing `Tab` or typing ` =` to step into the answer cleanly.
3. Visual selection & rich clipboard copy: Synchronize selection highlighting so dragging across an expression highlights its ghost answer, and intercept the `copy` event so copying lines with active calculations copies `expression = answer` (e.g. `20 inr to usd = $0.21`).
</objective>

<context>
Load these files for context:
- crates/plainpad-core/src/intent.rs
- crates/plainpad-core/tests/trigger_evaluator_tests.rs
- ui/src/main.ts
- ui/styles.css
</context>

<tasks>

<task type="auto">
  <name>Suppress Trivial Identity Numbers in Intent Classifier</name>
  <files>crates/plainpad-core/src/intent.rs, crates/plainpad-core/tests/trigger_evaluator_tests.rs</files>
  <action>
    1. In `crates/plainpad-core/src/intent.rs` within `classify_math()`:
       - Inspect `t.trim()`. If trimming optional trailing punctuation (`=`, `?`) leaves a string that is purely a standalone numeric literal (e.g., matching all digits, optional leading sign, commas, and decimal point) without any arithmetic operator (`+`, `*`, `/`, `^`, `%`), function call, unit, or variable reference, return `None`.
       - Check if the evaluated result is an identity no-op where `out.result.trim() == t.trim()`, returning `None`.
       - Ensure genuine calculations (`20 + 5`, `20 inr to usd`, `5 km in miles`, `sqrt(25)`, `x = 10`, `sum`) still evaluate with full fidelity.
    2. Add unit tests in `crates/plainpad-core/tests/trigger_evaluator_tests.rs`:
       - Verify `20`, `1234.5`, `-5`, and `20 =` produce NO annotations in both normal and forced math mode.
       - Verify `20 inr to usd`, `20 + 5`, `5 km in miles` produce annotations with expected results.
       - Verify block `sum` and `avg` continuing to parse bare numbers from preceding lines without disruption.
  </action>
  <verify>PATH=$PATH:/home/anv/.cargo/bin cargo test -p plainpad-core</verify>
  <done>All core tests pass; standalone numbers emit zero annotations while expressions evaluate properly.</done>
</task>

<task type="auto">
  <name>Fix Overlay Equals Cache Key and Ghost Equals Absorption</name>
  <files>ui/src/main.ts</files>
  <action>
    1. In `ui/src/main.ts` inside `renderOverlay()`:
       - Update the annotation cache key to include `hasTrailingEquals`:
         `const annoKey = `${anno.line}:${anno.kind}:${hasTrailingEquals ? 'eq' : 'noeq'}``;
       - This guarantees that when the user types ` =` or deletes `=`, the overlay cache detects the prefix change and re-renders the element immediately without displaying duplicate `= =`.
    2. In `ui/src/main.ts` editor `keydown` handler:
       - When the user presses `Tab` while the caret is on a line with an active calculation annotation (`math` or `conversion` in `lastAnalysis.annotations`), if the line doesn't end with `=`, append ` = ` and reposition the cursor cleanly instead of inserting a tab character.
       - When the user clicks on an active math or conversion annotation in the overlay, copy the answer (or insert ` = <value>`) and provide immediate visual confirmation.
  </action>
  <verify>npm run build:ui && npm run typecheck</verify>
  <done>Typing ' =' drops ghost '= ' instantly; Tab on expression lines adopts equals cleanly without duplicate tokens.</done>
</task>

<task type="auto">
  <name>Implement Visual Selection Sync and Rich Clipboard Copy Interceptor</name>
  <files>ui/src/main.ts, ui/styles.css, ui/style.css</files>
  <action>
    1. In `ui/styles.css` and `ui/style.css`:
       - Add `.anno.is-selected` styling matching the text selection highlight:
         `background: var(--selection-bg, rgba(194, 132, 122, 0.35)); border-radius: 3px;`
    2. In `ui/src/main.ts`:
       - Add a selection change handler (`syncOverlaySelection`) listening to `selectionchange`, `select`, `mouseup`, and `keyup` on `#editor`.
       - When text is selected, determine the line range `[startLine, endLine]`. For any line with an active calculation in `lastAnalysis.annotations`, if the selection spans the end of that line's text, add `.is-selected` to the corresponding overlay element.
       - Add a `copy` event listener on `#editor`:
         - If text is selected and the selection includes lines with active calculations:
           - For each line in the selection, if the selection extends to the end of the line's typed content and has an active calculation annotation, append the answer (` = ${anno.value}` or ` ${anno.value}` if trailing equals already present).
           - Write the synthesized text to `e.clipboardData.setData('text/plain', enhancedText)` and `e.preventDefault()`.
  </action>
  <verify>npm run build:ui && npm run typecheck</verify>
  <done>Dragging over calculation lines visually highlights answers; pressing Ctrl+C copies expression + answer into the clipboard.</done>
</task>

</tasks>
