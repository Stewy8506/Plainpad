---
phase: 1
plan: 1
wave: 1
depends_on: []
files_modified:
  - crates/plainpad-core/Cargo.toml
  - crates/plainpad-core/src/model.rs
  - crates/plainpad-core/src/store/mod.rs
  - crates/plainpad-core/src/store/schema.rs
  - crates/plainpad-core/src/lib.rs
  - crates/plainpad-core/tests/schema_tests.rs
autonomous: true
gap_closure: false
must_haves:
  truths:
    - "SQLite database initializes with PRAGMA foreign_keys = ON and WAL journal mode"
    - "Core Data Model 9 schema contains all 5 entities and all 17 Note attributes"
    - "Schema migrations 1 through 9 apply sequentially up to PRAGMA user_version = 9"
    - "Cascade deletion drops checklist items, antilinks, attachments, and sync state when parent note is deleted"
  artifacts:
    - "crates/plainpad-core/src/model.rs"
    - "crates/plainpad-core/src/store/schema.rs"
    - "crates/plainpad-core/src/store/mod.rs"
    - "crates/plainpad-core/tests/schema_tests.rs"
---

# Plan 1.1: Core Data Model 9 SQLite Schema & Migrations

<objective>
Establish the embedded SQLite engine in `plainpad-core` reproducing Antinote's Core Data Model 9 schema and migration history (from Model 1 initial release to Model 9). Define strongly typed Rust structs for all 5 Core Data entities (`Note` with 17 attributes, `ChecklistItem`, `Antilink`, `Attachment`, `SyncState`) and build the sequential migration runner.

Purpose: Antinote's persistent storage relies on Core Data Model 9 with strict foreign key relationships and cascade deletion. Plainpad must replicate this data model in embedded SQLite (`rusqlite`) with WAL mode and foreign key constraints.
Output: Rust entity models, SQLite migration runner (versions 1-9), `SqliteStore` initialization, and automated schema tests.
</objective>

<context>
Load these files for context:
- .gsd/SPEC.md (Section 2: Complete Reverse-Engineered Data Model Specification)
- .gsd/findings.md (Section 3: Data Model Architecture, Core Data Model 9 Evolution)
- crates/plainpad-core/Cargo.toml
- crates/plainpad-core/src/lib.rs
</context>

<tasks>

<task type="auto">
  <name>Define Core Data Model 9 data entities and add rusqlite dependency</name>
  <files>
    crates/plainpad-core/Cargo.toml
    crates/plainpad-core/src/model.rs
    crates/plainpad-core/src/lib.rs
  </files>
  <action>
    1. Add `rusqlite = { version = "0.31", features = ["bundled"] }`, `uuid = { version = "1", features = ["v4", "serde"] }`, and `chrono = { version = "0.4", features = ["serde"] }` to `crates/plainpad-core/Cargo.toml`.
    2. Create `crates/plainpad-core/src/model.rs` defining the 5 entities from Antinote Core Data Model 9:
       - `Note`:
         - `id: String` (UUID)
         - `content: String` (default `""`)
         - `created: String` (ISO 8601 UTC timestamp)
         - `last_modified: String` (ISO 8601 UTC timestamp)
         - `db_index: i32` (ordering index)
         - `is_slotted: bool` (default `false`)
         - `slot_index: i32` (range `-1..8`, default `-1`)
         - `is_locked: bool` (default `false`)
         - `is_archived: bool` (default `false`)
         - `is_private: bool` (default `false`)
         - `is_tutorial: bool` (default `false`)
         - `soft_deleted: bool` (default `false`)
         - `deleted_at: Option<String>` (default `None`)
         - `skipped_urls: Vec<String>` (default `[]`, stored as JSON string in SQLite)
         - `ck_change_tag: Option<String>`
         - `ck_system_fields: Option<Vec<u8>>`
         - Method `pub fn title(&self) -> String` deriving title from first non-empty line (max 60 chars)
       - `ChecklistItem`:
         - `id: String`, `note_id: String`, `content: String`, `checked: bool`, `index: i32`, `vertical_position: f64`, `responsibility_range_location: i32`, `responsibility_range_length: i32`, `is_hidden: bool`
       - `Antilink`:
         - `id: String`, `note_id: String`, `full_url: String`, `shortened_url: String`, `current_range_location: i32`, `current_range_length: i32`
       - `Attachment`:
         - `id: String`, `note_id: String`, `file_name: String`, `file_url: String`, `file_size: i64`, `file_type: String`, `mime_type: String`, `location_type: String`, `sort_order: i32`, `created: String`
       - `SyncState`:
         - `note_id: String`, `base_content: String`, `is_positioned: bool`, `pending_edit_snapshot: Option<Vec<u8>>`, `system_fields: Option<Vec<u8>>`, `last_synced_at: Option<String>`, `needs_upload: bool`, `change_tag: Option<String>`
       - Update or provide compatible `NoteMeta` struct with fields: `id`, `title`, `last_modified`, `db_index`, `is_slotted`, `slot_index`, `is_locked`, `is_archived`, `is_private`, `is_tutorial`, `soft_deleted`, `deleted_at`.
    3. In `crates/plainpad-core/src/lib.rs`, declare `pub mod model;` and re-export the entities and `now_iso()` timestamp helper.

    AVOID: using untyped JSON blobs or omitting any of the 17 Note attributes from SPEC §2.1.
    USE: strict Rust types with Serde derives and defaults matching Antinote specification.
  </action>
  <verify>cargo check --package plainpad-core</verify>
  <done>All 5 Core Data entities compile with exact attribute sets and helper methods.</done>
</task>

<task type="auto">
  <name>Implement SQLite schema migrations (Models 1 through 9) and connection initialization</name>
  <files>
    crates/plainpad-core/src/store/mod.rs
    crates/plainpad-core/src/store/schema.rs
  </files>
  <action>
    1. Create `crates/plainpad-core/src/store/schema.rs` with a versioned migration runner tracking `PRAGMA user_version`.
    2. Implement sequential migrations replicating Antinote's Core Data evolution history:
       - Version 1 (Initial schema):
         `CREATE TABLE IF NOT EXISTS notes (id TEXT PRIMARY KEY, content TEXT NOT NULL DEFAULT '', created TEXT NOT NULL, last_modified TEXT NOT NULL, db_index INTEGER NOT NULL DEFAULT 0, is_slotted INTEGER NOT NULL DEFAULT 0, slot_index INTEGER NOT NULL DEFAULT -1, skipped_urls TEXT NOT NULL DEFAULT '[]');`
         `CREATE TABLE IF NOT EXISTS checklist_items (id TEXT PRIMARY KEY, note_id TEXT NOT NULL, content TEXT NOT NULL DEFAULT '', checked INTEGER NOT NULL DEFAULT 0, "index" INTEGER NOT NULL DEFAULT 0, vertical_position REAL NOT NULL DEFAULT 0.0, responsibility_range_location INTEGER NOT NULL DEFAULT 0, responsibility_range_length INTEGER NOT NULL DEFAULT 0, is_hidden INTEGER NOT NULL DEFAULT 0, FOREIGN KEY (note_id) REFERENCES notes(id) ON DELETE CASCADE);`
         `CREATE TABLE IF NOT EXISTS antilinks (id TEXT PRIMARY KEY, note_id TEXT NOT NULL, full_url TEXT NOT NULL, shortened_url TEXT NOT NULL, current_range_location INTEGER NOT NULL DEFAULT 0, current_range_length INTEGER NOT NULL DEFAULT 0, FOREIGN KEY (note_id) REFERENCES notes(id) ON DELETE CASCADE);`
         `CREATE TABLE IF NOT EXISTS attachments (id TEXT PRIMARY KEY, note_id TEXT NOT NULL, file_name TEXT NOT NULL, file_url TEXT NOT NULL, file_size INTEGER NOT NULL DEFAULT 0, file_type TEXT NOT NULL DEFAULT '', mime_type TEXT NOT NULL DEFAULT '', location_type TEXT NOT NULL DEFAULT 'local', sort_order INTEGER NOT NULL DEFAULT 0, created TEXT NOT NULL, FOREIGN KEY (note_id) REFERENCES notes(id) ON DELETE CASCADE);`
         `CREATE TABLE IF NOT EXISTS app_state (key TEXT PRIMARY KEY, value TEXT NOT NULL);`
       - Version 2 (`SoftDelete`):
         `ALTER TABLE notes ADD COLUMN soft_deleted INTEGER NOT NULL DEFAULT 0;`
       - Version 3 (`addCkChangeTag`):
         `ALTER TABLE notes ADD COLUMN ck_change_tag TEXT;`
       - Version 4 (`added isTutorial tag`):
         `ALTER TABLE notes ADD COLUMN is_tutorial INTEGER NOT NULL DEFAULT 0;`
       - Version 5 (`added deletedAt for Trash`):
         `ALTER TABLE notes ADD COLUMN deleted_at TEXT;`
       - Version 6 (`added ckSystemFields`):
         `ALTER TABLE notes ADD COLUMN ck_system_fields BLOB;`
       - Version 7 (`sync state entity`):
         `CREATE TABLE IF NOT EXISTS sync_state (note_id TEXT PRIMARY KEY, base_content TEXT NOT NULL DEFAULT '', is_positioned INTEGER NOT NULL DEFAULT 0, pending_edit_snapshot BLOB, system_fields BLOB, last_synced_at TEXT, needs_upload INTEGER NOT NULL DEFAULT 0, change_tag TEXT, FOREIGN KEY (note_id) REFERENCES notes(id) ON DELETE CASCADE);`
       - Version 8 (`added isLocked`):
         `ALTER TABLE notes ADD COLUMN is_locked INTEGER NOT NULL DEFAULT 0;`
       - Version 9 (`added isArchived isPrivate`):
         `ALTER TABLE notes ADD COLUMN is_archived INTEGER NOT NULL DEFAULT 0;`
         `ALTER TABLE notes ADD COLUMN is_private INTEGER NOT NULL DEFAULT 0;`
       - Indices:
         `CREATE INDEX IF NOT EXISTS idx_notes_db_index ON notes(db_index);`
         `CREATE INDEX IF NOT EXISTS idx_notes_active ON notes(soft_deleted, is_archived);`
         `CREATE INDEX IF NOT EXISTS idx_checklist_note_id ON checklist_items(note_id);`
         `CREATE INDEX IF NOT EXISTS idx_antilinks_note_id ON antilinks(note_id);`
         `CREATE INDEX IF NOT EXISTS idx_attachments_note_id ON attachments(note_id);`
    3. Create `SqliteStore` in `crates/plainpad-core/src/store/mod.rs` wrapping `std::sync::Arc<std::sync::Mutex<rusqlite::Connection>>`.
       Provide constructors:
       - `SqliteStore::open(path: &Path) -> Result<Self, StoreError>`
       - `SqliteStore::open_in_memory() -> Result<Self, StoreError>`
       Configure `PRAGMA foreign_keys = ON;` and `PRAGMA journal_mode = WAL;` (for disk DBs).
       Run migration runner on open to guarantee database reaches `user_version = 9`.

    AVOID: skipping `PRAGMA foreign_keys = ON;` because SQLite disables foreign keys by default on every new connection.
    USE: automatic migration execution up to version 9 inside a transaction per step.
  </action>
  <verify>cargo check --package plainpad-core</verify>
  <done>SqliteStore opens disk or in-memory DBs, runs migrations 1-9, and enables foreign keys and WAL mode.</done>
</task>

<task type="auto">
  <name>Create comprehensive schema migration and foreign key cascade test suite</name>
  <files>
    crates/plainpad-core/tests/schema_tests.rs
  </files>
  <action>
    Create integration test suite in `crates/plainpad-core/tests/schema_tests.rs`:
    1. `test_schema_migration_fresh`: Open in-memory store, verify `PRAGMA user_version` is 9, all 5 tables (`notes`, `checklist_items`, `antilinks`, `attachments`, `sync_state`) and `app_state` exist, and column names match all 17 Note attributes.
    2. `test_foreign_key_cascade_deletion`: Insert a parent note and child rows in `checklist_items`, `antilinks`, `attachments`, and `sync_state`. Delete parent note; assert that all child rows are automatically removed by foreign key cascade delete.
    3. `test_foreign_key_constraint_enforced`: Attempt inserting a `checklist_item` referencing non-existent `note_id`; assert that insertion fails with foreign key violation error.
    4. `test_wal_and_pragmas`: Open on-disk store in a temporary directory, verify `PRAGMA journal_mode` is `wal` and `PRAGMA foreign_keys` is 1.

    AVOID: ignoring cascade verification because orphaned child rows cause data inconsistency.
    USE: assertions on row counts across all child tables before and after parent note deletion.
  </action>
  <verify>cargo test --package plainpad-core --test schema_tests</verify>
  <done>All schema tests pass with 0 failures, proving Model 9 schema and cascade constraints.</done>
</task>

</tasks>

<verification>
After all tasks, verify:
- [ ] `cargo test --package plainpad-core --test schema_tests` passes with 0 errors
- [ ] SQLite schema matches Core Data Model 9 (5 entities, 17 Note attributes)
- [ ] Foreign keys and WAL mode are verified active
</verification>

<success_criteria>
- [ ] All tasks verified
- [ ] Must-haves confirmed
- [ ] No compilation warnings or regressions in plainpad-core
</success_criteria>
