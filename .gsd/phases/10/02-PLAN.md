---
phase: 10
plan: 2
wave: 2
gap_closure: false
depends_on:
  - 10.1
files_modified:
  - src-tauri/src/main.rs
  - ui/style.css
  - ui/styles.css
  - ui/src/break_view.ts
  - ui/src/timer.ts
  - ui/index.html
autonomous: true
must_haves:
  truths:
    - "Tauri window expands to borderless fullscreen always-on-top when a Pomodoro break starts"
    - "Exiting or finishing a break restores original window dimensions, coordinates, and always-on-top pin state"
    - "Atmospheric dark frosted glass backdrop (blur 48px, radial vignette, ambient magenta/violet lighting) immerses the entire display"
    - "Clicking 'Sneak a peek' unblurs the backdrop to reveal underlying desktop while immediately penalizing the streak (current streak resets to 0)"
    - "Clicking 'End Pomodoro' cancels the active Pomodoro session, closes the break screen, and restores the window"
    - "Completing the break without peeking triggers complete_break_streak (incrementing current streak in SQLite) and transitions to work phase with backtowork.m4a"
  artifacts:
    - src-tauri/src/main.rs
    - ui/src/break_view.ts
    - ui/src/timer.ts
    - ui/style.css
    - ui/styles.css
    - ui/index.html
  key_links:
    - "src-tauri/src/main.rs provides enter_break_fullscreen and exit_break_fullscreen with geometry backup"
    - "ui/src/break_view.ts coordinates sneak-a-peek unblur, penalty reset, countdown sync, and break termination"
    - "ui/src/timer.ts binds timer-phase-transition to enter_break_fullscreen and completeBreakStreak"
---

# Plan 10.2: Cross-Platform Fullscreen Takeover Window Management and "Sneak a Peek" Unblur Mechanics

<objective>
Implement cross-platform fullscreen window takeover management in Tauri and complete the interactive break lifecycle:
1. Tauri borderless fullscreen window management: Provide `enter_break_fullscreen` and `exit_break_fullscreen` IPC commands that smoothly expand the Tauri window to borderless fullscreen always-on-top across desktop platforms while safely preserving and restoring previous window geometry, bounds, and pin states.
2. Immersive frosted glass backdrop & "Sneak a peek" mechanics: Render dark atmospheric frosted glass (`backdrop-filter: blur(48px)`), radial vignette, and subtle ambient magenta/violet lighting. Implement temporary unblur/peek mechanics with SQLite streak penalty reset, "End Pomodoro" cancellation, and sound transition upon break completion.
</objective>

<context>
Load these files for context:
- .gsd/SPEC.md (§6.3)
- src-tauri/src/main.rs
- ui/src/break_view.ts
- ui/src/timer.ts
- ui/style.css
- ui/styles.css
- ui/index.html
</context>

<tasks>

<task type="auto">
  <name>Implement Tauri Fullscreen Takeover and Window State Restoration</name>
  <files>src-tauri/src/main.rs</files>
  <action>
    1. Define `WindowStateBackup` storing original `x`, `y`, `width`, `height`, and `is_always_on_top`.
    2. Add `window_backup: Option<WindowStateBackup>` to `AppState`.
    3. Implement `enter_break_fullscreen` capturing current geometry, caching in `AppState`, setting always-on-top to true, activating fullscreen, unminimizing, and focusing.
    4. Implement `exit_break_fullscreen` deactivating fullscreen, restoring original dimensions and position, and restoring original pin state.
    5. Register both commands in `tauri::generate_handler!`.
  </action>
  <verify>cargo check --manifest-path src-tauri/Cargo.toml</verify>
  <done>Fullscreen window commands compile and safely cache and restore window geometry.</done>
</task>

<task type="auto">
  <name>Implement Break View Module, Sneak a Peek Unblur, and Timer Lifecycle Wiring</name>
  <files>ui/src/break_view.ts, ui/src/timer.ts, ui/style.css, ui/styles.css, ui/index.html</files>
  <action>
    1. Create `ui/src/break_view.ts` managing:
       - Peeking state (`hasPeekedThisBreak`), penalty recording via `record_peek_penalty`, and peek banner display.
       - Return from peek via button or Escape key.
       - "End Pomodoro" aborting timer and calling `exit_break_fullscreen`.
       - Countdown synchronization and streak refresh.
    2. In `ui/style.css` and `ui/styles.css`:
       - Add frosted glass styling (`backdrop-filter: blur(48px) saturate(180%)`), ambient lighting, and `.is-peeking` temporary unblur transparency (`blur(3px)`, `opacity: 0.12`).
    3. In `ui/src/timer.ts`:
       - On `timer-phase-transition` to break: reset peek status, play `workdone.m4a`, invoke `enter_break_fullscreen`, and show break view.
       - On `timer-phase-transition` to work: if not peeked, invoke `complete_break_streak`; invoke `exit_break_fullscreen`, hide break view, and play `backtowork.m4a`.
       - On timer stop or cancel: invoke `exit_break_fullscreen` and hide break view.
  </action>
  <verify>npm run build:ui && npm run typecheck</verify>
  <done>Break view presentation, sneak-a-peek unblur mechanics, and timer lifecycle transitions compile and run cleanly.</done>
</task>

</tasks>
