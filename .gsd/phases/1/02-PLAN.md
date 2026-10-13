---
phase: 1
plan: 2
wave: 2
depends_on: ["01"]
files_modified:
  - crates/plainpad-core/src/store/notes.rs
  - crates/plainpad-core/src/store/void.rs
  - crates/plainpad-core/src/store/mod.rs
  - crates/plainpad-core/tests/note_lifecycle_tests.rs
autonomous: true
gap_closure: false
must_haves:
  truths:
    - "Note CRUD handles all 17 attributes with stack ordering via db_index"
    - "Locked notes (is_locked = true) cannot be edited or soft-deleted"
    - "Archived notes (is_archived = true) are excluded from active list and queried via list_archived_notes"
    - "Soft-deleted notes move to The Void with deleted_at timestamp"
    - "Slotted notes cannot be moved to The Void"
    - "The Void can be purged permanently and cascades deletes to child tables"
  artifacts:
    - "crates/plainpad-core/src/store/notes.rs"
    - "crates/plainpad-core/src/store/void.rs"
    - "crates/plainpad-core/tests/note_lifecycle_tests.rs"
---

# Plan 1.2: Note Lifecycle CRUD, Soft Delete (The Void), Locking & Archiving

<objective>
Implement complete Note lifecycle operations on `SqliteStore`: CRUD with all 17 attributes, stack ordering (`db_index`), lock protection (`is_locked`), archiving (`is_archived`), privacy toggle (`is_private`), and the soft-delete/recovery/purge lifecycle for "The Void" (`soft_deleted`, `deleted_at`).

Purpose: In Antinote, scratchpad notes have explicit ordering (`dbIndex`), lock protection, archiving out of active stack, and soft deletion into "The Void" with 30-day retention and permanent purge. Plainpad's `SqliteStore` must reproduce this exact behavior.
Output: `notes.rs` (CRUD, reordering, locking, archiving), `void.rs` (soft delete, restore, purge), and test suite in `note_lifecycle_tests.rs`.
</objective>

<context>
Load these files for context:
- .gsd/SPEC.md (Section 2.1: Entity Note, Section 9: The Void & Metal Shaders)
- .gsd/phases/1/01-PLAN.md
- crates/plainpad-core/src/model.rs
- crates/plainpad-core/src/store/mod.rs
</context>

<tasks>

<task type="auto">
  <name>Implement Note CRUD, stack ordering, and note state operations in SqliteStore</name>
  <files>
    crates/plainpad-core/src/store/notes.rs
    crates/plainpad-core/src/store/mod.rs
  </files>
  <action>
    Create `crates/plainpad-core/src/store/notes.rs` implementing Note lifecycle methods for `SqliteStore`:
    1. `create_note(&self, content: &str) -> Result<Note, StoreError>`:
       - Generate UUID v4 for `id`.
       - Assign UTC ISO 8601 strings to `created` and `last_modified`.
       - Assign `db_index = SELECT COALESCE(MAX(db_index), -1) + 1 FROM notes`.
       - Set default fields: `is_slotted: false`, `slot_index: -1`, `is_locked: false`, `is_archived: false`, `is_private: false`, `is_tutorial: false`, `soft_deleted: false`, `deleted_at: None`, `skipped_urls: []`.
       - Insert row into SQLite `notes` table and return constructed `Note`.
    2. `load_note(&self, id: &str) -> Result<Option<Note>, StoreError>`:
       - Query row by `id` extracting all 17 columns and parsing JSON `skipped_urls`.
    3. `save_note(&self, id: &str, content: &str) -> Result<Note, StoreError>`:
       - Load current note. If `is_locked == true`, return `Err(StoreError::NoteLocked)`.
       - If `soft_deleted == true`, return `Err(StoreError::NoteInVoid)`.
       - Update `content` and `last_modified = now_iso()`. Return updated `Note`.
    4. `list_active_notes(&self) -> Result<Vec<Note>, StoreError>`:
       - Query `WHERE soft_deleted = 0 AND is_archived = 0 ORDER BY db_index ASC`.
    5. `reorder_notes(&self, note_ids_in_order: &[String]) -> Result<(), StoreError>`:
       - Run transaction updating `db_index = index` for each note ID in `note_ids_in_order`.
    6. State flag mutators:
       - `set_locked(&self, id: &str, locked: bool) -> Result<(), StoreError>`: Updates `is_locked` and `last_modified`.
       - `set_archived(&self, id: &str, archived: bool) -> Result<(), StoreError>`: Updates `is_archived` and `last_modified`.
       - `set_private(&self, id: &str, private: bool) -> Result<(), StoreError>`: Updates `is_private` and `last_modified`.
       - `list_archived_notes(&self) -> Result<Vec<Note>, StoreError>`: Queries `WHERE soft_deleted = 0 AND is_archived = 1 ORDER BY last_modified DESC`.
    7. App state key-value store helper methods:
       - `get_setting(&self, key: &str) -> Result<Option<serde_json::Value>, StoreError>`
       - `set_setting(&self, key: &str, value: serde_json::Value) -> Result<(), StoreError>`
       - `get_settings(&self) -> Result<serde_json::Map<String, serde_json::Value>, StoreError>`
       - `get_geometry(&self) -> Option<[i32; 4]>` / `set_geometry(&self, geo: [i32; 4])`

    AVOID: allowing writes to locked notes (`is_locked == true`) because lock state must protect notes from accidental edits.
    USE: transactions and clear domain errors (`StoreError::NoteLocked`, `StoreError::NoteNotFound`).
  </action>
  <verify>cargo check --package plainpad-core</verify>
  <done>Note CRUD, stack ordering, lock, archive, and settings methods compile with complete error handling.</done>
</task>

<task type="auto">
  <name>Implement The Void soft-delete, restore, and permanent purge engine</name>
  <files>
    crates/plainpad-core/src/store/void.rs
    crates/plainpad-core/src/store/mod.rs
  </files>
  <action>
    Create `crates/plainpad-core/src/store/void.rs` implementing The Void lifecycle for `SqliteStore`:
    1. `soft_delete_note(&self, id: &str) -> Result<(), StoreError>`:
       - Load note by ID; error if not found.
       - Check if note is slotted (`is_slotted == true`): return `Err(StoreError::SlottedNoteCannotBeDeleted)` (SPEC §3 / findings §4: slotted notes cannot be moved to The Void).
       - Check if note is locked (`is_locked == true`): return `Err(StoreError::NoteLocked)`.
       - Update note setting `soft_deleted = 1`, `deleted_at = Some(now_iso())`, and `last_modified = now_iso()`.
    2. `restore_from_void(&self, id: &str) -> Result<Note, StoreError>`:
       - Load note by ID; error if not found or not soft-deleted.
       - Update note setting `soft_deleted = 0`, `deleted_at = None`, and `db_index = SELECT COALESCE(MAX(db_index), -1) + 1 FROM notes WHERE soft_deleted = 0` (restores to front of active stack).
       - Return updated `Note`.
    3. `list_void(&self) -> Result<Vec<Note>, StoreError>`:
       - Query notes `WHERE soft_deleted = 1 ORDER BY deleted_at DESC`.
    4. `purge_void(&self) -> Result<usize, StoreError>`:
       - Execute `DELETE FROM notes WHERE soft_deleted = 1`.
       - Foreign keys automatically cascade deletion to `checklist_items`, `antilinks`, `attachments`, `sync_state`.
       - Return count of deleted notes (matches Antinote prompt: "This will permanently delete %lld notes. There are no turtles beyond The Void.").
    5. `permanently_delete_note(&self, id: &str) -> Result<(), StoreError>`:
       - Execute `DELETE FROM notes WHERE id = ? AND soft_deleted = 1`. Error if not in void.
    6. `purge_expired_void(&self, retention_days: u32) -> Result<usize, StoreError>`:
       - Compute ISO cutoff timestamp `now - retention_days`.
       - Execute `DELETE FROM notes WHERE soft_deleted = 1 AND deleted_at < ?`. Return count.

    AVOID: moving slotted notes (`is_slotted == true`) to The Void because Antinote protects the permanent 9 Greek deity slots.
    USE: soft-delete with `deleted_at` timestamp enabling retention policies and permanent purge.
  </action>
  <verify>cargo check --package plainpad-core</verify>
  <done>The Void soft-delete, restore, list, expired pruning, and permanent purge methods compile.</done>
</task>

<task type="auto">
  <name>Create test suite for Note lifecycle, ordering, lock protection, and The Void</name>
  <files>
    crates/plainpad-core/tests/note_lifecycle_tests.rs
  </files>
  <action>
    Create integration test suite in `crates/plainpad-core/tests/note_lifecycle_tests.rs`:
    1. `test_note_crud_and_stack_ordering`:
       - Create 3 notes. Verify sequential `db_index` assignments (0, 1, 2).
       - Call `reorder_notes` with reversed IDs. Verify `list_active_notes` returns notes in reversed order.
    2. `test_locked_note_protection`:
       - Create note. Call `set_locked(id, true)`.
       - Attempt `save_note(id, "new text")`; assert returns `Err(StoreError::NoteLocked)`.
       - Attempt `soft_delete_note(id)`; assert returns `Err(StoreError::NoteLocked)`.
       - Call `set_locked(id, false)`; assert `save_note` and `soft_delete_note` now succeed.
    3. `test_archive_filtering`:
       - Create note. Call `set_archived(id, true)`.
       - Verify note is NOT returned by `list_active_notes()`.
       - Verify note IS returned by `list_archived_notes()`.
       - Call `set_archived(id, false)`; verify note reappears in `list_active_notes()`.
    4. `test_the_void_soft_delete_and_restore`:
       - Create note. Call `soft_delete_note(id)`.
       - Verify note disappears from active list and appears in `list_void()` with non-null `deleted_at`.
       - Call `restore_from_void(id)`. Verify note returns to active notes at top of stack.
    5. `test_slotted_note_void_immunity`:
       - Create note, manually mark `is_slotted = true`.
       - Call `soft_delete_note(id)`; assert returns `Err(StoreError::SlottedNoteCannotBeDeleted)`.
    6. `test_purge_void_permanent`:
       - Create 3 notes. Soft-delete 2 into The Void.
       - Call `purge_void()`. Assert return count is 2.
       - Assert `list_void()` is empty, and the 1 active note remains intact.

    AVOID: using physical disk directories in tests when `:memory:` provides instant deterministic test execution.
    USE: `SqliteStore::open_in_memory()` with complete assertions.
  </action>
  <verify>cargo test --package plainpad-core --test note_lifecycle_tests</verify>
  <done>All Note lifecycle, stack ordering, lock protection, and Void tests pass with 0 errors.</done>
</task>

</tasks>

<verification>
After all tasks, verify:
- [ ] `cargo test --package plainpad-core --test note_lifecycle_tests` passes with 0 errors
- [ ] Locked notes cannot be modified or soft-deleted
- [ ] Archived notes are separated from active notes
- [ ] The Void lifecycle (soft delete, restore, permanent purge) functions as specified
</verification>

<success_criteria>
- [ ] All tasks verified
- [ ] Must-haves confirmed
- [ ] No compilation warnings or regressions in plainpad-core
</success_criteria>
