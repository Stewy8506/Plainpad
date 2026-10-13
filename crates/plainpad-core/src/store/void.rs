//! The Void: soft-delete, restore, retention pruning, and permanent purge engine.

use crate::model::{now_iso, Note};
use crate::store::notes::{parse_note_row, NOTE_COLUMNS};
use crate::store::{SqliteStore, StoreError};

impl SqliteStore {
    /// Soft-deletes a note into The Void.
    ///
    /// Fails if:
    /// - Note is slotted (`is_slotted == true`): Slotted notes are immune to deletion (SPEC §3).
    /// - Note is locked (`is_locked == true`): Note must be unlocked first.
    pub fn soft_delete_note(&self, id: &str) -> Result<(), StoreError> {
        let note = self
            .load_note(id)?
            .ok_or_else(|| StoreError::NotFound(id.to_string()))?;

        if note.is_slotted {
            return Err(StoreError::SlottedNoteCannotBeDeleted);
        }
        if note.is_locked {
            return Err(StoreError::NoteLocked);
        }

        let now = now_iso();
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE notes SET soft_deleted = 1, deleted_at = ?1, last_modified = ?2 WHERE id = ?3",
                rusqlite::params![&now, &now, id],
            )?;
            Ok(())
        })
    }

    /// Restores a soft-deleted note from The Void back into active notes.
    ///
    /// Assigns `db_index = MAX(active db_index) + 1` to place it at the front of the active stack.
    pub fn restore_from_void(&self, id: &str) -> Result<Note, StoreError> {
        let note = self
            .load_note(id)?
            .ok_or_else(|| StoreError::NotFound(id.to_string()))?;

        if !note.soft_deleted {
            return Err(StoreError::InvalidOperation(format!(
                "Note {id} is not in The Void"
            )));
        }

        let now = now_iso();
        self.with_conn_mut(|conn| {
            let tx = conn.transaction()?;
            let next_index: i32 = tx.query_row(
                "SELECT COALESCE(MAX(db_index), -1) + 1 FROM notes WHERE soft_deleted = 0 AND is_slotted = 0",
                [],
                |row| row.get(0),
            )?;
            tx.execute(
                "UPDATE notes SET soft_deleted = 0, deleted_at = NULL, db_index = ?1, last_modified = ?2 WHERE id = ?3",
                rusqlite::params![next_index, &now, id],
            )?;
            tx.commit()?;
            Ok(())
        })?;

        let updated = self
            .load_note(id)?
            .ok_or_else(|| StoreError::NotFound(id.to_string()))?;
        Ok(updated)
    }

    /// Lists all soft-deleted notes currently in The Void, ordered by `deleted_at DESC`.
    pub fn list_void(&self) -> Result<Vec<Note>, StoreError> {
        self.with_conn(|conn| {
            let sql = format!(
                "SELECT {NOTE_COLUMNS} FROM notes WHERE soft_deleted = 1 ORDER BY deleted_at DESC"
            );
            let mut stmt = conn.prepare(&sql)?;
            let rows = stmt.query_map([], |row| parse_note_row(row))?;
            let mut notes = Vec::new();
            for n in rows {
                notes.push(n?);
            }
            Ok(notes)
        })
    }

    /// Permanently deletes all notes in The Void.
    ///
    /// Cascades foreign keys to child tables (`checklist_items`, `antilinks`, `attachments`, `sync_state`).
    /// Returns the count of deleted notes.
    pub fn purge_void(&self) -> Result<usize, StoreError> {
        self.with_conn(|conn| {
            let count = conn.execute("DELETE FROM notes WHERE soft_deleted = 1", [])?;
            Ok(count)
        })
    }

    /// Permanently deletes a single note from The Void.
    ///
    /// Fails if the note does not exist or is not in The Void.
    pub fn permanently_delete_note(&self, id: &str) -> Result<(), StoreError> {
        let note = self
            .load_note(id)?
            .ok_or_else(|| StoreError::NotFound(id.to_string()))?;

        if !note.soft_deleted {
            return Err(StoreError::InvalidOperation(format!(
                "Note {id} is not in The Void"
            )));
        }

        self.with_conn(|conn| {
            conn.execute(
                "DELETE FROM notes WHERE id = ?1 AND soft_deleted = 1",
                [id],
            )?;
            Ok(())
        })
    }

    /// Purges notes from The Void that have been deleted longer than `retention_days`.
    ///
    /// Returns the count of purged notes.
    pub fn purge_expired_void(&self, retention_days: u32) -> Result<usize, StoreError> {
        let cutoff = (chrono::Utc::now() - chrono::Duration::days(retention_days as i64))
            .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);

        self.with_conn(|conn| {
            let count = conn.execute(
                "DELETE FROM notes WHERE soft_deleted = 1 AND deleted_at < ?1",
                [&cutoff],
            )?;
            Ok(count)
        })
    }
}
