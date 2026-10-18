---
phase: 10
plan: 1
wave: 1
gap_closure: false
depends_on: []
files_modified:
  - crates/plainpad-core/src/store/notes.rs
  - crates/plainpad-core/tests/timer_engine_tests.rs
  - src-tauri/src/main.rs
  - ui/src/break_catalog.ts
  - ui/src/break_view.ts
  - ui/index.html
  - ui/style.css
  - ui/styles.css
autonomous: true
must_haves:
  truths:
    - "SQLite app_state stores and maintains persistent current_no_peek_streak and best_no_peek_streak across restarts"
    - "Tauri IPC commands get_no_peek_streak, record_peek_penalty, and complete_break_streak allow querying and updating streaks"
    - "Curated offline trivia catalog provides 30+ facts and prompts categorized into WILDLIFE FACT-FILE, SPACE TRIVIA, MINDFUL MOMENTS, and SCIENCE SNIPPETS"
    - "Break Time view renders a centered mint-green SVG orbit illustration (#4ade80) with category badge, rotating fact card, and golden accent streak counter"
    - "Live interactive countdown displays mm:ss break time with an inline pause/resume control"
  artifacts:
    - crates/plainpad-core/src/store/notes.rs
    - src-tauri/src/main.rs
    - ui/src/break_catalog.ts
    - ui/src/break_view.ts
    - ui/index.html
    - ui/style.css
  key_links:
    - "crates/plainpad-core/src/store/notes.rs persists streak integers into SQLite app_state table"
    - "src-tauri/src/main.rs bridges streak state via get_no_peek_streak, record_peek_penalty, and complete_break_streak"
    - "ui/src/break_catalog.ts supplies categorized trivia to ui/src/break_view.ts"
    - "ui/src/break_view.ts binds real-time countdown digits and streak counters to HTML elements in ui/index.html"
---

# Plan 10.1: Fact-File Catalog, SQLite No-Peek Streak Engine, and High-Fidelity Break Time View

<objective>
Implement the foundational data storage, content catalog, and visual component for Antinote's Pomodoro break experience:
1. SQLite No-Peek Streak engine: Persist `current_no_peek_streak` and `best_no_peek_streak` in `app_state` with backend operations to query streaks, apply peek penalties (resetting current to 0), and record successful unpeeked completions.
2. Fact-File & mindfulness catalog: Provide a rich offline catalog of bite-sized wildlife facts, space trivia, mindfulness prompts, and science snippets with category badges and randomized rotation.
3. High-fidelity `BreakTimeView`: Build the UI component featuring a centered mint-green line-art orbit illustration (`#4ade80`), category badge, fact quote, live interactive countdown with inline pause/resume, and golden accent No-Peek streak indicator (`No-Peek Streak: X (Best: Y)`).
</objective>

<context>
Load these files for context:
- .gsd/SPEC.md (§6.3 Pomodoro Break Screen & Fullscreen Takeover)
- .gsd/REQUIREMENTS.md (REQ-31)
- crates/plainpad-core/src/store/notes.rs
- crates/plainpad-core/src/timers.rs
- src-tauri/src/main.rs
- ui/src/timer.ts
- ui/index.html
- ui/style.css
</context>

<tasks>

<task type="auto">
  <name>Implement SQLite No-Peek Streak Storage and Tauri IPC Endpoints</name>
  <files>crates/plainpad-core/src/store/notes.rs, crates/plainpad-core/tests/timer_engine_tests.rs, src-tauri/src/main.rs</files>
  <action>
    1. In `crates/plainpad-core/src/store/notes.rs`:
       - Add `get_no_peek_streak(&self) -> Result<(u32, u32), StoreError>`:
         - Queries `app_state` for keys `"current_no_peek_streak"` and `"best_no_peek_streak"`.
         - Parses each as `u32` (defaulting to 0 if missing or invalid). Returns `(current, best)`.
       - Add `record_peek_penalty(&self) -> Result<(u32, u32), StoreError>`:
         - Resets `"current_no_peek_streak"` to 0 in `app_state`.
         - Preserves `"best_no_peek_streak"`. Returns `(0, best)`.
       - Add `complete_break_streak(&self) -> Result<(u32, u32), StoreError>`:
         - Reads current streak, increments by 1.
         - Reads best streak, sets `best = max(best, new_current)`.
         - Persists both `"current_no_peek_streak"` and `"best_no_peek_streak"` to `app_state`.
         - Returns `(new_current, new_best)`.
    2. In `src-tauri/src/main.rs`:
       - Define `#[derive(serde::Serialize, Clone, Debug)] pub struct NoPeekStreak { pub current: u32, pub best: u32 }`.
       - Implement Tauri IPC commands:
         - `get_no_peek_streak(state: tauri::State<'_, Mutex<AppState>>) -> Result<NoPeekStreak, String>`
         - `record_peek_penalty(state: tauri::State<'_, Mutex<AppState>>, app: AppHandle) -> Result<NoPeekStreak, String>` which emits `"no-peek-streak-updated"` event with the updated streak.
         - `complete_break_streak(state: tauri::State<'_, Mutex<AppState>>, app: AppHandle) -> Result<NoPeekStreak, String>` which emits `"no-peek-streak-updated"` event with the updated streak.
       - Register commands in the `tauri::generate_handler![...]` invocation.
    3. In `crates/plainpad-core/tests/timer_engine_tests.rs`:
       - Add test verifying initial streaks return `(0, 0)`, three completions yield `(3, 3)`, a peek penalty resets current to `(0, 3)`, and subsequent completions increase current to `(1, 3)` without lowering best.
    AVOID: Keeping streak counters solely in frontend memory or browser localStorage where they would vanish on reload or crash.
    USE: Embedded SQLite `app_state` table managed by `SqliteStore`.
  </action>
  <verify>PATH=$PATH:/home/anv/.cargo/bin cargo test -p plainpad-core --test timer_engine_tests</verify>
  <done>Streak persistence tests pass; Tauri IPC endpoints compile and return typed streak records.</done>
</task>

<task type="auto">
  <name>Create Fact-File and Mindfulness Catalog with Category Rotation Engine</name>
  <files>ui/src/break_catalog.ts</files>
  <action>
    1. Create `ui/src/break_catalog.ts`.
    2. Export `BreakFact` interface:
       ```ts
       export interface BreakFact {
         id: string;
         category: 'WILDLIFE FACT-FILE' | 'SPACE TRIVIA' | 'MINDFUL MOMENTS' | 'SCIENCE SNIPPETS';
         text: string;
         source?: string;
       }
       ```
    3. Define and export a rich array of at least 30 verified, engaging facts and prompts matching Antinote's style:
       - Wildlife: Ostriches running faster than horses, sea otters holding hands, octopuses having three hearts and blue blood, hummingbirds flying backward, cows having best friends, elephant trunks having 40,000 muscles, tardigrades surviving deep space vacuum.
       - Space: A day on Venus outlasting a Venusian year, neutron star teaspoon mass, footsteps on the Moon enduring millions of years, complete vacuum acoustics, Saturn's rings consisting of 99% water ice.
       - Mindfulness: Taking three slow diaphragmatic breaths, dropping shoulders from ears, 20-20-20 eye gaze relaxation, noticing five textures in the room, mindful sip of water.
       - Science: Honey never expiring in sealed vessels, bananas possessing trace potassium-40 radioactivity, Mpemba effect in thermodynamics, human DNA sharing 60% with bananas.
    4. Implement retrieval functions:
       - `getRandomFact(preferredCategory?: string): BreakFact`
       - `getNextFact(currentId?: string, categoryFilter?: string): BreakFact`
       - `getFactById(id: string): BreakFact | undefined`
       - `getAllCategories(): string[]`
    AVOID: Calling third-party network APIs during breaks which fail offline or introduce latency.
    USE: Self-contained TypeScript catalog with deterministic rotation and category filtering.
  </action>
  <verify>npm run build:ui && npm run typecheck</verify>
  <done>ui/src/break_catalog.ts compiles with zero TypeScript errors and provides categorized fact lookup functions.</done>
</task>

<task type="auto">
  <name>Build High-Fidelity Break Time View UI Component and Styles</name>
  <files>ui/src/break_view.ts, ui/index.html, ui/style.css, ui/styles.css</files>
  <action>
    1. In `ui/index.html`, add `#break-time-view` container (hidden by default):
       - Centered container containing:
         - Orbit illustration: `#break-orbit-svg` with concentric elliptical orbits and nodes styled in `#4ade80`.
         - Category badge: `<span id="break-category-badge" class="break-category-badge">WILDLIFE FACT-FILE</span>`.
         - Fact display card: `<p id="break-fact-text" class="break-fact-text"></p>` and `<button id="break-fact-cycle-btn" class="break-fact-cycle-btn" type="button" title="Show another fact">↻ Another fact</button>`.
         - Live countdown row: `<div class="break-countdown-row"><span class="break-countdown-label">Break time remaining:</span> <button id="break-pause-btn" class="break-pause-btn" type="button" title="Pause / Resume">⏸</button> <span id="break-digits" class="break-digits">05:00</span></div>`.
         - Gamification streak pill: `<div class="break-streak-pill">No-Peek Streak: <span id="break-streak-current">0</span> <span class="break-streak-best">(Best: <span id="break-streak-best">0</span>)</span></div>`.
         - Action buttons: `<div class="break-actions-row"><button id="btn-sneak-peek" class="break-btn-peek" type="button">Sneak a peek</button><button id="btn-end-pomodoro" class="break-btn-end" type="button">End Pomodoro</button></div>`.
    2. Create `ui/src/break_view.ts`:
       - Export class/functions to control the break screen:
         - `initBreakView()`: attaches click listeners to cycle fact button, pause/resume button, sneak peek, and end pomodoro.
         - `showBreakView(initialRemainingMs: number, cycleCount: number)`: reveals break view, loads initial random fact, requests current streak from IPC, and updates display.
         - `hideBreakView()`: hides break view.
         - `updateBreakCountdown(remainingMs: number, isPaused: boolean)`: updates digits and pause button icon.
         - `updateStreakDisplay(current: number, best: number)`: updates streak pill.
    3. In `ui/style.css` and `ui/styles.css`:
       - Add styles for `.break-time-view`, `.break-orbit-svg`, `.break-category-badge`, `.break-fact-text`, `.break-countdown-row`, and `.break-streak-pill`.
       - Category badge: uppercase tracked font (`letter-spacing: 0.12em; font-size: 11px; font-weight: 700; color: #4ade80; background: rgba(74, 222, 128, 0.12); padding: 4px 12px; border-radius: 9999px; border: 1px solid rgba(74, 222, 128, 0.28);`).
       - Fact text: centered readable typography (`font-size: 18px; line-height: 1.5; color: #f8fafc; max-width: 520px; text-align: center;`).
       - Streak pill: golden accents (`color: #fbbf24; font-weight: 600; background: rgba(251, 191, 36, 0.12); border: 1px solid rgba(251, 191, 36, 0.25); border-radius: 9999px; padding: 4px 14px;`).
  </action>
  <verify>npm run build:ui && npm run typecheck</verify>
  <done>Break Time view markup, styles, and controller compile cleanly and export interactive break controls.</done>
</task>

</tasks>

<verification>
After all tasks, verify:
- [ ] `cargo test -p plainpad-core --test timer_engine_tests` passes for streak persistence.
- [ ] `npm run build:ui && npm run typecheck` succeeds without compilation errors.
- [ ] Break Time view displays the mint-green orbit vector, tracked category badge, rotating fact, countdown digits, and golden streak pill.
</verification>

<success_criteria>
- [ ] All tasks verified with empirical outputs.
- [ ] Must-haves confirmed.
</success_criteria>
