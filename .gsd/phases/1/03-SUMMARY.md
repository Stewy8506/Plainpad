# Plan 1.3 Summary: Relational Store, Pre-Restore Snapshots & Tauri IPC Integration

**Status:** Completed  
**Plan:** `.gsd/phases/1/03-PLAN.md`  
**Phase:** 1 (SQLite Storage Engine & Core Data Model 9 Parity)  

## Objective Achieved

Implemented full relational child entity operations (`ChecklistItem`, `Antilink`, `Attachment`, `SyncState`) with atomic transaction and foreign key cascade deletion guarantees. Implemented the pre-restore safety snapshot manager (`BackupRestoreSheet` parity) using SQLite's online backup engine with automatic safety snapshots and cloud sync isolation (`sync_isolated: true`). Replaced `FileStore` with `SqliteStore` in the Tauri desktop application (`src-tauri/src/main.rs`), wiring all core note operations, app state, geometry, and exposing new Model 9 IPC commands (`purge_void`, `set_locked`, `set_archived`, `set_private`, `reorder_notes`, `create_pre_restore_snapshot`, `restore_from_snapshot`).

## Tasks Completed

| Task | Summary | Commit |
|---|---|---|
| Task 1 | Implemented relational child entity operations (Checklists, Antilinks, Attachments, SyncState) in `crates/plainpad-core/src/store/relational.rs` | `fcc75d5` |
| Task 2 | Implemented Pre-Restore Safety Snapshot manager with sync isolation in `crates/plainpad-core/src/store/snapshot.rs` and tests in `relational_and_snapshot_tests.rs` | `99d2f02` |
| Task 3 | Replaced `FileStore` with `SqliteStore` in `src-tauri` AppState, updated IPC commands, and registered Model 9 handlers | `15faae2` |

## Key Artifacts Created / Modified

- `crates/plainpad-core/src/store/relational.rs`:
  - `get_checklist_items`, `replace_checklist_items`, `toggle_checklist_item`
  - `get_antilinks`, `save_antilinks`
  - `get_attachments`, `add_attachment`, `delete_attachment`
  - `get_sync_state`, `set_sync_state`
- `crates/plainpad-core/src/store/snapshot.rs`:
  - `SnapshotMeta` struct (`path`, `timestamp`, `reason`, `size_bytes`)
  - `create_pre_restore_snapshot`: creates transactional snapshot using SQLite online backup API
  - `restore_from_snapshot`: creates safety snapshot (`pre_restore_safety`), restores pages, re-enforces foreign keys and migrations, and writes `sync_isolated = true` to `app_state`
  - `list_snapshots`: lists snapshots sorted newest first
  - `is_sync_isolated`: checks sync isolation status
- `crates/plainpad-core/src/store/mod.rs`: Added `path: Option<PathBuf>` to `SqliteStore`, `path()` accessor, and exposed `relational` and `snapshot` modules.
- `crates/plainpad-core/Cargo.toml`: Added `"backup"` feature to `rusqlite`.
- `crates/plainpad-core/tests/relational_and_snapshot_tests.rs`: Integration tests for relational CRUD, cascading deletion, pre-restore snapshots, and safe restore with sync isolation.
- `src-tauri/Cargo.toml`: Added `chrono = "0.4"` dependency.
- `src-tauri/src/main.rs`:
  - Wired `SqliteStore` wrapped in `Arc` into `AppState`.
  - Initialized SQLite database at `data_dir().join("plainpad.db")`.
  - Updated IPC commands: `list_notes`, `load_note`, `save_note`, `create_note`, `delete_note`, `restore_note`, `list_trash`, `set_pinned`, `set_plain`, `get_settings`, `set_setting`, `get_geometry`, `set_geometry`, `empty_trash_now`.
  - Added new IPC commands: `purge_void`, `set_locked`, `set_archived`, `set_private`, `reorder_notes`, `create_pre_restore_snapshot`, `restore_from_snapshot`.

## Verification Proof

```
$ cargo test --package plainpad-core --test relational_and_snapshot_tests
running 2 tests
test test_relational_crud_and_cascade ... ok
test test_snapshot_and_safe_restore ... ok

test result: ok. 2 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

$ cargo check --manifest-path src-tauri/Cargo.toml
    Checking plainpad v0.1.0 (/home/anv/Documents/Github/Plainpad/src-tauri)
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.92s

$ cargo test
test result: ok. 41 passed; 0 failed (plainpad_core lib)
test result: ok. 8 passed; 0 failed (note_lifecycle_tests)
test result: ok. 2 passed; 0 failed (relational_and_snapshot_tests)
test result: ok. 4 passed; 0 failed (schema_tests)
test result: ok. 0 passed; 0 failed (plainpad bin)
```

## Deviations & Notes

- Enabled the `"backup"` feature on `rusqlite` to allow transactional zero-copy backup and restore through SQLite's official online backup API, avoiding WAL lock conflicts.
- Maintained complete backwards compatibility for existing frontend IPC commands (`list_notes`, `load_note`, `save_note`, `create_note`, `delete_note`, `empty_trash_now`).

## Next Steps

Phase 1 (SQLite Storage Engine & Core Data Model 9 Parity) execution is complete. All requirements REQ-01 through REQ-04 are satisfied. Proceed to Phase 1 verification audit or transition to Phase 2: Slotted Notes (Greek Gods System) & Quick Navigation.
