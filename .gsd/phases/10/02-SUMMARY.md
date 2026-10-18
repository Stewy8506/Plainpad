# Phase 10 Plan 2: Cross-Platform Fullscreen Takeover Window Management and "Sneak a Peek" Unblur Mechanics Summary

## Objective
Implement cross-platform fullscreen window takeover management in Tauri and complete the interactive break lifecycle:
1. Tauri borderless fullscreen window management: Provide `enter_break_fullscreen` and `exit_break_fullscreen` IPC commands that smoothly expand the Tauri window to borderless fullscreen always-on-top across desktop platforms while safely preserving and restoring previous window geometry, bounds, and pin states.
2. Immersive frosted glass backdrop & "Sneak a peek" mechanics: Render dark atmospheric frosted glass (`backdrop-filter: blur(48px)`), radial vignette, and subtle ambient magenta/violet lighting. Implement temporary unblur/peek mechanics with SQLite streak penalty reset, "End Pomodoro" cancellation, and sound transition upon break completion.

## Changes Completed

### 1. Tauri Backend Window Management (`src-tauri/src/main.rs`)
- **`WindowStateBackup` Struct & AppState Integration**:
  - Defined `WindowStateBackup` storing original `x: i32`, `y: i32`, `width: f64`, `height: f64`, and `is_always_on_top: bool`.
  - Added `window_backup: Option<WindowStateBackup>` to `AppState` under `Mutex`.
- **`enter_break_fullscreen` Command**:
  - Captures current window size (`inner_size()`), position (`outer_position()`), and scale factor, with graceful fallback to persisted SQLite geometry settings.
  - Caches geometry and pin state in `s.window_backup`.
  - Sets window always-on-top (`window.set_always_on_top(true)`), activates fullscreen (`window.set_fullscreen(true)`), ensures window is focused, visible, and unminimized.
- **`exit_break_fullscreen` Command**:
  - Disables fullscreen (`window.set_fullscreen(false)`).
  - Restores original window dimensions (`window.set_size`), position (`window.set_position`), and pin status (`window.set_always_on_top`), clearing the cached backup.
  - Falls back to SQLite settings if no backup was cached.
- **Command Handler Registration**:
  - Registered `enter_break_fullscreen` and `exit_break_fullscreen` in `tauri::generate_handler!`.

### 2. Frosted Glass Immersion & Sneak a Peek Styling (`ui/style.css`, `ui/styles.css`)
- **Immersive Fullscreen Takeover**:
  - Styled `.break-time-overlay` and `.break-time-view` with `position: fixed; inset: 0; z-index: 99999; backdrop-filter: blur(48px) saturate(180%);` and dark radial atmospheric gradients.
  - Added ambient magenta/violet radial lighting behind the orbit illustration using `.break-graphic-container::before` (`box-shadow: 0 0 120px 30px rgba(168, 85, 247, 0.18)`).
- **Sneak a Peek Mode**:
  - Implemented `.break-time-view.is-peeking` with backdrop transparency (`opacity: 0.12`, `backdrop-filter: blur(3px)`).
  - Floating top-center banner `.peek-banner` with red alert badge styling (`background: rgba(15, 23, 42, 0.94); border: 1px solid rgba(239, 68, 68, 0.5); color: #fca5a5;`).

### 3. Break View Presentation Module (`ui/src/break_view.ts`)
- Created `ui/src/break_view.ts` modularizing all Break Time View behaviors:
  - Tracks `hasPeekedThisBreak` state with `getHasPeeked()` and `resetPeekStatus()`.
  - Sneak a peek handler: invokes `record_peek_penalty` to reset SQLite streak to 0, updates live streak display, sets `is-peeking`, and reveals `#peek-banner`.
  - Return to break: handles `#peek-return-btn` click and global `Escape` keypress to exit peek mode.
  - End Pomodoro handler: invokes `cancel_timer` and `exit_break_fullscreen`, and hides break view.
  - Break countdown sync: `updateBreakCountdown(remainingMs, isPaused)` updates live digits and pause/resume button status.

### 4. Timer Lifecycle Integration (`ui/src/timer.ts`, `ui/index.html`)
- Updated `ui/index.html` with `id="break-time-view"`.
- Wired `initBreakView` into `initTimerUI`.
- Synchronized `timer-phase-transition`:
  - When transitioning to break (`!p.is_work_phase`): resets peek status, plays `workdone.m4a`, invokes `enter_break_fullscreen`, and presents the Break Time View with live countdown and random fact.
  - When transitioning to work (`p.is_work_phase`): if not peeked, calls `completeBreakStreak()`, invokes `exit_break_fullscreen`, hides break view, and plays `backtowork.m4a`.
- Forwarded `timer-tick` countdown updates to `updateBreakCountdown()`.
- Synchronized all timer cancellation pathways (`timer-stopped`, stop pill button, `clearTimer`) to invoke `exit_break_fullscreen` and hide the break view.

## Verification Evidence
1. **Backend Cargo Check**:
   - `cargo check --manifest-path src-tauri/Cargo.toml`: Code 0, finished in 1.20s without warnings.
2. **Frontend TypeScript & Asset Compilation**:
   - `npm run typecheck`: Code 0, clean output.
   - `npm run build:ui`: Code 0, compiled TypeScript and copied all assets (`backtowork.m4a`, `workdone.m4a`, `countdowndone.m4a`, HTML, CSS) to `ui/dist`.
3. **Core Test Suites**:
   - `cargo test --test streak_tests`: 2/2 passed (`test_break_facts_catalog`, `test_no_peek_streak_lifecycle`).
   - `cargo test -p plainpad-core`: 17/17 test suites (95+ tests) passed cleanly.
4. **Git Status Check**:
   - Changes ready for phase commit (per user instruction, git commit deferred to phase completion).

## Must-Haves Verification
- [x] Tauri window expands to borderless fullscreen always-on-top when a Pomodoro break starts.
- [x] Exiting or finishing a break restores original window dimensions, coordinates, and always-on-top pin state.
- [x] Atmospheric dark frosted glass backdrop (blur 48px, radial vignette, ambient magenta/violet lighting) immerses the entire display.
- [x] Clicking 'Sneak a peek' unblurs the backdrop to reveal underlying desktop while immediately penalizing the streak (current streak resets to 0).
- [x] Clicking 'End Pomodoro' cancels the active Pomodoro session, closes the break screen, and restores the window.
- [x] Completing the break without peeking triggers complete_break_streak (incrementing current streak in SQLite) and transitions to work phase with backtowork.m4a.
