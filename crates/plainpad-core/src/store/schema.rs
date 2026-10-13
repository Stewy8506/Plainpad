//! SQLite schema migrations reproducing Antinote's Core Data Model 1 through 9.

use rusqlite::{Connection, Result as SqliteResult};

/// Active Core Data schema version (Antinote Model 9).
pub const CURRENT_SCHEMA_VERSION: i32 = 9;

/// Reads the current SQLite user_version.
pub fn get_user_version(conn: &Connection) -> SqliteResult<i32> {
    conn.query_row("PRAGMA user_version;", [], |row| row.get(0))
}

/// Sets the SQLite user_version.
pub fn set_user_version(conn: &Connection, version: i32) -> SqliteResult<()> {
    conn.execute(&format!("PRAGMA user_version = {version};"), [])?;
    Ok(())
}

/// Executes sequential migrations (1 through 9) inside transactions per version.
pub fn run_migrations(conn: &Connection) -> SqliteResult<()> {
    let mut version = get_user_version(conn)?;

    if version < 1 {
        // Model 1 (Initial schema): Note, ChecklistItem, Antilink, Attachment, AppState
        conn.execute_batch(
            "BEGIN;
            CREATE TABLE IF NOT EXISTS notes (
                id TEXT PRIMARY KEY,
                content TEXT NOT NULL DEFAULT '',
                created TEXT NOT NULL,
                last_modified TEXT NOT NULL,
                db_index INTEGER NOT NULL DEFAULT 0,
                is_slotted INTEGER NOT NULL DEFAULT 0,
                slot_index INTEGER NOT NULL DEFAULT -1,
                skipped_urls TEXT NOT NULL DEFAULT '[]'
            );

            CREATE TABLE IF NOT EXISTS checklist_items (
                id TEXT PRIMARY KEY,
                note_id TEXT NOT NULL,
                content TEXT NOT NULL DEFAULT '',
                checked INTEGER NOT NULL DEFAULT 0,
                \"index\" INTEGER NOT NULL DEFAULT 0,
                vertical_position REAL NOT NULL DEFAULT 0.0,
                responsibility_range_location INTEGER NOT NULL DEFAULT 0,
                responsibility_range_length INTEGER NOT NULL DEFAULT 0,
                is_hidden INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (note_id) REFERENCES notes(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS antilinks (
                id TEXT PRIMARY KEY,
                note_id TEXT NOT NULL,
                full_url TEXT NOT NULL,
                shortened_url TEXT NOT NULL,
                current_range_location INTEGER NOT NULL DEFAULT 0,
                current_range_length INTEGER NOT NULL DEFAULT 0,
                FOREIGN KEY (note_id) REFERENCES notes(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS attachments (
                id TEXT PRIMARY KEY,
                note_id TEXT NOT NULL,
                file_name TEXT NOT NULL,
                file_url TEXT NOT NULL,
                file_size INTEGER NOT NULL DEFAULT 0,
                file_type TEXT NOT NULL DEFAULT '',
                mime_type TEXT NOT NULL DEFAULT '',
                location_type TEXT NOT NULL DEFAULT 'local',
                sort_order INTEGER NOT NULL DEFAULT 0,
                created TEXT NOT NULL,
                FOREIGN KEY (note_id) REFERENCES notes(id) ON DELETE CASCADE
            );

            CREATE TABLE IF NOT EXISTS app_state (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            COMMIT;",
        )?;
        set_user_version(conn, 1)?;
        version = 1;
    }

    if version < 2 {
        // Model 2: SoftDelete flag added to Note
        conn.execute_batch(
            "BEGIN;
            ALTER TABLE notes ADD COLUMN soft_deleted INTEGER NOT NULL DEFAULT 0;
            COMMIT;",
        )?;
        set_user_version(conn, 2)?;
        version = 2;
    }

    if version < 3 {
        // Model 3: addCkChangeTag
        conn.execute_batch(
            "BEGIN;
            ALTER TABLE notes ADD COLUMN ck_change_tag TEXT;
            COMMIT;",
        )?;
        set_user_version(conn, 3)?;
        version = 3;
    }

    if version < 4 {
        // Model 4: added isTutorial tag
        conn.execute_batch(
            "BEGIN;
            ALTER TABLE notes ADD COLUMN is_tutorial INTEGER NOT NULL DEFAULT 0;
            COMMIT;",
        )?;
        set_user_version(conn, 4)?;
        version = 4;
    }

    if version < 5 {
        // Model 5: added deletedAt for Trash
        conn.execute_batch(
            "BEGIN;
            ALTER TABLE notes ADD COLUMN deleted_at TEXT;
            COMMIT;",
        )?;
        set_user_version(conn, 5)?;
        version = 5;
    }

    if version < 6 {
        // Model 6: added ckSystemFields
        conn.execute_batch(
            "BEGIN;
            ALTER TABLE notes ADD COLUMN ck_system_fields BLOB;
            COMMIT;",
        )?;
        set_user_version(conn, 6)?;
        version = 6;
    }

    if version < 7 {
        // Model 7: sync state entity
        conn.execute_batch(
            "BEGIN;
            CREATE TABLE IF NOT EXISTS sync_state (
                note_id TEXT PRIMARY KEY,
                base_content TEXT NOT NULL DEFAULT '',
                is_positioned INTEGER NOT NULL DEFAULT 0,
                pending_edit_snapshot BLOB,
                system_fields BLOB,
                last_synced_at TEXT,
                needs_upload INTEGER NOT NULL DEFAULT 0,
                change_tag TEXT,
                FOREIGN KEY (note_id) REFERENCES notes(id) ON DELETE CASCADE
            );
            COMMIT;",
        )?;
        set_user_version(conn, 7)?;
        version = 7;
    }

    if version < 8 {
        // Model 8: added isLocked
        conn.execute_batch(
            "BEGIN;
            ALTER TABLE notes ADD COLUMN is_locked INTEGER NOT NULL DEFAULT 0;
            COMMIT;",
        )?;
        set_user_version(conn, 8)?;
        version = 8;
    }

    if version < 9 {
        // Model 9: added isArchived isPrivate and indices
        conn.execute_batch(
            "BEGIN;
            ALTER TABLE notes ADD COLUMN is_archived INTEGER NOT NULL DEFAULT 0;
            ALTER TABLE notes ADD COLUMN is_private INTEGER NOT NULL DEFAULT 0;
            CREATE INDEX IF NOT EXISTS idx_notes_db_index ON notes(db_index);
            CREATE INDEX IF NOT EXISTS idx_notes_active ON notes(soft_deleted, is_archived);
            CREATE INDEX IF NOT EXISTS idx_checklist_note_id ON checklist_items(note_id);
            CREATE INDEX IF NOT EXISTS idx_antilinks_note_id ON antilinks(note_id);
            CREATE INDEX IF NOT EXISTS idx_attachments_note_id ON attachments(note_id);
            COMMIT;",
        )?;
        set_user_version(conn, 9)?;
    }

    Ok(())
}
