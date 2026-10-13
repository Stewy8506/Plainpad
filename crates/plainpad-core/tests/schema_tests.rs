use plainpad_core::model::now_iso;
use plainpad_core::store::SqliteStore;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

fn tmp_db_path(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("plainpad-schema-test-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir.join("test.sqlite")
}

#[test]
fn test_schema_migration_fresh() {
    let store = SqliteStore::open_in_memory().expect("Failed to open in-memory store");

    store
        .with_conn(|conn| {
            // 1. Verify PRAGMA user_version == 9
            let version: i32 = conn
                .query_row("PRAGMA user_version;", [], |row| row.get(0))
                .expect("Failed to query user_version");
            assert_eq!(version, 9, "Expected PRAGMA user_version to be 9");

            // 2. Verify all 5 entities and app_state tables exist
            let mut stmt = conn
                .prepare("SELECT name FROM sqlite_master WHERE type='table';")
                .unwrap();
            let table_names: HashSet<String> = stmt
                .query_map([], |row| row.get(0))
                .unwrap()
                .filter_map(Result::ok)
                .collect();

            let expected_tables = [
                "notes",
                "checklist_items",
                "antilinks",
                "attachments",
                "sync_state",
                "app_state",
            ];
            for table in expected_tables {
                assert!(
                    table_names.contains(table),
                    "Expected table '{}' to exist in database",
                    table
                );
            }

            // 3. Verify all 16 Note column attributes exist
            let mut stmt = conn.prepare("PRAGMA table_info(notes);").unwrap();
            let note_columns: HashSet<String> = stmt
                .query_map([], |row| row.get(1))
                .unwrap()
                .filter_map(Result::ok)
                .collect();

            let expected_note_columns = [
                "id",
                "content",
                "created",
                "last_modified",
                "db_index",
                "is_slotted",
                "slot_index",
                "skipped_urls",
                "soft_deleted",
                "ck_change_tag",
                "is_tutorial",
                "deleted_at",
                "ck_system_fields",
                "is_locked",
                "is_archived",
                "is_private",
            ];
            for col in expected_note_columns {
                assert!(
                    note_columns.contains(col),
                    "Expected column '{}' in notes table",
                    col
                );
            }

            // 4. Verify checklist_items columns
            let mut stmt = conn.prepare("PRAGMA table_info(checklist_items);").unwrap();
            let checklist_columns: HashSet<String> = stmt
                .query_map([], |row| row.get(1))
                .unwrap()
                .filter_map(Result::ok)
                .collect();
            for col in [
                "id",
                "note_id",
                "content",
                "checked",
                "index",
                "vertical_position",
                "responsibility_range_location",
                "responsibility_range_length",
                "is_hidden",
            ] {
                assert!(
                    checklist_columns.contains(col),
                    "Expected column '{}' in checklist_items",
                    col
                );
            }

            // 5. Verify antilinks columns
            let mut stmt = conn.prepare("PRAGMA table_info(antilinks);").unwrap();
            let antilink_columns: HashSet<String> = stmt
                .query_map([], |row| row.get(1))
                .unwrap()
                .filter_map(Result::ok)
                .collect();
            for col in [
                "id",
                "note_id",
                "full_url",
                "shortened_url",
                "current_range_location",
                "current_range_length",
            ] {
                assert!(
                    antilink_columns.contains(col),
                    "Expected column '{}' in antilinks",
                    col
                );
            }

            // 6. Verify attachments columns
            let mut stmt = conn.prepare("PRAGMA table_info(attachments);").unwrap();
            let attachment_columns: HashSet<String> = stmt
                .query_map([], |row| row.get(1))
                .unwrap()
                .filter_map(Result::ok)
                .collect();
            for col in [
                "id",
                "note_id",
                "file_name",
                "file_url",
                "file_size",
                "file_type",
                "mime_type",
                "location_type",
                "sort_order",
                "created",
            ] {
                assert!(
                    attachment_columns.contains(col),
                    "Expected column '{}' in attachments",
                    col
                );
            }

            // 7. Verify sync_state columns
            let mut stmt = conn.prepare("PRAGMA table_info(sync_state);").unwrap();
            let sync_columns: HashSet<String> = stmt
                .query_map([], |row| row.get(1))
                .unwrap()
                .filter_map(Result::ok)
                .collect();
            for col in [
                "note_id",
                "base_content",
                "is_positioned",
                "pending_edit_snapshot",
                "system_fields",
                "last_synced_at",
                "needs_upload",
                "change_tag",
            ] {
                assert!(
                    sync_columns.contains(col),
                    "Expected column '{}' in sync_state",
                    col
                );
            }

            Ok(())
        })
        .unwrap();
}

#[test]
fn test_foreign_key_cascade_deletion() {
    let store = SqliteStore::open_in_memory().expect("Failed to open in-memory store");

    store
        .with_conn_mut(|conn| {
            let now = now_iso();
            let note_id = "test-cascade-note-1";

            // Insert parent note
            conn.execute(
                "INSERT INTO notes (id, content, created, last_modified) VALUES (?1, ?2, ?3, ?4);",
                rusqlite::params![note_id, "Parent note text", now, now],
            )
            .unwrap();

            // Insert child checklist item
            conn.execute(
                "INSERT INTO checklist_items (id, note_id, content, checked) VALUES (?1, ?2, ?3, 0);",
                rusqlite::params!["item-1", note_id, "Buy coffee"],
            )
            .unwrap();

            // Insert child antilink
            conn.execute(
                "INSERT INTO antilinks (id, note_id, full_url, shortened_url) VALUES (?1, ?2, ?3, ?4);",
                rusqlite::params!["link-1", note_id, "https://github.com", "github.com"],
            )
            .unwrap();

            // Insert child attachment
            conn.execute(
                "INSERT INTO attachments (id, note_id, file_name, file_url, created) VALUES (?1, ?2, ?3, ?4, ?5);",
                rusqlite::params!["att-1", note_id, "specs.pdf", "/files/specs.pdf", now],
            )
            .unwrap();

            // Insert child sync_state
            conn.execute(
                "INSERT INTO sync_state (note_id, base_content) VALUES (?1, ?2);",
                rusqlite::params![note_id, "Base content"],
            )
            .unwrap();

            // Assert row counts before delete
            let checklist_count: i64 = conn
                .query_row("SELECT COUNT(*) FROM checklist_items WHERE note_id = ?1;", [note_id], |r| r.get(0))
                .unwrap();
            let antilink_count: i64 = conn
                .query_row("SELECT COUNT(*) FROM antilinks WHERE note_id = ?1;", [note_id], |r| r.get(0))
                .unwrap();
            let attachment_count: i64 = conn
                .query_row("SELECT COUNT(*) FROM attachments WHERE note_id = ?1;", [note_id], |r| r.get(0))
                .unwrap();
            let sync_count: i64 = conn
                .query_row("SELECT COUNT(*) FROM sync_state WHERE note_id = ?1;", [note_id], |r| r.get(0))
                .unwrap();

            assert_eq!(checklist_count, 1);
            assert_eq!(antilink_count, 1);
            assert_eq!(attachment_count, 1);
            assert_eq!(sync_count, 1);

            // Delete the parent note
            let deleted = conn
                .execute("DELETE FROM notes WHERE id = ?1;", [note_id])
                .unwrap();
            assert_eq!(deleted, 1, "Expected 1 note to be deleted");

            // Assert cascade deletion removed all child entities
            let checklist_after: i64 = conn
                .query_row("SELECT COUNT(*) FROM checklist_items WHERE note_id = ?1;", [note_id], |r| r.get(0))
                .unwrap();
            let antilink_after: i64 = conn
                .query_row("SELECT COUNT(*) FROM antilinks WHERE note_id = ?1;", [note_id], |r| r.get(0))
                .unwrap();
            let attachment_after: i64 = conn
                .query_row("SELECT COUNT(*) FROM attachments WHERE note_id = ?1;", [note_id], |r| r.get(0))
                .unwrap();
            let sync_after: i64 = conn
                .query_row("SELECT COUNT(*) FROM sync_state WHERE note_id = ?1;", [note_id], |r| r.get(0))
                .unwrap();

            assert_eq!(checklist_after, 0, "Checklist items must cascade delete");
            assert_eq!(antilink_after, 0, "Antilinks must cascade delete");
            assert_eq!(attachment_after, 0, "Attachments must cascade delete");
            assert_eq!(sync_after, 0, "SyncState must cascade delete");

            Ok(())
        })
        .unwrap();
}

#[test]
fn test_foreign_key_constraint_enforced() {
    let store = SqliteStore::open_in_memory().expect("Failed to open in-memory store");

    store
        .with_conn_mut(|conn| {
            // Attempt inserting a checklist item with a non-existent parent note_id
            let res = conn.execute(
                "INSERT INTO checklist_items (id, note_id, content) VALUES (?1, ?2, ?3);",
                rusqlite::params!["orphan-item", "non-existent-note-id", "Orphan content"],
            );

            assert!(res.is_err(), "Foreign key violation must reject insert");
            if let Err(rusqlite::Error::SqliteFailure(err, _)) = res {
                assert_eq!(
                    err.extended_code,
                    787, // SQLITE_CONSTRAINT_FOREIGNKEY
                    "Expected SQLITE_CONSTRAINT_FOREIGNKEY error"
                );
            } else {
                panic!("Expected SQLite constraint failure error");
            }

            Ok(())
        })
        .unwrap();
}

#[test]
fn test_wal_and_pragmas() {
    let db_path = tmp_db_path("wal-test");
    let store = SqliteStore::open(&db_path).expect("Failed to open on-disk store");

    store
        .with_conn(|conn| {
            let journal_mode: String = conn
                .query_row("PRAGMA journal_mode;", [], |row| row.get(0))
                .expect("Failed to get journal_mode");
            assert_eq!(
                journal_mode.to_lowercase(),
                "wal",
                "Expected WAL journal mode"
            );

            let foreign_keys: i32 = conn
                .query_row("PRAGMA foreign_keys;", [], |row| row.get(0))
                .expect("Failed to get foreign_keys");
            assert_eq!(foreign_keys, 1, "Expected foreign_keys to be ON (1)");

            Ok(())
        })
        .unwrap();

    // Clean up temporary files
    if let Some(parent) = db_path.parent() {
        let _ = fs::remove_dir_all(parent);
    }
}
