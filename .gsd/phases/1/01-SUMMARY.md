# Plan 1.1 Summary: Core Data Model 9 SQLite Schema & Migrations

**Status:** Completed  
**Plan:** `.gsd/phases/1/01-PLAN.md`  
**Phase:** 1 (SQLite Storage Engine & Core Data Model 9 Parity)  

## Objective Achieved

Established the embedded SQLite engine in `plainpad-core` reproducing Antinote's Core Data Model 9 schema and historical migration evolution (Models 1 through 9). Defined strongly typed Rust structs for all 5 Core Data entities (`Note`, `ChecklistItem`, `Antilink`, `Attachment`, `SyncState`) and built `SqliteStore` with automatic WAL journal mode, mandatory `PRAGMA foreign_keys = ON` enforcement, sequential migration execution, and foreign key cascade deletion.

## Tasks Completed

| Task | Summary | Commit |
|---|---|---|
| Task 1 | Defined Core Data Model 9 data entities in `src/model.rs` and added `rusqlite`, `uuid`, and `chrono` dependencies to `plainpad-core` | `e8038a0` |
| Task 2 | Implemented sequential SQLite schema migrations (Models 1-9) in `src/store/schema.rs` and `SqliteStore` in `src/store/mod.rs` | `6747c85` |
| Task 3 | Created integration test suite in `tests/schema_tests.rs` verifying migrations, table schemas, cascade deletions, foreign key enforcement, and WAL mode | `69e342c` |

## Key Artifacts Created / Modified

- `crates/plainpad-core/Cargo.toml`: Added `rusqlite = { version = "0.31", features = ["bundled"] }`, `uuid`, `chrono`.
- `crates/plainpad-core/src/model.rs`: Defined Core Data Model 9 entities:
  - `Note`: 17 attributes (`id`, `content`, `created`, `last_modified`, `db_index`, `is_slotted`, `slot_index`, `is_locked`, `is_archived`, `is_private`, `is_tutorial`, `soft_deleted`, `deleted_at`, `skipped_urls`, `ck_change_tag`, `ck_system_fields`), plus title derivation method.
  - `NoteMeta`: Listing metadata struct.
  - `ChecklistItem`: Checklist rows with range coordinates and vertical position.
  - `Antilink`: Shortened URL pill descriptors.
  - `Attachment`: Metadata and file reference.
  - `SyncState`: Sync diff snapshots, cloud change tags, and system fields.
- `crates/plainpad-core/src/store/schema.rs`: Sequential migration runner for versions 1 to 9 tracking `PRAGMA user_version`.
- `crates/plainpad-core/src/store/mod.rs`: `SqliteStore` with thread-safe `Arc<Mutex<Connection>>`, `StoreError`, `open(path)` and `open_in_memory()`. Re-exported legacy `FileStore` in `src/store/file.rs` for backward compatibility.
- `crates/plainpad-core/src/lib.rs`: Exposed `pub mod model;` and re-exported entities and `now_iso()`.
- `crates/plainpad-core/tests/schema_tests.rs`: Integration tests covering fresh migrations, table and column verification, foreign key cascades, and WAL mode.

## Verification Proof

```
$ cargo test --package plainpad-core --test schema_tests
running 4 tests
test test_foreign_key_constraint_enforced ... ok
test test_schema_migration_fresh ... ok
test test_foreign_key_cascade_deletion ... ok
test test_wal_and_pragmas ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s

$ cargo test --package plainpad-core --lib
test result: ok. 41 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.29s
```

## Deviations & Notes

- Preserved existing `FileStore` code by moving to `src/store/file.rs` and re-exporting from `store::mod`, ensuring zero regressions in existing tests while introducing `SqliteStore` alongside it for subsequent plan transitions.

## Next Steps

Proceed to Plan 1.2 (`02-PLAN.md`): Note lifecycle CRUD, stack ordering (`db_index`), lock protection, archiving, and The Void soft-delete lifecycle.
