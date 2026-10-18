---
phase: 10
plan: 1
wave: 1
gap_closure: false
depends_on: []
files_modified:
  - crates/plainpad-core/src/store/notes.rs
  - crates/plainpad-core/resources/break_facts.json
  - crates/plainpad-core/tests/streak_tests.rs
  - src-tauri/src/main.rs
  - ui/index.html
  - ui/src/timer.ts
  - ui/style.css
  - ui/styles.css
autonomous: true
must_haves:
  truths:
    - "SQLite app_state stores and updates current_no_peek_streak and best_no_peek_streak"
    - "Completing a break without peeking increments the streak and updates best_no_peek_streak if exceeded"
    - "Curated break_facts.json provides categorized bite-sized trivia (Wildlife, Space, Mindfulness)"
    - "Break Time overlay displays mint-green SVG orbit graphic, category badge, quote text, live countdown digits, and streak counter"
    - "Pause/resume button in the break view toggles Pomodoro break ticking in real-time"
  artifacts:
    - crates/plainpad-core/resources/break_facts.json
    - crates/plainpad-core/tests/streak_tests.rs
    - ui/src/timer.ts
    - ui/index.html
    - ui/style.css
  key_links:
    - "plainpad-core/src/store/notes.rs provides streak queries and mutations persisting in app_state"
    - "src-tauri/src/main.rs exposes get_no_peek_streak, increment_no_peek_streak, reset_no_peek_streak IPC commands"
    - "ui/src/timer.ts binds TimerEngine tick events to live break countdown and streak badge"
---

# Plan 10.1: SQLite No-Peek Streak Engine, Fact Catalog, and High-Fidelity Break Time View

<objective>
Build the core data model, backend IPC, and frontend presentation layer for Antinote's Break Time view:
1. SQLite streak tracking: Persist `current_no_peek_streak` and `best_no_peek_streak` in `app_state` with increment and reset operations.
2. Fact-File catalog: Provide a rich offline JSON collection of categorized trivia and mindfulness thoughts (`WILDLIFE FACT-FILE`, `SPACE TRIVIA`, `MINDFUL MOMENT`).
3. High-Fidelity Break View Component: Replicate Antinote's exact visual composition: mint-green SVG orbit illustration, category badge, random fact presentation, interactive countdown digits with pause/resume, and No-Peek streak counter with golden accents.
</objective>

<context>
Load these files for context:
- .gsd/SPEC.md (§6.3)
- crates/plainpad-core/src/store/notes.rs
- crates/plainpad-core/src/timers.rs
- src-tauri/src/main.rs
- ui/src/timer.ts
- ui/index.html
- ui/style.css
</context>

<tasks>

<task type="auto">
  <name>Implement SQLite No-Peek Streak Storage and Fact-File Catalog</name>
  <files>crates/plainpad-core/src/store/notes.rs, crates/plainpad-core/resources/break_facts.json, crates/plainpad-core/tests/streak_tests.rs, src-tauri/src/main.rs</files>
  <action>
    1. In `crates/plainpad-core/resources/break_facts.json`:
       - Create an offline JSON catalog of categorized facts with keys `category` and `text` (e.g. `WILDLIFE FACT-FILE`: "Ostriches can run faster than horses.", "Sea otters hold hands while sleeping so they don't drift apart.", "Octopuses have three hearts and blue blood.", `SPACE TRIVIA`: "A day on Venus is longer than a year on Venus.", `MINDFUL MOMENT`: "Take a deep breath. Release your shoulders.").
    2. In `crates/plainpad-core/src/store/notes.rs`:
       - Implement `get_no_peek_streak(&self) -> Result<(u32, u32), StoreError>` returning `(current, best)`.
       - Implement `increment_no_peek_streak(&self) -> Result<(u32, u32), StoreError>` incrementing `current_no_peek_streak`, updating `best_no_peek_streak` if `current > best`, and persisting both in `app_state`.
       - Implement `reset_no_peek_streak(&self) -> Result<(u32, u32), StoreError>` setting `current_no_peek_streak = 0` while preserving `best_no_peek_streak`.
    3. In `src-tauri/src/main.rs`:
       - Register Tauri commands: `get_no_peek_streak`, `increment_no_peek_streak`, `reset_no_peek_streak`, and `get_break_facts`.
    4. Write integration tests in `crates/plainpad-core/tests/streak_tests.rs`:
       - Verify streak increments on completion, best updates properly, and reset resets current to 0 while maintaining best.
  </action>
  <verify>cargo test --test streak_tests && cargo check --manifest-path src-tauri/Cargo.toml</verify>
  <done>Streak methods compile, pass unit tests, and are accessible via Tauri IPC.</done>
</task>

<task type="auto">
  <name>Build High-Fidelity Break Time View DOM Component and Real-Time Sync</name>
  <files>ui/index.html, ui/style.css, ui/styles.css, ui/src/timer.ts</files>
  <action>
    1. In `ui/index.html`:
       - Add `#break-time-overlay` dialog container:
         - SVG Mint-Green Orbit Graphic: Orbit ellipse looping through a triangle (`#4ade80`, stroke-width: 2.2, fill: none).
         - Header: `<h1 class="break-headline">Break Time</h1>`
         - Category badge: `<span class="break-category-badge" id="break-category-badge">WILDLIFE FACT-FILE</span>`
         - Fact display: `<p class="break-fact-text" id="break-fact-text">Ostriches can run faster than horses.</p>`
         - Subtitle: `<span class="break-remaining-label">Break time remaining:</span>`
         - Timer digits & controls: `<div class="break-digits-row"><button id="break-pause-btn" class="break-pause-btn" type="button">⏸</button><span id="break-countdown-digits" class="break-countdown-digits">05:00</span></div>`
         - Actions row: `<button id="break-sneak-btn" class="break-btn-pill">Sneak a peek</button><button id="break-end-btn" class="break-btn-pill">End Pomodoro</button>`
         - Streak indicator: `<div class="break-streak-row">No-Peek Streak: <span id="break-streak-current">0</span> (Best: <span id="break-streak-best">0</span>)</div>`
    2. In `ui/style.css` and `ui/styles.css`:
       - Style `#break-time-overlay`: Full viewport fixed overlay (`z-index: 99999`), centered flex layout, deep dark atmospheric background (`rgba(15, 18, 24, 0.94)` or `rgba(10, 12, 16, 0.92)`), `backdrop-filter: blur(48px) saturate(180%)`, radial vignette overlay with subtle ambient bottom-right violet/magenta lighting (`radial-gradient(circle at 85% 85%, rgba(139, 92, 246, 0.15), transparent 50%)`).
       - Typography: Clean sans-serif, bold `Break Time` title, uppercase tracked category badge, yellow/golden accent for streak numbers (`#fbbf24`), translucent pill buttons (`rgba(255, 255, 255, 0.12)` with hover highlight `rgba(255, 255, 255, 0.22)`).
    3. In `ui/src/timer.ts`:
       - Wire `timer-tick` to update `#break-countdown-digits` when in Pomodoro break phase (`!timer.is_work_phase`).
       - Wire `#break-pause-btn` to toggle pause/resume on click and update pause icon (`⏸` vs `▶`).
       - Implement `showBreakTimeView(timer)` and `hideBreakTimeView()`, fetching initial streak and rotating a random fact from catalog on display.
  </action>
  <verify>npm run build:ui && npm run typecheck</verify>
  <done>Break Time view renders with high-fidelity styling matching the Antinote screenshot, updates countdown live, and toggles pause/resume cleanly.</done>
</task>

</tasks>
