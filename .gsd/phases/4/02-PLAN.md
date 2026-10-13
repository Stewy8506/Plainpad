---
phase: 4
plan: 2
wave: 2
depends_on:
  - "4.1"
files_modified:
  - crates/plainpad-core/Cargo.toml
  - crates/plainpad-core/src/clipboard.rs
  - crates/plainpad-core/src/lib.rs
  - src-tauri/Cargo.toml
  - src-tauri/src/main.rs
  - ui/src/main.ts
  - ui/style.css
  - crates/plainpad-core/tests/autopaste_tests.rs
autonomous: true
gap_closure: false
must_haves:
  truths:
    - "Typing paste trigger or clicking AutoPaste chip displays the AutoPasteIndicator banner"
    - "Activating AutoPaste starts continuous OS-wide clipboard monitoring without blocking GUI responsiveness"
    - "New text copied to the system clipboard anywhere across the OS is automatically appended to the active note on a new line"
    - "Duplicate consecutive copies of identical text are suppressed and not appended repeatedly"
    - "Pressing Esc stops clipboard monitoring, dismisses the AutoPasteIndicator, and restores standard editor state"
  artifacts:
    - "crates/plainpad-core/src/clipboard.rs"
    - "crates/plainpad-core/src/lib.rs"
    - "src-tauri/Cargo.toml"
    - "src-tauri/src/main.rs"
    - "ui/src/main.ts"
    - "ui/style.css"
    - "crates/plainpad-core/tests/autopaste_tests.rs"
  key_links:
    - "src-tauri/src/main.rs manages background thread running plainpad_core::clipboard::AutoPasteEngine"
    - "Tauri emits autopaste-clip event containing newly copied clipboard text"
    - "ui/src/main.ts listens for autopaste-clip and appends text onto the active note"
    - "ui/src/main.ts intercepts Escape key to invoke stop_autopaste when AutoPaste is active"
---

# Plan 4.2: AutoPaste Continuous Clipboard Ingestion Engine with Esc Termination

<objective>
Implement the continuous AutoPaste clipboard capture engine replicating Antinote's `AutoPasteMonitor` (REQ-14, SPEC §5, findings §6). When the user types the `paste` trigger or confirms AutoPaste suggestion:
1. Display the `AutoPasteIndicator` banner with status text:
   *"Enter to start AutoPaste. Everything you copy will be added here. Esc stops autopaste."* (or active status when running).
2. Start a cross-platform background clipboard watcher thread in Rust/Tauri using `arboard` that monitors system clipboard copies with duplicate suppression.
3. Whenever text is copied anywhere on the OS, append the content to the active note on a new line.
4. Pressing `Esc` immediately deactivates clipboard monitoring, dismisses the indicator, and restores normal editor behavior.

Purpose: Antinote allows rapid multi-source research and clipping by continuously ingesting whatever the user copies into the scratchpad until explicitly dismissed with Esc.
Output: Cross-platform clipboard monitor in `clipboard.rs`, Tauri IPC & background worker in `main.rs`, UI banner & append logic in `main.ts`, styling in `style.css`, and integration test suite `autopaste_tests.rs`.
</objective>

<context>
Load these files for context:
- .gsd/SPEC.md (§5 Natural-Language Triggers & Keyword Taxonomy)
- .gsd/findings.md (§6 AutoPaste & Clipboard Monitoring Engine)
- .gsd/REQUIREMENTS.md (REQ-14)
- crates/plainpad-core/src/trigger_config.rs
- crates/plainpad-core/src/intent.rs
- src-tauri/src/main.rs
- ui/src/main.ts
- ui/style.css
</context>

<tasks>

<task type="auto">
  <name>Implement Rust cross-platform clipboard watcher service, duplicate suppression, and Tauri IPC commands</name>
  <files>
    crates/plainpad-core/Cargo.toml
    crates/plainpad-core/src/clipboard.rs
    crates/plainpad-core/src/lib.rs
    src-tauri/Cargo.toml
    src-tauri/src/main.rs
  </files>
  <action>
    1. In `crates/plainpad-core/Cargo.toml` and `src-tauri/Cargo.toml`:
       - Add `arboard = "3.4"` dependency for cross-platform OS clipboard access.
    2. Create `crates/plainpad-core/src/clipboard.rs`:
       - Define `pub trait ClipboardReader: Send + Sync { fn read_text(&mut self) -> Result<Option<String>, String>; }`.
       - Implement `SystemClipboardReader`: wraps `arboard::Clipboard` and calls `get_text()`.
       - Implement `MockClipboardReader`: stores in-memory text queue for headless CI tests where no physical display server (X11/Wayland) is running.
       - Define `pub struct AutoPasteEngine`:
         - `is_active: Arc<std::sync::atomic::AtomicBool>`
         - `last_content: Arc<std::sync::Mutex<Option<String>>>`
       - Implement methods on `AutoPasteEngine`:
         - `pub fn new() -> Self`
         - `pub fn start(&self) -> bool`: sets `is_active` to true, returns previous state.
         - `pub fn stop(&self) -> bool`: sets `is_active` to false, returns previous state.
         - `pub fn is_active(&self) -> bool`
         - `pub fn process_clip(&self, text: &str) -> Option<String>`:
           - Trims leading/trailing whitespace.
           - If trimmed string is empty, returns `None`.
           - Locks `last_content`: if identical to `Some(trimmed)`, returns `None` (suppress duplicate consecutive copies).
           - Updates `last_content = Some(trimmed.to_string())`, and returns `Some(trimmed.to_string())`.
    3. In `crates/plainpad-core/src/lib.rs`: export `pub mod clipboard;` and re-export `AutoPasteEngine`, `ClipboardReader`, `MockClipboardReader`.
    4. In `src-tauri/src/main.rs`:
       - Add `autopaste: Arc<plainpad_core::clipboard::AutoPasteEngine>` to `AppState`.
       - Implement Tauri command `start_autopaste(app: AppHandle, state: tauri::State<'_, AppStateLock>) -> Result<(), String>`:
         - Call `state.lock().unwrap().autopaste.start()`.
         - Spawn background thread with `app_handle` and `Arc::clone(&autopaste)`:
           - Instantiates clipboard reader (with graceful handling if display is unavailable).
           - Loops while `autopaste.is_active()`:
             - Polls clipboard every 250ms (`std::thread::sleep(Duration::from_millis(250))`).
             - When new text is retrieved and `process_clip(&text)` yields `Some(new_clip)`:
               - Emits Tauri event `autopaste-clip` to webview with payload `{"text": new_clip}`.
       - Implement Tauri command `stop_autopaste(state: tauri::State<'_, AppStateLock>) -> Result<(), String>`:
         - Calls `state.lock().unwrap().autopaste.stop()`.
       - Implement Tauri command `get_autopaste_status(state: tauri::State<'_, AppStateLock>) -> bool`:
         - Returns `state.lock().unwrap().autopaste.is_active()`.
       - Register `start_autopaste`, `stop_autopaste`, and `get_autopaste_status` in `tauri::generate_handler![...]`.

    AVOID: Running clipboard read operations synchronously on the UI main thread or creating orphaned polling threads.
    USE: Atomic-flag controlled worker thread with duplicate suppression and clean Tauri event emission.
  </action>
  <verify>
    cargo test -p plainpad-core --lib clipboard::
  </verify>
  <done>
    `AutoPasteEngine` tracks active state, suppresses duplicates, and Tauri IPC commands start/stop the clipboard monitoring worker thread cleanly.
  </done>
</task>

<task type="auto">
  <name>Implement UI AutoPasteIndicator banner, clipboard append handler, and Esc deactivation</name>
  <files>
    ui/src/main.ts
    ui/style.css
  </files>
  <action>
    1. In `ui/style.css`:
       - Style `.autopaste-indicator`:
         - Fixed floating banner positioned above the note editor or at the bottom bar.
         - Glassmorphic translucent styling (`backdrop-filter: blur(12px)`), rounded corners, subtle amber border and glow.
         - Status text with monospace/clean font.
         - Animated pulsating recording dot (`.indicator-pulse`) when actively monitoring.
         - "Stop (Esc)" action button with clear hover state.
         - Smooth CSS enter/exit transitions (`transform: translateY(...)`, `opacity`).
    2. In `ui/src/main.ts`:
       - Add state variable `let isAutoPasteActive = false;`.
       - Create DOM element `autopasteIndicator`:
         - Proposed state text: *"Enter to start AutoPaste. Everything you copy will be added here. Esc stops autopaste."*
         - Active state text: *"AutoPaste active. Everything you copy will be added here. Esc stops autopaste."*
       - When intent analysis detects a `paste` trigger chip:
         - Show the `autopasteIndicator` in proposed state.
       - Implement `startAutoPaste()`:
         - Invoke Tauri `start_autopaste`.
         - Set `isAutoPasteActive = true`.
         - Update indicator styling to active state (pulsating dot).
         - If cursor is currently on a line that consists solely of `paste`, clear that trigger line from `editor.value`.
       - Implement `stopAutoPaste()`:
         - Invoke Tauri `stop_autopaste`.
         - Set `isAutoPasteActive = false`.
         - Hide `autopasteIndicator`.
       - Setup listener for Tauri event `autopaste-clip`:
         - On incoming `{ text }`:
           - If `editor.value` has content and does not end with `\n`, append `\n`.
           - Append `text` to `editor.value`.
           - Ensure cursor/scroll moves smoothly to the bottom.
           - Call `scheduleSave()`.
           - Call `scheduleAnalyze()`.
       - Update keyboard listener in `editor.addEventListener('keydown', ...)`:
         - If `isAutoPasteActive` is true and `e.key === 'Escape'`:
           - `e.preventDefault()`.
           - Call `stopAutoPaste()`.
           - Return (do NOT hide the application window).
         - If cursor is on a line matching the `paste` trigger and user presses `Enter` or `Tab`:
           - `e.preventDefault()`.
           - Call `startAutoPaste()`.
       - Wire click listener on `autopasteIndicator` "Stop (Esc)" button to call `stopAutoPaste()`.

    AVOID: Allowing the Esc key to close the main Plainpad window while AutoPaste is active.
    USE: Priority keydown interception ensuring Esc first deactivates AutoPaste as specified in findings §6.
  </action>
  <verify>
    cargo check --manifest-path src-tauri/Cargo.toml
  </verify>
  <done>
    `AutoPasteIndicator` renders cleanly, Enter starts monitoring, incoming clips append on new lines, and Esc stops AutoPaste and hides the banner.
  </done>
</task>

<task type="auto">
  <name>Create integration test suite for AutoPaste lifecycle, duplicate suppression, and Esc deactivation</name>
  <files>
    crates/plainpad-core/tests/autopaste_tests.rs
  </files>
  <action>
    1. Create `crates/plainpad-core/tests/autopaste_tests.rs`.
    2. Write integration tests covering:
       - Engine creation: initial state is inactive (`is_active() == false`).
       - Starting engine: `start()` sets `is_active() == true`.
       - Stopping engine: `stop()` sets `is_active() == false`.
       - Valid clip ingestion: `process_clip("First clip")` returns `Some("First clip".to_string())`.
       - Duplicate suppression: calling `process_clip("First clip")` again immediately returns `None`.
       - Interleaved distinct clips: `process_clip("Clip A")` -> `Some`, `process_clip("Clip B")` -> `Some`, `process_clip("Clip A")` -> `Some`.
       - Empty and whitespace filtering: `process_clip("")`, `process_clip("   ")`, `process_clip("\n\t")` all return `None`.
       - Leading/trailing whitespace trimming: `process_clip("  trimmed  ")` returns `Some("trimmed".to_string())`.
       - Concurrent thread safety: spawning multiple reader threads asserting no panics, data races, or deadlocks.
       - Restart lifecycle: start -> ingest -> stop -> start -> ingest operates without corruption.

    AVOID: Depend on an active X11/Wayland display server in core tests.
    USE: `MockClipboardReader` and direct `AutoPasteEngine` testing for 100% deterministic CI execution.
  </action>
  <verify>
    cargo test -p plainpad-core --test autopaste_tests
  </verify>
  <done>
    All integration tests in `autopaste_tests.rs` pass with 0 failures, proving REQ-14 compliance.
  </done>
</task>

</tasks>

<verification>
After all tasks complete, verify:
- [ ] `cargo test -p plainpad-core --test autopaste_tests` passes with 0 failures
- [ ] AutoPaste engine starts and stops cleanly via Tauri IPC
- [ ] New system clipboard text is ingested and appended on a new line
- [ ] Duplicate consecutive copies are suppressed
- [ ] `AutoPasteIndicator` displays status text matching findings §6
- [ ] Esc key terminates AutoPaste and dismisses indicator without closing the main window
</verification>

<success_criteria>
- [ ] All 3 tasks verified passing
- [ ] Must-haves confirmed
- [ ] Zero test regressions across `plainpad-core`
</success_criteria>
