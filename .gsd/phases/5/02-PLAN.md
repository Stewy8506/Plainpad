---
phase: 5
plan: 2
wave: 2
depends_on:
  - "01"
files_modified:
  - scripts/copy-ui.mjs
  - src-tauri/tauri.conf.json
  - ui/src/timer.ts
  - ui/src/main.ts
  - ui/style.css
  - ui/index.html
  - ui/src/settings.ts
  - ui/settings.html
autonomous: true
gap_closure: false
must_haves:
  truths:
    - "Interactive Timer Pill displays animated circular progress with circleDiameter and circleSpacing"
    - "Timer Pill shows remaining or elapsed time, work vs break phase badge, and cycle count"
    - "Clicking Timer Pill pauses/resumes active timer; hover reveals restart and stop controls"
    - "Typing timer phrases in editor (`timer`, `timer 5`, `timer 3:30`, `timer pomo`, `timer 57 11`, `timer p`, `timer r`, `timer s`) shows interactive chips and starts/controls timers"
    - "Audio alerts (`countdowndone.m4a`, `workdone.m4a`, `backtowork.m4a`) play upon timer expiration and Pomodoro phase transitions"
    - "Full-screen alert overlay (`TimerCompleteAlertView`) displays when enabled via `timerPomoShowFullScreenAlert`"
    - "Settings `timerCountdownPlaySound`, `timerPomoShowFullScreenAlert`, and `pauseTheTimerIfAppClosed` load and save correctly"
  artifacts:
    - "ui/src/timer.ts"
    - "ui/style.css"
    - "ui/index.html"
    - "scripts/copy-ui.mjs"
    - "ui/src/settings.ts"
    - "ui/settings.html"
  key_links:
    - "ui/src/timer.ts listens for Tauri events timer-tick, timer-fired, and timer-phase-transition"
    - "ui/src/timer.ts renders circular SVG progress ring and audio playback"
    - "ui/src/main.ts connects editor chips (timer, timer_stop, timer_control) to timer actions"
    - "ui/src/settings.ts synchronizes timer settings to backend preferences store"
---

# Plan 5.2: Interactive Timer Pill UI (TimerView), Circular Visuals, Sound Alerts, and Full-Screen Overlay

<objective>
Build the interactive Timer Pill UI (`TimerView`) with animated circular SVG progress ring (`circleDiameter`, `circleSpacing`), Pomodoro work/break phase indicators, cycle counter, click-to-pause/resume, hover controls (restart, stop), native audio chime playback (`countdowndone.m4a`, `workdone.m4a`, `backtowork.m4a`), full-screen alert overlay (`TimerCompleteAlertView`), and preferences synchronization (`timerCountdownPlaySound`, `timerPomoShowFullScreenAlert`, `pauseTheTimerIfAppClosed`).

Purpose: Antinote provides a distinctive, non-intrusive floating timer pill with animated circular progress and audio cues that keep track of productivity sessions without obscuring notes.
Output: `ui/src/timer.ts`, updated `ui/src/main.ts`, enhanced `style.css` and `index.html`, asset copying pipeline in `scripts/copy-ui.mjs`, and preferences controls in `settings.ts` / `settings.html`.
</objective>

<context>
Load these files for context:
- .gsd/SPEC.md (§6 Timer & Pomodoro Engine, §11 Preferences)
- .gsd/findings.md (§11 Timer, Stopwatch & Pomodoro Subsystem)
- .gsd/REQUIREMENTS.md (REQ-15, REQ-16, REQ-17)
- .gsd/phases/5/01-PLAN.md
- ui/src/main.ts
- ui/src/settings.ts
- ui/settings.html
- ui/index.html
- ui/style.css
- scripts/copy-ui.mjs
- src-tauri/tauri.conf.json
</context>

<tasks>

<task type="auto">
  <name>Build interactive Timer Pill (TimerView) with circular SVG progress, controls, and full-screen alert modal</name>
  <files>
    ui/src/timer.ts
    ui/index.html
    ui/style.css
  </files>
  <action>
    1. Create `ui/src/timer.ts`:
       - Define types:
         ```ts
         export type TimerType = 'Stopwatch' | 'Countdown' | 'Pomodoro';

         export interface Timer {
           id: number;
           name: string;
           timer_type: TimerType;
           duration_ms: number;
           break_duration_ms: number;
           elapsed_ms: number;
           remaining_ms: number;
           is_work_phase: boolean;
           cycle_count: number;
           is_paused: boolean;
           fired: boolean;
         }
         ```
       - Implement `initTimerUI()`:
         - Mount `#timer-pill` inside `#timer-display`.
         - Structure of `#timer-pill`:
           - Circular SVG progress container (`.timer-progress-ring`):
             - Outer diameter `circleDiameter = 28px`, stroke width `2.5px`, spacing `4px`.
             - Background track `<circle class="ring-bg" />`.
             - Dynamic foreground arc `<circle class="ring-progress" />`.
           - Meta container (`.timer-meta`):
             - Phase badge (`.timer-phase-badge`): e.g. "Work (Cycle 1)" or "Break (Cycle 1)" or "Stopwatch" or timer name.
             - Time digits (`.timer-digits`): tabular numeric format `mm:ss` or `hh:mm:ss`.
           - Hover action controls (`.timer-controls`):
             - Pause / Resume toggle button (`.timer-btn-pause`, ⏸ / ▶)
             - Restart button (`.timer-btn-restart`, ↺)
             - Stop / Dismiss button (`.timer-btn-stop`, ✕)
         - Wire event listeners for Tauri events:
           - `"timer-tick"`: update progress ring dashoffset, time digits, paused visual state, and phase badge in real time.
           - `"timer-fired"`: trigger expiration chime and temporary celebration pulse.
           - `"timer-phase-transition"`: trigger work/break chime and display full-screen overlay if enabled.
         - On startup, call `invoke<Timer | null>('get_active_timer')` to restore display immediately if a timer is active.
       - Implement circular ring calculation:
         - Radius `r = (circleDiameter - strokeWidth) / 2 = 12.75px`.
         - Circumference `c = 2 * Math.PI * r = 80.11px`.
         - Set `strokeDasharray = c`.
         - For Countdown & Pomodoro: `progress = remaining_ms / current_phase_duration`; `strokeDashoffset = c * (1 - progress)`.
         - For Stopwatch: continuous animated rotation or pulsing glowing ring.
         - Ring colors:
           - Work phase: `--accent-coral`
           - Break phase: `--accent-teal`
           - Paused: `--accent-amber` with dashed stroke
       - Implement Full-Screen Overlay (`TimerCompleteAlertView`):
         - Modal `#timer-fullscreen-alert`:
           - Centered card with large phase transition icon, headline ("Work Complete!", "Break Finished!", or "Time's Up!"), cycle summary ("Completed Cycle 2"), and primary action button ("Start Break", "Resume Work", or "Dismiss").
           - Dismissable with `Escape` or `Enter`.
    2. In `ui/index.html`:
       - In `#timer-display`, ensure structure accommodates `#timer-pill`.
       - Add `#timer-fullscreen-alert` container before closing `</body>`.
    3. In `ui/style.css`:
       - Add CSS classes:
         - `.timer-pill`, `.timer-progress-ring`, `.ring-bg`, `.ring-progress`, `.timer-meta`, `.timer-phase-badge`, `.timer-digits`, `.timer-controls`, `.timer-btn-pause`, `.timer-btn-restart`, `.timer-btn-stop`.
         - Style `.timer-fullscreen-alert` with translucent backdrop blur (`backdrop-filter: blur(12px)`), smooth fade-in, and prominent typography.
         - Ensure responsive positioning at bottom center with clean elevation shadows.

    AVOID: Polling backend repeatedly from UI when background tick events are broadcast via Tauri.
    USE: Reactive event handling from `"timer-tick"` and SVG `strokeDashoffset` animation.
  </action>
  <verify>
    npm run typecheck
  </verify>
  <done>
    `TimerView` renders circular SVG progress, work/break phase indicators, cycle counts, hover controls, and full-screen modal alert with zero TypeScript compilation errors.
  </done>
</task>

<task type="auto">
  <name>Integrate audio playback, asset copying, editor trigger chips, and timer settings</name>
  <files>
    scripts/copy-ui.mjs
    src-tauri/tauri.conf.json
    ui/src/timer.ts
    ui/src/main.ts
    ui/src/settings.ts
    ui/settings.html
  </files>
  <action>
    1. In `scripts/copy-ui.mjs`:
       - Ensure `ui/dist/sounds/` directory is created recursively.
       - Copy all audio files from `ui/sounds/` (`backtowork.m4a`, `countdowndone.m4a`, `workdone.m4a`) into `ui/dist/sounds/`.
    2. In `src-tauri/tauri.conf.json`:
       - Update security CSP: append `media-src 'self' asset: data: blob:;` to permit HTML5 Audio playback.
    3. In `ui/src/timer.ts`:
       - Implement `playTimerChime(soundFile: string)`:
         - Check settings store for `timerCountdownPlaySound`. If sound is disabled, return early.
         - Create `const audio = new Audio('./sounds/' + soundFile)`.
         - Call `audio.play().catch(...)` to handle autoplay policy gracefully.
       - Connect `playTimerChime` to `"timer-fired"` (`countdowndone.m4a`) and `"timer-phase-transition"` (`workdone.m4a` when work phase finishes, `backtowork.m4a` when break phase finishes).
    4. In `ui/src/main.ts`:
       - Call `initTimerUI()` on initialization.
       - Enhance `acceptChip` for timer trigger categories:
         - When `anno.kind === 'timer'`: call `invoke('start_timer_phrase', { phrase: line })`.
         - When `anno.kind === 'timer_control'`:
           - If `anno.value === 'pause'`: call `invoke('pause_timer')`.
           - If `anno.value === 'restart'`: call `invoke('restart_timer')`.
         - When `anno.kind === 'timer_stop'`: call `invoke('cancel_timer')`.
       - Render `.anno-timer-chip` with distinct icon (`⏱`) and label based on trigger type (Stopwatch, Countdown, Pomodoro, Pause, Restart, Stop).
    5. In `ui/settings.html`:
       - In the "Timer & Stopwatch" tab:
         - Ensure `#setting-timer-sound` ("Audio alert chime") controls `timerCountdownPlaySound`.
         - Add `#setting-timer-pomo-fullscreen` ("Show full-screen alert on Pomodoro completion") controlling `timerPomoShowFullScreenAlert`.
         - Add `#setting-timer-pause-on-close` ("Pause active timer when app closes or minimizes") controlling `pauseTheTimerIfAppClosed`.
    6. In `ui/src/settings.ts`:
       - Register and load default values for:
         - `timerCountdownPlaySound` (default `true`)
         - `timerPomoShowFullScreenAlert` (default `false`)
         - `pauseTheTimerIfAppClosed` (default `true`)
       - Bind `change` event listeners to update and persist preferences immediately through `saveSetting()`.

    AVOID: Missing audio files in `ui/dist/sounds` during production and dev bundling.
    USE: Automated asset copy in `copy-ui.mjs` and native HTML5 Audio playback with CSP permissions.
  </action>
  <verify>
    npm run build:ui
  </verify>
  <done>
    Sound assets copy to `ui/dist/sounds`, chimes play on transitions, editor trigger chips start and control timers, settings persist cleanly, and `npm run build:ui` succeeds without error.
  </done>
</task>

</tasks>

<verification>
After all tasks complete, verify:
- [ ] `npm run build:ui` completes with 0 errors
- [ ] `scripts/copy-ui.mjs` copies `ui/sounds/*.m4a` to `ui/dist/sounds/`
- [ ] Interactive Timer Pill displays circular SVG progress ring, formatted time, work/break phase, and cycle count
- [ ] Clicking Timer Pill pauses/resumes active timer; hover controls allow restarting and stopping
- [ ] Typing timer keywords in editor (`timer`, `timer 5`, `timer 3:30`, `timer pomo`, `timer 57 11`, `timer p`, `timer r`, `timer s`) shows chips and starts/controls timers upon Enter or click
- [ ] Sound alerts (`countdowndone.m4a`, `workdone.m4a`, `backtowork.m4a`) play when countdown completes or Pomodoro phases alternate
- [ ] Full-screen alert overlay (`TimerCompleteAlertView`) displays when enabled via `timerPomoShowFullScreenAlert`
- [ ] Settings toggles for audio alerts, full-screen alerts, and pause on close persist and load correctly
</verification>

<success_criteria>
- [ ] All 2 tasks verified passing
- [ ] Must-haves confirmed
- [ ] Zero TypeScript or bundling errors
</success_criteria>
