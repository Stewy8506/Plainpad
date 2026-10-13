---
phase: 2
plan: 1
wave: 1
depends_on: []
files_modified:
  - crates/plainpad-core/src/store/notes.rs
  - crates/plainpad-core/src/store/mod.rs
  - src-tauri/src/main.rs
  - crates/plainpad-core/tests/slotted_notes_tests.rs
autonomous: true
gap_closure: false
must_haves:
  truths:
    - "SQLite store initializes exactly 9 permanent slotted notes assigned to Greek deities (Hermes through Zeus) with slot_index 0..8"
    - "Initialization is idempotent and preserves user edits across subsequent launches"
    - "Slotted notes cannot be moved to The Void, batch-deleted, or have their slot positions displaced"
    - "Tauri IPC commands expose slotted notes listing and direct slot loading"
  artifacts:
    - "crates/plainpad-core/src/store/notes.rs"
    - "src-tauri/src/main.rs"
    - "crates/plainpad-core/tests/slotted_notes_tests.rs"
---

# Plan 2.1: Slotted Notes Backend Store Logic, Initialization, and Slot Invariance Enforcement

<objective>
Implement the permanent 9-slot scratchpad architecture in `plainpad-core` mapped to Greek deities (Hermes, Apollo/Aphrodite, Ares, Cronus, Gaia, Hades, Ouranos, Poseidon, Zeus). Guarantee idempotent database initialization, slot-based retrieval, bounds validation (0..8), and strict slot invariance rules (immunity to deletion into The Void, immunity to promotion displacement, and protection of empty slotted notes). Expose IPC commands in Tauri and verify with comprehensive integration tests.

Purpose: Antinote reserves 9 permanent scratchpad slots that stay with the user permanently. They are immune to deletion or stack displacement and provide instant predictable storage.
Output: Rust store methods (`ensure_slotted_notes`, `list_slotted_notes`, `get_slotted_note`), Tauri IPC commands, and integration test suite in `slotted_notes_tests.rs`.
</objective>

<context>
Load these files for context:
- .gsd/SPEC.md (§3 The 9 Slotted Notes Architecture (Greek Gods System))
- .gsd/findings.md (§4 The 9 Slotted Notes Architecture)
- crates/plainpad-core/src/model.rs
- crates/plainpad-core/src/store/notes.rs
- crates/plainpad-core/src/store/void.rs
- crates/plainpad-core/src/store/mod.rs
- src-tauri/src/main.rs
</context>

<tasks>

<task type="auto">
  <name>Implement 9-slot Greek deity initialization, query methods, and slot invariance protections in SqliteStore</name>
  <files>
    crates/plainpad-core/src/store/notes.rs
    crates/plainpad-core/src/store/mod.rs
  </files>
  <action>
    1. Define constants for the 9 Greek deities and their default initial contents:
       - Slot 0: Hermes (default text: "This is your first of nine slotted notes. It'll stay with you.")
       - Slot 1: Apollo
       - Slot 2: Ares
       - Slot 3: Cronus
       - Slot 4: Gaia
       - Slot 5: Hades
       - Slot 6: Ouranos
       - Slot 7: Poseidon
       - Slot 8: Zeus
    2. In `crates/plainpad-core/src/store/notes.rs`, add:
       - `pub fn ensure_slotted_notes(&self) -> Result<Vec<Note>, StoreError>`:
         - Uses a database transaction.
         - For each `slot_index` from 0 to 8:
           - Queries `SELECT id FROM notes WHERE is_slotted = 1 AND slot_index = ?1`.
           - If not found, generates a new UUID and inserts a row into `notes`:
             `is_slotted = 1`, `slot_index = i`, `db_index = i`, `content = default_text_for_slot(i)`, `created = now`, `last_modified = now`.
           - If found, preserves the existing note and content unchanged (ensuring idempotence).
         - Returns all 9 slotted notes ordered by `slot_index ASC`.
       - Wire `ensure_slotted_notes` into `SqliteStore::open` and `SqliteStore::open_in_memory` so that any store instance automatically ensures all 9 slots exist on initialization.
       - `pub fn list_slotted_notes(&self) -> Result<Vec<Note>, StoreError>`:
         - Queries `SELECT {NOTE_COLUMNS} FROM notes WHERE is_slotted = 1 AND soft_deleted = 0 ORDER BY slot_index ASC`.
         - Returns the vector of 9 notes.
       - `pub fn get_slotted_note(&self, slot_index: i32) -> Result<Option<Note>, StoreError>`:
         - Rejects invalid indices (`slot_index < 0 || slot_index > 8`) with `StoreError::InvalidOperation("Slot index must be between 0 and 8".to_string())`.
         - Queries `SELECT {NOTE_COLUMNS} FROM notes WHERE is_slotted = 1 AND slot_index = ?1 AND soft_deleted = 0`.
    3. Enforce slot invariance rules:
       - In `save_note(id, content)`:
         - Ensure saving empty content (`""`) to a slotted note is fully allowed and DOES NOT soft-delete or remove the note.
       - In `reorder_notes(note_ids_in_order)`:
         - Ensure reordering applies only to unslotted scratch notes or preserves `is_slotted` and `slot_index` intact, preventing slotted notes from having their slot index mutated or stripped.
       - In `soft_delete_note(id)`:
         - Confirm that `if note.is_slotted { return Err(StoreError::SlottedNoteCannotBeDeleted); }` prevents moving any slotted note into The Void.

    AVOID: overwriting existing slotted note content when `ensure_slotted_notes()` runs on subsequent app launches.
    USE: atomic transaction checking existence before insert, and strict bounds checking (0..8).
  </action>
  <verify>cargo test --package plainpad-core</verify>
  <done>SqliteStore initializes exactly 9 Greek deity slotted notes idempotently, validates slot bounds 0..8, and rejects deletion.</done>
</task>

<task type="auto">
  <name>Expose slotted note IPC commands in Tauri and create integration test suite</name>
  <files>
    src-tauri/src/main.rs
    crates/plainpad-core/tests/slotted_notes_tests.rs
  </files>
  <action>
    1. In `src-tauri/src/main.rs`:
       - Add IPC command `#[tauri::command] fn list_slotted_notes(state: tauri::State<'_, Mutex<AppState>>) -> Vec<NoteMeta>`:
         - Calls `state.lock().unwrap().store.list_slotted_notes().unwrap_or_default()`, maps each to `NoteMeta::from(&n)`.
       - Add IPC command `#[tauri::command] fn get_slotted_note(slot: i32, state: tauri::State<'_, Mutex<AppState>>) -> Result<Option<Note>, String>`:
         - Calls `state.lock().unwrap().store.get_slotted_note(slot).map_err(|e| e.to_string())`.
       - Register `list_slotted_notes` and `get_slotted_note` in `tauri::generate_handler![...]`.
       - In `main()`, after opening `SqliteStore`, call `store.ensure_slotted_notes().expect("Failed to initialize slotted notes");` to guarantee the 9 slots exist before webview initialization.
    2. Create `crates/plainpad-core/tests/slotted_notes_tests.rs` covering:
       - `test_slotted_notes_initialization_count_and_deities`:
         - Initializes an in-memory store.
         - Verifies `list_slotted_notes()` returns exactly 9 notes.
         - Verifies slot 0 has `is_slotted = true`, `slot_index = 0`, and contains Hermes text ("This is your first of nine slotted notes. It'll stay with you.").
         - Verifies slot indices 0 through 8 are monotonically assigned.
       - `test_slotted_notes_idempotence`:
         - Modifies note in slot 0 with custom text ("Hermes custom notes").
         - Calls `ensure_slotted_notes()` again.
         - Verifies slot 0 still contains "Hermes custom notes" (not reset to default).
       - `test_slotted_notes_bounds_and_query`:
         - `get_slotted_note(0)` through `get_slotted_note(8)` return `Ok(Some(note))`.
         - `get_slotted_note(-1)` and `get_slotted_note(9)` return `Err(StoreError::InvalidOperation(_))`.
       - `test_slotted_notes_invariance_rules`:
         - Verifies calling `soft_delete_note` on any slotted note returns `Err(StoreError::SlottedNoteCannotBeDeleted)`.
         - Verifies saving empty string `""` on a slotted note succeeds and keeps `is_slotted = true`.
         - Verifies creating unslotted scratch notes via `create_note()` assigns `is_slotted = false` and `slot_index = -1`, coexisting with slotted notes.

    AVOID: forgetting to register new commands in `generate_handler![]` which causes frontend `invoke` errors.
    USE: comprehensive integration tests asserting all 9 slots, idempotence, and invariance rules.
  </action>
  <verify>cargo test --test slotted_notes_tests && cargo check --manifest-path src-tauri/Cargo.toml</verify>
  <done>All tests in slotted_notes_tests pass green and Tauri IPC compiles cleanly.</done>
</task>

</tasks>

<verification>
After all tasks, verify:
- [ ] Exactly 9 slotted notes (slot_index 0..8) are initialized in SQLite on startup
- [ ] Slot 0 contains initial Hermes text and subsequent slots are ready
- [ ] Initialization is idempotent and does not overwrite existing notes
- [ ] Out-of-bounds slot requests (<0 or >8) return error
- [ ] Slotted notes reject deletion into The Void
- [ ] `list_slotted_notes` and `get_slotted_note` IPC commands compile and are registered in Tauri
</verification>

<success_criteria>
- [ ] All tasks verified via cargo test
- [ ] Must-haves confirmed
</success_criteria>
