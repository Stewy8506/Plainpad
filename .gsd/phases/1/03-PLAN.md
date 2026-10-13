---
phase: 1
plan: 3
wave: 3
depends_on: ["02"]
files_modified:
  - crates/plainpad-core/src/store/relational.rs
  - crates/plainpad-core/src/store/snapshot.rs
  - crates/plainpad-core/src/store/mod.rs
  - src-tauri/Cargo.toml
  - src-tauri/src/main.rs
  - crates/plainpad-core/tests/relational_and_snapshot_tests.rs
autonomous: true
gap_closure: false
must_haves:
  truths:
    - "Checklist items, antilinks, attachments, and sync state have complete CRUD operations with cascade deletion"
    - "Pre-restore safety snapshots are automatically generated before restoring any backup"
    - "Restoration from snapshot enables sync isolation (sync_isolated: true) to prevent premature cloud propagation"
    - "Tauri IPC commands wire SqliteStore into the desktop application, replacing FileStore"
  artifacts:
    - "crates/plainpad-core/src/store/relational.rs"
    - "crates/plainpad-core/src/store/snapshot.rs"
    - "src-tauri/src/main.rs"
    - "crates/plainpad-core/tests/relational_and_snapshot_tests.rs"
---

# Plan 1.3: Relational Store, Pre-Restore Snapshots & Tauri IPC Integration

<objective>
Implement relational child store operations (`ChecklistItem`, `Antilink`, `Attachment`, `SyncState`), the pre-restore safety snapshot manager (`BackupRestoreSheet` parity with sync isolation), and wire `SqliteStore` into the Tauri backend (`src-tauri/src/main.rs`) replacing `FileStore`.

Purpose: In Antinote, rich elements (inline checklists, antilinks, attachments) and sync state are relational child records with cascade deletion, while the backup/restore engine automatically creates pre-restore safety snapshots and decouples sync. This plan completes the SQLite backend and integrates it into the desktop application.
Output: `relational.rs`, `snapshot.rs`, updated `src-tauri/src/main.rs`, and test suite in `relational_and_snapshot_tests.rs`.
</objective>

<context>
Load these files for context:
- .gsd/SPEC.md (Section 2: Entities 2.2-2.5, Section 9: Safety Backup & Restore Lifecycle)
- .gsd/findings.md (Section 16.2: Backup & Restore Lifecycle)
- .gsd/phases/1/02-PLAN.md
- src-tauri/src/main.rs
</context>

<tasks>

<task type="auto">
  <name>Implement relational child entity operations (Checklists, Antilinks, Attachments, SyncState)</name>
  <files>
    crates/plainpad-core/src/store/relational.rs
    crates/plainpad-core/src/store/mod.rs
  </files>
  <action>
    Create `crates/plainpad-core/src/store/relational.rs` implementing relational child entity queries and mutations for `SqliteStore`:
    1. Checklist Items (`ChecklistItem`):
       - `get_checklist_items(&self, note_id: &str) -> Result<Vec<ChecklistItem>, StoreError>`: Queries `checklist_items` ordered by `"index" ASC`.
       - `replace_checklist_items(&self, note_id: &str, items: &[ChecklistItem]) -> Result<(), StoreError>`: Runs a transaction deleting existing items for `note_id` and inserting new items with their layout offset and responsibility ranges.
       - `toggle_checklist_item(&self, item_id: &str, checked: bool) -> Result<(), StoreError>`: Updates `checked` flag for individual item.
    2. Antilinks (`Antilink`):
       - `get_antilinks(&self, note_id: &str) -> Result<Vec<Antilink>, StoreError>`: Queries `antilinks` for note.
       - `save_antilinks(&self, note_id: &str, links: &[Antilink]) -> Result<(), StoreError>`: Replaces antilink records for note in a transaction.
    3. Attachments (`Attachment`):
       - `get_attachments(&self, note_id: &str) -> Result<Vec<Attachment>, StoreError>`: Queries `attachments` ordered by `sort_order ASC`.
       - `add_attachment(&self, attachment: &Attachment) -> Result<(), StoreError>`: Inserts attachment record.
       - `delete_attachment(&self, id: &str) -> Result<(), StoreError>`: Deletes specific attachment.
    4. Sync State (`SyncState`):
       - `get_sync_state(&self, note_id: &str) -> Result<Option<SyncState>, StoreError>`: Queries `sync_state` for note.
       - `set_sync_state(&self, state: &SyncState) -> Result<(), StoreError>`: Upserts sync state row using `INSERT OR REPLACE INTO sync_state ...`.

    AVOID: orphaned child rows when notes are updated or replaced.
    USE: atomic transactions with foreign key cascade guarantees.
  </action>
  <verify>cargo check --package plainpad-core</verify>
  <done>Relational operations for checklists, antilinks, attachments, and sync_state compile with full error handling.</done>
</task>

<task type="auto">
  <name>Implement Pre-Restore Safety Snapshot manager with sync isolation</name>
  <files>
    crates/plainpad-core/src/store/snapshot.rs
    crates/plainpad-core/src/store/mod.rs
    crates/plainpad-core/tests/relational_and_snapshot_tests.rs
  </files>
  <action>
    Create `crates/plainpad-core/src/store/snapshot.rs` implementing pre-restore snapshotting:
    1. Define `SnapshotMeta`:
       - `path: PathBuf`, `timestamp: String`, `reason: String`, `size_bytes: u64`.
    2. `create_pre_restore_snapshot(&self, reason: &str) -> Result<PathBuf, StoreError>`:
       - Determine snapshot folder `<root>/snapshots/`.
       - Generate filename `snapshot_<timestamp_ms>_<sanitized_reason>.db`.
       - Use SQLite `VACUUM INTO ?` or online backup API on connection to produce a clean, transactionally consistent snapshot file.
       - Return path to generated snapshot.
    3. `restore_from_snapshot(&self, snapshot_path: &Path) -> Result<(), StoreError>`:
       - Automatic safety backup: Automatically call `create_pre_restore_snapshot("pre_restore_safety")` first to back up current state before performing any overwrite (`pendingBackupRestoreAnchorPath` pattern from SPEC §9 / findings §16.2).
       - Sync isolation: Write `sync_isolated = true` to `app_state` table to decouple cloud/network sync during restore evaluation.
       - Copy snapshot over active database file (or use SQLite restore API) and re-open connection.
    4. `list_snapshots(&self) -> Result<Vec<SnapshotMeta>, StoreError>`:
       - Enumerate files in `<root>/snapshots/` returning metadata sorted newest first.
    5. Write tests in `crates/plainpad-core/tests/relational_and_snapshot_tests.rs`:
       - `test_relational_crud_and_cascade`: Insert note, add 2 checklist items, 1 antilink, 1 attachment, 1 sync state. Verify retrieval. Delete note; verify all child tables are now empty.
       - `test_snapshot_and_safe_restore`:
         - Insert note with child entities into a disk store.
         - Call `create_pre_restore_snapshot("test_backup")`.
         - Modify note content and delete attachment.
         - Call `restore_from_snapshot(&snapshot_file)`.
         - Verify pre-restore safety backup was created in `snapshots/`.
         - Verify database restored to original state with original content and attachment.
         - Verify `sync_isolated` is set to `true`.

    AVOID: raw file copy while SQLite WAL log is dirty because raw file copy can lead to corrupt snapshots.
    USE: SQLite `VACUUM INTO` / backup API for transactional snapshot generation.
  </action>
  <verify>cargo test --package plainpad-core --test relational_and_snapshot_tests</verify>
  <done>Relational queries and pre-restore snapshot manager tests pass with 0 errors.</done>
</task>

<task type="auto">
  <name>Wire SqliteStore into src-tauri AppState and expose updated IPC commands</name>
  <files>
    src-tauri/Cargo.toml
    src-tauri/src/main.rs
  </files>
  <action>
    Update `src-tauri/src/main.rs` to replace `FileStore` with `SqliteStore`:
    1. In `src-tauri/Cargo.toml`, ensure `plainpad-core` dependency is available.
    2. In `src-tauri/src/main.rs`:
       - Change `AppState` field `store: plainpad_core::store::FileStore` to `store: std::sync::Arc<plainpad_core::store::SqliteStore>`.
       - In `main()`, initialize:
         `let db_path = data_dir().join("plainpad.db");`
         `let store = std::sync::Arc::new(plainpad_core::store::SqliteStore::open(&db_path).expect("Failed to initialize SQLite store"));`
       - Update existing IPC commands:
         - `list_notes`: calls `store.list_active_notes()` and returns `Vec<NoteMeta>`.
         - `load_note`: calls `store.load_note(&id)` and returns `Option<Note>`.
         - `save_note`: calls `store.save_note(&id, &text)`.
         - `create_note`: calls `store.create_note("")`.
         - `delete_note`: calls `store.soft_delete_note(&id)` (moves note to The Void).
         - `restore_note`: calls `store.restore_from_void(&id)`.
         - `list_trash`: calls `store.list_void()` and maps to `TrashEntry`.
         - `set_pinned`: calls `store.set_setting(&format!("pinned_{}", id), serde_json::Value::Bool(pinned))` or note flag.
         - `set_plain`: calls `store.set_setting(&format!("plain_{}", id), serde_json::Value::Bool(plain))`.
         - `get_settings`, `set_setting`, `get_geometry`, `set_geometry`: route to `store.get_settings()`, `store.set_setting()`, `store.get_geometry()`, `store.set_geometry()`.
       - Add new IPC commands matching Antinote Core Data Model 9 & The Void:
         - `purge_void`: calls `store.purge_void()`.
         - `set_locked`: calls `store.set_locked(&id, locked)`.
         - `set_archived`: calls `store.set_archived(&id, archived)`.
         - `set_private`: calls `store.set_private(&id, private)`.
         - `reorder_notes`: calls `store.reorder_notes(&ids)`.
         - `create_pre_restore_snapshot`: calls `store.create_pre_restore_snapshot(&reason)`.
         - `restore_from_snapshot`: calls `store.restore_from_snapshot(Path::new(&path))`.
         - Register all commands in `tauri::generate_handler![...]`.

    AVOID: breaking existing frontend IPC calls (`list_notes`, `load_note`, `save_note`, `delete_note`).
    USE: seamless adapter mapping so frontend continues functioning with full backwards compatibility while gaining new Model 9 features.
  </action>
  <verify>cargo check --manifest-path src-tauri/Cargo.toml</verify>
  <done>src-tauri compiles cleanly with SqliteStore wired into AppState and IPC commands registered.</done>
</task>

</tasks>

<verification>
After all tasks, verify:
- [ ] `cargo test --package plainpad-core --test relational_and_snapshot_tests` passes with 0 errors
- [ ] `cargo check --manifest-path src-tauri/Cargo.toml` compiles with 0 errors
- [ ] Pre-restore safety snapshots and sync isolation verified
- [ ] SqliteStore replaces FileStore throughout the application
</verification>

<success_criteria>
- [ ] All tasks verified
- [ ] Must-haves confirmed
- [ ] No compilation warnings or regressions in plainpad-core and src-tauri
</success_criteria>
