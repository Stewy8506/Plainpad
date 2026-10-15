---
phase: 5
plan: 3
wave: 3
gap_closure: true
depends_on:
  - "01"
  - "02"
files_modified:
  - src-tauri/src/main.rs
  - ui/src/timer.ts
  - ui/src/main.ts
  - crates/plainpad-core/tests/timer_engine_tests.rs
autonomous: true
must_haves:
  truths:
    - "Pausing or resuming a timer updates the Timer Pill immediately with paused state, yellow dashed stroke, and resume button"
    - "Cancelling or stopping a timer from the editor (timer s or chip click) immediately clears the Timer Pill from the UI"
    - "Active timers pause automatically when Plainpad window closes or hides if pauseTheTimerIfAppClosed is enabled"
  artifacts:
    - "src-tauri/src/main.rs"
    - "ui/src/timer.ts"
    - "ui/src/main.ts"
    - "crates/plainpad-core/tests/timer_engine_tests.rs"
  key_links:
    - "pause_resume_timer and pause_timer in main.rs emit timer-tick event with updated timer state"
    - "cancel_timer in main.rs emits timer-stopped event to clear frontend display"
    - "hide_window and window CloseRequested query pauseTheTimerIfAppClosed and invoke timers.pause_active()"
---

# Plan 5.3 (Gap Closure): Timer Reactivity on Pause/Stop and Window Hide/Close Pause Integration

<objective>
Close the 3 gaps identified during Phase 5 verification:
1. Real-time UI synchronization on pause/resume: ensure `pause_timer` and `pause_resume_timer` emit an immediate `"timer-tick"` event so the Timer Pill updates to its paused state (`is-paused` class, `▶` button, yellow dashed ring), and update `ui/src/timer.ts` to consume IPC return values.
2. Immediate dismiss on timer cancellation: ensure `cancel_timer` emits a `"timer-stopped"` event and that `ui/src/timer.ts` / `ui/src/main.ts` clears the Timer Pill immediately when stopped from editor phrases (`timer s`) or chips.
3. `pauseTheTimerIfAppClosed` execution (SPEC §6.2 & §11): wire `timers.pause_active()` into `hide_window`, tray menu "hide", and the main window `CloseRequested` event handler when `pauseTheTimerIfAppClosed` setting is true.
</objective>

<context>
Load these files for context:
- .gsd/SPEC.md (§6.2 Visuals & Audio Signals, §11 Preferences)
- crates/plainpad-core/src/timers.rs
- src-tauri/src/main.rs
- ui/src/timer.ts
- ui/src/main.ts
</context>

<tasks>

<task type="auto">
  <name>Wire pause and cancel event emission in main.rs and handle them reactively in timer.ts and main.ts</name>
  <files>
    src-tauri/src/main.rs
    ui/src/timer.ts
    ui/src/main.ts
  </files>
  <action>
    1. In `src-tauri/src/main.rs`:
       - Update `pause_timer` and `pause_resume_timer` to accept `app: AppHandle`:
         - After toggling pause via `state.lock().unwrap().timers.pause_resume(id)`, emit `"timer-tick"` with the updated `Timer` so the UI receives the paused state immediately.
       - Update `restart_timer` to accept `app: AppHandle`:
         - After restart, emit `"timer-tick"` with the restarted `Timer`.
       - Update `cancel_timer` to accept `app: AppHandle`:
         - After removing timer via `state.lock().unwrap().timers.stop(id)`, emit `"timer-stopped"`, serde_json::json!({ "id": id }).
    2. In `ui/src/timer.ts`:
       - In `initTimerUI()`:
         - When clicking the pill or pause button, handle the returned promise:
           ```ts
           const updated = await invoke<Timer | null>('pause_resume_timer');
           if (updated) renderTimer(updated);
           ```
         - When clicking the restart button:
           ```ts
           const updated = await invoke<Timer | null>('restart_timer');
           if (updated) renderTimer(updated);
           ```
         - Listen for `'timer-stopped'`:
           ```ts
           await listen('timer-stopped', () => {
             renderTimer(null);
           });
           ```
       - Export `clearTimer()` helper that calls `renderTimer(null)`.
    3. In `ui/src/main.ts`:
       - When clicking timer stop chip or pressing Enter on `timer s` / `stop timer`:
         - Call `clearTimer()` from `timer.ts` so the timer pill is cleared without delay.
  </action>
  <verify>
    npm run build:ui
  </verify>
  <done>
    `npm run build:ui` succeeds without errors, pause/resume updates the UI immediately, and timer cancel clears the UI pill.
  </done>
</task>

<task type="auto">
  <name>Implement pauseTheTimerIfAppClosed on window hide, close, and tray events</name>
  <files>
    src-tauri/src/main.rs
    crates/plainpad-core/tests/timer_engine_tests.rs
  </files>
  <action>
    1. In `src-tauri/src/main.rs`:
       - Create helper function:
         ```rust
         fn pause_timer_if_configured(app: &AppHandle) {
             let state = app.state::<Mutex<AppState>>();
             let s = state.lock().unwrap();
             let should_pause = s.store.get_setting("pauseTheTimerIfAppClosed")
                 .and_then(|v| v.as_bool())
                 .unwrap_or(true);
             if should_pause {
                 if let Some(paused) = s.timers.pause_active() {
                     let _ = app.emit("timer-tick", paused);
                 }
             }
         }
         ```
       - Call `pause_timer_if_configured(&app)` in:
         - `hide_window`: before hiding window.
         - Tray menu "hide" event.
         - Global hotkey hide event.
         - `on_window_event`: when `WindowEvent::CloseRequested` occurs on `"main"`.
    2. In `crates/plainpad-core/tests/timer_engine_tests.rs`:
       - Add unit test `test_pause_active_idempotent`:
         - Verify `engine.pause_active()` pauses a running countdown/stopwatch, and returns unchanged if already paused.
  </action>
  <verify>
    /home/anv/.cargo/bin/cargo test --test timer_engine_tests
    /home/anv/.cargo/bin/cargo check --manifest-path src-tauri/Cargo.toml
  </verify>
  <done>
    All timer engine tests pass and `cargo check --manifest-path src-tauri/Cargo.toml` compiles with 0 errors.
  </done>
</task>

</tasks>

<must_haves>
After tasks complete, verify:
- [ ] Pausing a timer immediately renders the paused UI styling (yellow dashed circle, pause/resume button toggles to ▶)
- [ ] `timer s` in editor or stop button immediately dismisses the Timer Pill
- [ ] Window hide and close pause running timers when `pauseTheTimerIfAppClosed` is true
</must_haves>

<success_criteria>
- [ ] All 2 tasks verified passing
- [ ] Zero TypeScript errors
- [ ] Zero Rust compilation errors
</success_criteria>
