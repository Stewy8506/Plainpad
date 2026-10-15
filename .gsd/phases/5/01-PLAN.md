---
phase: 5
plan: 1
wave: 1
depends_on: []
files_modified:
  - crates/plainpad-core/src/timers.rs
  - crates/plainpad-core/src/intent.rs
  - src-tauri/src/main.rs
  - crates/plainpad-core/tests/timer_engine_tests.rs
autonomous: true
gap_closure: false
must_haves:
  truths:
    - "`timer` keyword without arguments creates and starts a stopwatch counting up from 00:00"
    - "`timer 5` creates a 5-minute countdown and `timer 3:30` creates a 3-minute 30-second countdown"
    - "`timer pomo` creates a classic Pomodoro (25m work, 5m break)"
    - "`timer 57 11` creates a custom Pomodoro (57m work, 11m break)"
    - "`timer p` pauses or resumes the active timer or pomodoro"
    - "`timer r` restarts the active timer or pomodoro from its initial duration"
    - "`timer s` stops and dismisses the active timer"
    - "Pomodoro state machine transitions from work phase to break phase with workdone signal, and from break phase to work phase with backtowork signal while incrementing cycle count"
    - "Timers persist to JSON storage across app reload and window close"
  artifacts:
    - "crates/plainpad-core/src/timers.rs"
    - "crates/plainpad-core/src/intent.rs"
    - "src-tauri/src/main.rs"
    - "crates/plainpad-core/tests/timer_engine_tests.rs"
  key_links:
    - "intent.rs dispatches timer syntax variations through timers::parse_timer_phrase"
    - "TimerEngine provides tick loop, Pomodoro phase transitions, and JSON file persistence"
    - "src-tauri/src/main.rs exposes timer commands and emits Tauri events (timer-tick, timer-fired, timer-phase-transition)"
---

# Plan 5.1: Timer Backend Engine, Pomodoro State Machine, Syntax Parsing, and IPC

<objective>
Implement the backend timer and Pomodoro engine in `plainpad-core` supporting all syntax variations (`timer` stopwatch counting up from 00:00, `timer 5` countdown, `timer 3:30` mm:ss countdown, `timer pomo` 25m/5m, `timer 57 11` custom Pomodoro, `timer p` pause/resume, `timer r` restart, `timer s` stop), Pomodoro state transitions (work -> break -> work with cycle count), background tick thread, disk persistence (`state/timers.json`), and Tauri IPC commands with real-time event broadcasting.

Purpose: Antinote allows starting timers, stopwatches, and Pomodoros directly from plain text in notes, keeping track of work/break cycles and persisting them reliably across restarts.
Output: Enhanced `timers.rs`, updated `intent.rs` trigger annotations, Tauri IPC commands in `main.rs`, and integration test suite `timer_engine_tests.rs`.
</objective>

<context>
Load these files for context:
- .gsd/SPEC.md (§6 Timer & Pomodoro Engine, §11 Preferences)
- .gsd/findings.md (§11 Timer, Stopwatch & Pomodoro Subsystem)
- .gsd/REQUIREMENTS.md (REQ-15, REQ-16, REQ-17)
- crates/plainpad-core/src/timers.rs
- crates/plainpad-core/src/intent.rs
- src-tauri/src/main.rs
</context>

<tasks>

<task type="auto">
  <name>Implement comprehensive timer, stopwatch, and pomodoro syntax parser and intent annotations</name>
  <files>
    crates/plainpad-core/src/timers.rs
    crates/plainpad-core/src/intent.rs
  </files>
  <action>
    1. In `crates/plainpad-core/src/timers.rs`:
       - Define `TimerType` enum:
         ```rust
         #[derive(serde::Serialize, serde::Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
         pub enum TimerType {
             Stopwatch,
             Countdown,
             Pomodoro,
         }
         ```
       - Define `TimerControlAction` enum:
         ```rust
         #[derive(Debug, Clone, Copy, PartialEq, Eq)]
         pub enum TimerControlAction {
             PauseResume,
             Restart,
             Stop,
         }
         ```
       - Define `TimerPhrase` enum:
         ```rust
         #[derive(Debug, Clone, PartialEq)]
         pub enum TimerPhrase {
             StartStopwatch,
             StartCountdown { duration_ms: u64, name: String },
             StartPomodoro { work_ms: u64, break_ms: u64, name: String },
             Control(TimerControlAction),
         }
         ```
       - Implement `parse_timer_phrase(line: &str) -> Option<TimerPhrase>`:
         - Normalize line: trim, case-insensitive.
         - Check control keywords first:
           - `timer p`, `timer pause`, `timer resume` -> `TimerPhrase::Control(TimerControlAction::PauseResume)`
           - `timer r`, `timer restart`, `timer reset` -> `TimerPhrase::Control(TimerControlAction::Restart)`
           - `timer s`, `timer stop`, `timer cancel`, `stop timer`, `cancel timer` -> `TimerPhrase::Control(TimerControlAction::Stop)`
         - Check bare trigger:
           - `timer` -> `TimerPhrase::StartStopwatch` (Stopwatch counting up from 00:00)
         - Check Pomodoro triggers:
           - `timer pomo`, `timer pomodoro` -> `TimerPhrase::StartPomodoro { work_ms: 25 * 60_000, break_ms: 5 * 60_000, name: "Pomodoro".into() }`
           - Two integer arguments (e.g. `timer 57 11`): parse as `<work_minutes> <break_minutes>` -> `TimerPhrase::StartPomodoro { work_ms: work * 60_000, break_ms: break * 60_000, name: "Pomodoro".into() }`
         - Check countdown triggers:
           - `timer 5` -> 5 minutes countdown (`duration_ms: 5 * 60_000`)
           - `timer 3:30` -> mm:ss countdown (`duration_ms: 210_000`)
           - `timer 1h 30m pasta` -> duration parser with name extraction
           - `remind me in 10 minutes` -> duration parser
       - Retain backwards-compatible `is_timer_stop_phrase(line: &str) -> bool`.
    2. In `crates/plainpad-core/src/intent.rs`:
       - Update `TriggerCategory::Timer` handling:
         - Call `crate::timers::parse_timer_phrase(trimmed_line)`.
         - If `Some(TimerPhrase::StartStopwatch)`:
           - Emit `Annotation { line: i, kind: "timer", confidence: 0.95, value: "Stopwatch".into(), chip: "Start Stopwatch (Enter)".into(), why: "timer trigger keyword".into() }`.
         - If `Some(TimerPhrase::StartCountdown { duration_ms, .. })`:
           - Emit `Annotation { line: i, kind: "timer", confidence: 0.95, value: format_duration(duration_ms), chip: format!("Start timer ({}) (Enter)", format_duration(duration_ms)), why: "countdown timer trigger".into() }`.
         - If `Some(TimerPhrase::StartPomodoro { work_ms, break_ms, .. })`:
           - Emit `Annotation { line: i, kind: "timer", confidence: 0.95, value: format!("Pomodoro ({}m/{}m)", work_ms / 60_000, break_ms / 60_000), chip: "Start Pomodoro (Enter)".into(), why: "pomodoro trigger".into() }`.
         - If `Some(TimerPhrase::Control(TimerControlAction::PauseResume))`:
           - Emit `Annotation { line: i, kind: "timer_control", confidence: 0.95, value: "pause".into(), chip: "Pause / Resume timer (Enter)".into(), why: "phrase to pause or resume active timer".into() }`.
         - If `Some(TimerPhrase::Control(TimerControlAction::Restart))`:
           - Emit `Annotation { line: i, kind: "timer_control", confidence: 0.95, value: "restart".into(), chip: "Restart timer (Enter)".into(), why: "phrase to restart active timer".into() }`.
         - If `Some(TimerPhrase::Control(TimerControlAction::Stop))`:
           - Emit `Annotation { line: i, kind: "timer_stop", confidence: 0.95, value: "stop".into(), chip: "Stop timer (Enter)".into(), why: "phrase to stop running timer".into() }`.

    AVOID: Returning `None` for bare `timer` string (SPEC §6.1 requires it to start a stopwatch).
    USE: Structured `TimerPhrase` enum for clean intent dispatch.
  </action>
  <verify>
    cargo test -p plainpad-core --lib timers::
  </verify>
  <done>
    `parse_timer_phrase` correctly identifies all 8 syntax forms (`timer`, `timer 5`, `timer 3:30`, `timer pomo`, `timer 57 11`, `timer p`, `timer r`, `timer s`), and `intent.rs` emits corresponding annotations.
  </done>
</task>

<task type="auto">
  <name>Enhance TimerEngine with Pomodoro transitions, tick thread, and Tauri IPC commands</name>
  <files>
    crates/plainpad-core/src/timers.rs
    src-tauri/src/main.rs
  </files>
  <action>
    1. In `crates/plainpad-core/src/timers.rs`:
       - Extend `Timer` struct:
         ```rust
         #[derive(serde::Serialize, serde::Deserialize, Clone, Debug, PartialEq)]
         pub struct Timer {
             pub id: u64,
             pub name: String,
             pub timer_type: TimerType,
             pub duration_ms: u64,
             pub break_duration_ms: u64,
             pub elapsed_ms: u64,
             pub remaining_ms: u64,
             pub is_work_phase: bool,
             pub cycle_count: u32,
             pub is_paused: bool,
             pub started_at_ms: u64,
             pub due_ms: u64,
             pub last_tick_ms: u64,
             pub fired: bool,
         }
         ```
       - Define `TimerEvent`:
         ```rust
         #[derive(serde::Serialize, Clone, Debug)]
         pub enum TimerEvent {
             Tick(Timer),
             Fired { id: u64, name: String, sound: String, overdue: bool },
             PhaseTransition { id: u64, is_work_phase: bool, cycle_count: u32, sound: String },
         }
         ```
       - Implement engine operations:
         - `start_stopwatch(&self, name: &str) -> Timer`: starts stopwatch counting up from 0.
         - `start_countdown(&self, name: &str, duration_ms: u64) -> Timer`: starts countdown with remaining_ms = duration_ms.
         - `start_pomodoro(&self, name: &str, work_ms: u64, break_ms: u64) -> Timer`: starts Pomodoro in work phase (`is_work_phase = true`, `remaining_ms = work_ms`, `cycle_count = 0`).
         - `start_with_phrase(&self, phrase: &str) -> Option<Timer>`: parses phrase and dispatches to appropriate start method.
         - `pause_resume(&self, id: Option<u64>) -> Option<Timer>`: toggles `is_paused`. When unpausing, updates `last_tick_ms` to avoid jumping.
         - `restart(&self, id: Option<u64>) -> Option<Timer>`: resets elapsed/remaining to initial duration, preserves cycle_count or resets appropriately.
         - `get_active(&self) -> Option<Timer>`: returns first active (unfired, non-dismissed) timer.
         - `tick(&self, now: u64) -> Vec<TimerEvent>`:
           - Computes delta between `now` and `last_tick_ms`.
           - For Stopwatch: increases `elapsed_ms += delta`.
           - For Countdown: decreases `remaining_ms = remaining_ms.saturating_sub(delta)`. If reaching 0 and `!fired`: marks `fired = true`, emits `TimerEvent::Fired { sound: "countdowndone.m4a".into(), overdue: false }`.
           - For Pomodoro: decreases `remaining_ms = remaining_ms.saturating_sub(delta)`.
             - When work phase reaches 0: switches `is_work_phase = false`, sets `remaining_ms = break_duration_ms`, emits `TimerEvent::PhaseTransition { is_work_phase: false, cycle_count, sound: "workdone.m4a".into() }`.
             - When break phase reaches 0: switches `is_work_phase = true`, increments `cycle_count += 1`, sets `remaining_ms = duration_ms`, emits `TimerEvent::PhaseTransition { is_work_phase: true, cycle_count, sound: "backtowork.m4a".into() }`.
           - Emits `TimerEvent::Tick` for UI sync.
           - Calls `persist()`.
         - In `spawn_watcher`: loop every 250ms calling `tick()`, dispatching events to callback.
         - Add `pause_active(&self)` and `resume_active(&self)` to support `pauseTheTimerIfAppClosed`.
    2. In `src-tauri/src/main.rs`:
       - Add Tauri commands:
         - `start_timer(name: String, duration_ms: Option<u64>, timer_type: Option<String>, break_ms: Option<u64>, state: tauri::State<'_, Mutex<AppState>>) -> Timer`
         - `start_timer_phrase(phrase: String, state: tauri::State<'_, Mutex<AppState>>) -> Result<Timer, String>`
         - `pause_timer(id: Option<u64>, state: tauri::State<'_, Mutex<AppState>>) -> Option<Timer>`
         - `resume_timer(id: Option<u64>, state: tauri::State<'_, Mutex<AppState>>) -> Option<Timer>`
         - `restart_timer(id: Option<u64>, state: tauri::State<'_, Mutex<AppState>>) -> Option<Timer>`
         - `get_active_timer(state: tauri::State<'_, Mutex<AppState>>) -> Option<Timer>`
         - Update `cancel_timer` to accept `id: Option<u64>` (canceling active timer if None).
       - In watcher thread in `main.rs`:
         - Listen to `TimerEvent` and emit corresponding Tauri events to webview:
           - `"timer-tick"` with serialized `Timer`
           - `"timer-fired"` with payload `{ id, name, sound, overdue }`
           - `"timer-phase-transition"` with payload `{ id, is_work_phase, cycle_count, sound }`
       - Register all new commands in `generate_handler![...]`.
       - Handle `pauseTheTimerIfAppClosed` setting on window hide/minimize and restore.

    AVOID: Unsynchronized time jumps when resuming a paused timer.
    USE: Delta-based ticking from `last_tick_ms` and persistent JSON state.
  </action>
  <verify>
    cargo check --manifest-path src-tauri/Cargo.toml
  </verify>
  <done>
    `TimerEngine` manages Stopwatch, Countdown, and Pomodoro lifecycles with state transitions, emits tick and transition events, and all Tauri IPC commands compile cleanly.
  </done>
</task>

<task type="auto">
  <name>Write comprehensive unit and integration tests for TimerEngine and Pomodoro transitions</name>
  <files>
    crates/plainpad-core/tests/timer_engine_tests.rs
  </files>
  <action>
    Create `crates/plainpad-core/tests/timer_engine_tests.rs`:
    1. Test phrase parser:
       - `timer` -> `TimerPhrase::StartStopwatch`
       - `timer 5` -> `TimerPhrase::StartCountdown` with 300,000 ms
       - `timer 3:30` -> `TimerPhrase::StartCountdown` with 210,000 ms
       - `timer pomo` -> `TimerPhrase::StartPomodoro` with 25m work, 5m break
       - `timer 57 11` -> `TimerPhrase::StartPomodoro` with 57m work, 11m break
       - `timer p`, `timer r`, `timer s` -> correct `TimerControlAction` variants
    2. Test Stopwatch engine:
       - Start stopwatch, advance time, tick -> assert `elapsed_ms` increases.
       - Pause stopwatch -> advance time, tick -> assert `elapsed_ms` does not increase.
       - Restart stopwatch -> assert `elapsed_ms` resets to 0.
    3. Test Countdown engine:
       - Start countdown with 5000ms.
       - Advance time past 5000ms -> assert `Fired` event emitted with `countdowndone.m4a` sound and `fired == true`.
    4. Test Pomodoro transitions:
       - Start Pomodoro with work_ms = 2000ms, break_ms = 1000ms.
       - Assert initial `is_work_phase == true`, `cycle_count == 0`.
       - Tick past 2000ms -> assert transition to `is_work_phase == false` and `PhaseTransition` event with `workdone.m4a`.
       - Tick past another 1000ms -> assert transition back to `is_work_phase == true`, `cycle_count == 1`, and `PhaseTransition` event with `backtowork.m4a`.
    5. Test persistence and restart:
       - Start active countdown and save to temp file.
       - Reload from disk -> assert active timer reloaded with matching id, duration, and remaining time.
       - Expire timer while app offline, reload -> assert overdue fired timer emitted.

    AVOID: Leaking temporary test state files.
    USE: Process-scoped temp paths cleaned up in test teardown.
  </action>
  <verify>
    cargo test -p plainpad-core --test timer_engine_tests
  </verify>
  <done>
    All timer engine unit and integration tests pass with 0 failures, proving robust parsing, ticking, Pomodoro transitions, and persistence.
  </done>
</task>

</tasks>

<verification>
After all tasks complete, verify:
- [ ] `cargo test -p plainpad-core --test timer_engine_tests` passes with 0 failures
- [ ] `timer` starts a Stopwatch counting up from 00:00
- [ ] `timer 5` and `timer 3:30` start countdowns with exact millisecond durations
- [ ] `timer pomo` (25m/5m) and `timer 57 11` (57m/11m) start Pomodoro sessions
- [ ] `timer p`, `timer r`, `timer s` trigger pause/resume, restart, and stop actions
- [ ] Pomodoro transitions between work phase and break phase, tracking cycle count and emitting sound triggers (`workdone.m4a`, `backtowork.m4a`)
- [ ] Timer state persists to JSON and recovers correctly across engine reloads
</verification>

<success_criteria>
- [ ] All 3 tasks verified passing
- [ ] Must-haves confirmed
- [ ] No regressions in plainpad-core or src-tauri
</success_criteria>
