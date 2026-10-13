# Plan 1.2 Summary: Note Lifecycle CRUD, Soft Delete (The Void), Locking & Archiving

**Status:** Completed  
**Plan:** `.gsd/phases/1/02-PLAN.md`  
**Phase:** 1 (SQLite Storage Engine & Core Data Model 9 Parity)  

## Objective Achieved

Implemented complete Note lifecycle operations on `SqliteStore`:
- Note CRUD handling all Core Data Model 9 attributes (`id`, `content`, `created`, `last_modified`, `db_index`, `is_slotted`, `slot_index`, `is_locked`, `is_archived`, `is_private`, `is_tutorial`, `soft_deleted`, `deleted_at`, `skipped_urls`, `ck_change_tag`, `ck_system_fields`).
- Note stack ordering with `db_index` auto-increment on creation and atomic transaction reordering (`reorder_notes`).
- Note protection: locked notes (`is_locked = true`) reject saves and soft deletion (`StoreError::NoteLocked`).
- Archiving: archived notes (`is_archived = true`) are excluded from active notes and queried via `list_archived_notes`.
- Privacy and settings: `set_private`, `get_setting`, `set_setting`, `get_settings`, `get_geometry`, and `set_geometry`.
- The Void lifecycle: soft deletion with `deleted_at` timestamp, immunity for slotted notes (`StoreError::SlottedNoteCannotBeDeleted`), notes in void cannot be edited (`StoreError::NoteInVoid`), restoration to front of stack (`db_index = MAX + 1`), retention-based pruning (`purge_expired_void`), and permanent purge (`purge_void`, `permanently_delete_note`).

## Tasks Completed

| Task | Summary | Commit |
|---|---|---|
| Task 1 | Implemented Note CRUD, stack ordering, lock protection, archiving, and settings in `notes.rs` | `a3baadf` |
| Task 2 | Implemented The Void soft-delete, restore, retention, and permanent purge engine in `void.rs` | `bb4c4cd` |
| Task 3 | Created comprehensive integration test suite in `tests/note_lifecycle_tests.rs` with 8 tests | `f8acca8` |

## Key Artifacts Created / Modified

- `crates/plainpad-core/src/store/mod.rs`: Added `pub mod notes;`, `pub mod void;`, and `StoreError::SlottedNoteCannotBeDeleted`.
- `crates/plainpad-core/src/store/notes.rs`:
  - `create_note`: Assigns sequential `db_index` and defaults for 16 table attributes.
  - `load_note`: Reads all columns and parses JSON `skipped_urls`.
  - `save_note`: Rejects edits if locked or soft-deleted, updates `content` and `last_modified`.
  - `list_active_notes`: Filters `soft_deleted = 0 AND is_archived = 0` ordered by `db_index ASC`.
  - `reorder_notes`: Atomic transaction updating `db_index` for a sequence of note IDs.
  - `set_locked`, `set_archived`, `set_private`: Mutates state flags with `last_modified` updates.
  - `list_archived_notes`: Filters `soft_deleted = 0 AND is_archived = 1` ordered by `last_modified DESC`.
  - `get_setting`, `set_setting`, `get_settings`: App-state key-value store backed by SQLite `app_state`.
  - `get_geometry`, `set_geometry`: Window bounds `[x, y, width, height]` storage.
- `crates/plainpad-core/src/store/void.rs`:
  - `soft_delete_note`: Protects slotted and locked notes; sets `soft_deleted = 1` and `deleted_at`.
  - `restore_from_void`: Clears `deleted_at` and restores note to top of active stack (`MAX(db_index) + 1`).
  - `list_void`: Queries soft-deleted notes ordered by `deleted_at DESC`.
  - `purge_void`: Permanently deletes all notes in The Void, cascading to child tables.
  - `permanently_delete_note`: Permanently deletes an individual note from The Void.
  - `purge_expired_void`: Prunes notes older than `retention_days` cutoff.
- `crates/plainpad-core/tests/note_lifecycle_tests.rs`: Integration tests for CRUD, ordering, lock protection, archiving, The Void lifecycle, slotted immunity, settings, and expired void purge.

## Verification Proof

```
$ cargo test --package plainpad-core --test note_lifecycle_tests
running 8 tests
test test_settings_and_geometry ... ok
test test_slotted_note_void_immunity ... ok
test test_locked_note_protection ... ok
test test_note_crud_and_stack_ordering ... ok
test test_the_void_soft_delete_and_restore ... ok
test test_purge_void_permanent ... ok
test test_archive_filtering ... ok
test test_purge_expired_void ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s

$ cargo test --package plainpad-core
test result: ok. 41 passed; 0 failed; 0 ignored; 0 measured; finished in 0.31s (unittests)
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; finished in 0.02s (note_lifecycle_tests)
test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; finished in 0.01s (schema_tests)
Total: 53 passed; 0 failed
```

## Deviations & Notes

None. All implementations conform to `.gsd/SPEC.md` §2.1 and §9.

## Next Steps

Proceed to Plan 1.3 (`03-PLAN.md`): Relational child entity operations (`ChecklistItem`, `Antilink`, `Attachment`, `SyncState`), pre-restore safety snapshot manager (`BackupRestoreSheet`), and wiring `SqliteStore` into Tauri backend commands.
