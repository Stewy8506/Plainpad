---
phase: 6
plan: 2
wave: 2
gap_closure: false
depends_on:
  - "6.1"
files_modified:
  - ui/src/vim.ts
  - ui/src/main.ts
  - ui/styles.css
  - ui/src/settings.ts
  - ui/settings.html
  - ui/index.html
  - crates/plainpad-core/tests/vim_mode_tests.rs
autonomous: true
must_haves:
  truths:
    - "Vim mode provides 4 modal states: NORMAL, INSERT, VISUAL, and VISUAL LINE (enterVisualLineMode)"
    - "In NORMAL mode, motions h, j, k, l, w, b, e, 0, $, gg, G and arrow keys navigate cursor position"
    - "Rapid repetitive motions are handled through a debounced timer (debounceTimerForVimMotions) to prevent scroll jitter"
    - "NORMAL and VISUAL modes render a solid block caret at the active cursor position, switching to standard vertical line caret in INSERT mode"
    - "Pressing Esc or Ctrl+[ in any mode returns to NORMAL mode"
    - "Action operators d (delete), y (yank), p/P (paste), u (undo), x (delete char), c (change), r (replace char), i/a/o/O (insert entries) manipulate text and registers accurately"
    - "A persistent status line pill displays active mode, pending operator/count buffer, and line/column numbers"
    - "Vim mode can be enabled or disabled via preference toggle activateVimMode in Settings without breaking standard textarea behavior"
  artifacts:
    - "ui/src/vim.ts"
    - "crates/plainpad-core/tests/vim_mode_tests.rs"
  key_links:
    - "main.ts intercepts keydown events when activateVimMode is true and routes them through VimEngine in vim.ts"
    - "vim.ts manipulates editor textarea selections, registers, and undo stack"
    - "vim.ts updates the status line indicator in bottom-bar or top-bar"
---

# Plan 6.2: Modal Vim Mode Engine, Debounced Motions, Action Operators, and Status Line Indicator

<objective>
Implement a full-featured Modal Vim Mode engine (`Normal`, `Insert`, `Visual`, `Visual Line`) in the Plainpad editor with debounced motion handling (`h,j,k,l,w,b,e,0,$,gg,G`), action operators (`d,y,p,u,x,c,r,i,a,o,O`), yank registers, undo integration, block caret rendering, bottom status line indicator, and toggle setting `activateVimMode`.

Purpose: Antinote provides a modal Vim experience for keyboard-centric note-taking, enabling fast navigation, manipulation, and line selections directly within notes without touching the mouse.
Output: `ui/src/vim.ts`, updated `main.ts`, block caret styles, status line indicator in `index.html`, settings toggle in `settings.html`, and automated test suite `vim_mode_tests.rs`.
</objective>

<context>
Load these files for context:
- .gsd/SPEC.md (§7 Editor Capabilities & Rich Inline Elements, §11 Preferences)
- .gsd/findings.md (§15 Editor Controls & Typography Tweaks, §18 Checklist)
- .gsd/REQUIREMENTS.md (REQ-22)
- ui/src/main.ts
- ui/styles.css
- ui/index.html
- ui/src/settings.ts
- ui/settings.html
</context>

<tasks>

<task type="auto">
  <name>Implement modal state machine, debounced motions, and block caret rendering</name>
  <files>
    ui/src/vim.ts
    ui/src/main.ts
    ui/styles.css
  </files>
  <action>
    Create `ui/src/vim.ts` and integrate with `main.ts`:
    1. Define `VimMode` enum: `NORMAL`, `INSERT`, `VISUAL`, `VISUAL_LINE`.
    2. Implement `VimEngine` class:
       - State: `mode`, `commandBuffer` (for multi-key commands and count prefixes like `3j`, `gg`, `dw`), `yankRegister`, `anchorPos`, `cursorPos`.
       - Debounced motion timer (`debounceTimerForVimMotions`): when rapid motion keys (e.g. holding `j` or `k`) fire, debounce overlay re-rendering and coordinate recalculation so scrolling and cursor movement remain butter-smooth at 60fps.
    3. Motion navigation:
       - `h` (left), `l` (right), `j` (down one visual/logical line), `k` (up one line).
       - Arrow keys (`ArrowLeft`, `ArrowRight`, `ArrowDown`, `ArrowUp`) mirror `h, l, j, k`.
       - `w`: advance to beginning of next word.
       - `b`: retreat to beginning of previous word.
       - `e`: advance to end of current/next word.
       - `0`: jump to start of line.
       - `^`: jump to first non-whitespace character of line.
       - `$`: jump to end of line.
       - `gg`: jump to top of document (line 1).
       - `G`: jump to bottom of document (or specific line if numeric count provided).
    4. Block Caret:
       - In `NORMAL`, `VISUAL`, and `VISUAL_LINE` modes, render a dedicated `.vim-block-caret` element overlaying the character under cursor (or set textarea class `.vim-normal-mode` with block cursor styling).
       - In `INSERT` mode, hide the block caret and restore default thin vertical text cursor.
       - `Esc` or `Ctrl+[`: transition immediately to `NORMAL` mode from any other mode, moving cursor 1 position left (standard Vim behavior).
    5. In `ui/src/main.ts`, hook `vimEngine.handleKeyDown(e)` in the editor's keydown listener when Vim mode is active.
    
    AVOID: Capturing typing keys during `INSERT` mode.
    USE: In `INSERT` mode, pass through all keys except `Esc` or `Ctrl+[` directly to the browser.
  </action>
  <verify>
    npm run typecheck && npm run build:ui
  </verify>
  <done>
    VimEngine handles modal transitions between NORMAL and INSERT, executes motions (h,j,k,l,w,b,e,0,$,gg,G), debounces rapid motion updates, and renders block caret in NORMAL mode.
  </done>
</task>

<task type="auto">
  <name>Implement action operators, register clipboard, and visual selection modes</name>
  <files>
    ui/src/vim.ts
    ui/src/main.ts
  </files>
  <action>
    In `ui/src/vim.ts`, implement action operators, registers, and visual modes:
    1. Entry Actions:
       - `i`: enter INSERT mode at cursor position.
       - `a`: advance cursor by 1 character, then enter INSERT mode.
       - `o`: insert newline below current line, advance cursor to it, and enter INSERT mode.
       - `O`: insert newline above current line, move cursor to it, and enter INSERT mode.
    2. Manipulation Operators:
       - `x`: delete character under cursor into `yankRegister` and maintain NORMAL mode.
       - `r{char}`: wait for next keypress, replace character under cursor with `{char}`, and stay in NORMAL mode.
       - `d` (Delete) operator:
         - `dd`: delete entire current line into `yankRegister`, pull lower lines up.
         - `dw`: delete from cursor to start of next word into `yankRegister`.
         - `d$` / `D`: delete from cursor to end of line into `yankRegister`.
         - In VISUAL or VISUAL LINE mode: `d` deletes the visual selection span into `yankRegister` and returns to NORMAL mode.
       - `y` (Yank) operator:
         - `yy`: copy current line to `yankRegister`.
         - `yw`: copy word to `yankRegister`.
         - In VISUAL or VISUAL LINE mode: `y` copies selected range to `yankRegister` and returns to NORMAL mode.
       - `p` / `P` (Paste/Put):
         - `p`: paste contents of `yankRegister` after cursor (or on new line below if line-yanked).
         - `P`: paste contents of `yankRegister` before cursor (or on new line above if line-yanked).
       - `c` (Change) operator:
         - `cc`: clear line and enter INSERT mode.
         - `cw`: delete word and enter INSERT mode.
         - `C`: delete to end of line and enter INSERT mode.
         - In VISUAL mode: `c` deletes selected range and enters INSERT mode.
       - `u`: trigger undo on editor document.
    3. Visual Selection Modes:
       - `v`: enter VISUAL mode. Set `anchorPos`. As cursor moves via motions, highlight the range between `anchorPos` and `cursorPos` using textarea selection or overlay span.
       - `V` / `enterVisualLineMode`: enter VISUAL LINE mode. Select entire lines from `anchorPos` line to `cursorPos` line.
       - Pressing `v` in VISUAL mode or `V` in VISUAL LINE mode or `Esc` exits back to NORMAL mode.
  </action>
  <verify>
    npm run typecheck && npm run build:ui
  </verify>
  <done>
    Action operators (d, y, p, P, u, x, c, r, i, a, o, O) correctly modify text and registers; Visual and Visual Line selection modes allow targeted span operations.
  </done>
</task>

<task type="auto">
  <name>Implement Vim status line indicator, settings toggle, and automated test suite</name>
  <files>
    ui/src/vim.ts
    ui/src/main.ts
    ui/src/settings.ts
    ui/settings.html
    ui/index.html
    ui/styles.css
    crates/plainpad-core/tests/vim_mode_tests.rs
  </files>
  <action>
    Complete UI integration, settings wiring, and testing:
    1. **Status Line Indicator:**
       - In `ui/index.html`, add `#vim-status-indicator` element in `#bottom-bar` (or next to note index).
       - Style indicator in `ui/styles.css`: mono font, pill badge, discreet background `rgba(255,255,255,0.08)`.
       - Display format:
         - Mode badge: `-- NORMAL --`, `-- INSERT --`, `-- VISUAL --`, `-- VISUAL LINE --`.
         - Pending keys: show buffer when count or operator active (e.g. `2d`, `g`, `r`).
         - Coordinates: `Ln 14, Col 6`.
    2. **Settings Integration (`activateVimMode`):**
       - In `settings.html` (under Editor section), add toggle `activateVimMode` with label "Vim Modal Keybindings".
       - In `settings.ts`, bind `#setting-activate-vim-mode` to load and save `activateVimMode` preference (default `false`).
       - In `main.ts`, listen for `activateVimMode` changes:
         - When disabled: hide `#vim-status-indicator`, remove block caret, restore standard textarea behavior.
         - When enabled: show indicator in NORMAL mode, activate block caret and key interception.
    3. **Automated Test Suite:**
       - Create `crates/plainpad-core/tests/vim_mode_tests.rs` verifying Vim motion math, line and word boundary logic, delete/yank register storage, and preference persistence.
  </action>
  <verify>
    cargo test -p plainpad-core --test vim_mode_tests && npm run typecheck
  </verify>
  <done>
    Vim status line indicator displays current mode, buffer, and coordinates; activateVimMode preference enables/disables Vim mode; automated tests pass.
  </done>
</task>

</tasks>

## Must-Haves
After all tasks complete, verify:
- [ ] Vim mode switches accurately between NORMAL, INSERT, VISUAL, and VISUAL LINE modes
- [ ] Standard motions (h, j, k, l, w, b, e, 0, $, gg, G) and arrows move the cursor correctly
- [ ] Motions are debounced via debounceTimerForVimMotions to avoid scrolling jitter
- [ ] Block caret displays in NORMAL and VISUAL modes, reverting to vertical line cursor in INSERT mode
- [ ] Operators d, y, p, u, x, c, r, i, a, o, O perform expected text edits and register updates
- [ ] Status line shows active mode, pending keys, and cursor position
- [ ] Toggle setting activateVimMode enables and disables modal keybindings cleanly

## Success Criteria
- [ ] All tasks verified passing
- [ ] `npm run typecheck` and `npm run build:ui` pass with zero errors
- [ ] `cargo test -p plainpad-core --test vim_mode_tests` passes with 100% green tests
- [ ] No regressions in existing tests
