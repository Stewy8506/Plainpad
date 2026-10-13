//! Relational child entity operations for ChecklistItem, Antilink, Attachment, and SyncState.

use crate::model::{Antilink, Attachment, ChecklistItem, SyncState};
use crate::store::{SqliteStore, StoreError};

impl SqliteStore {
    // -------------------------------------------------------------------------
    // Checklist Items
    // -------------------------------------------------------------------------

    /// Retrieves all checklist items for a note, ordered by `"index" ASC`.
    pub fn get_checklist_items(&self, note_id: &str) -> Result<Vec<ChecklistItem>, StoreError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, note_id, content, checked, \"index\",
                        vertical_position, responsibility_range_location,
                        responsibility_range_length, is_hidden
                 FROM checklist_items
                 WHERE note_id = ?1
                 ORDER BY \"index\" ASC",
            )?;

            let rows = stmt.query_map([note_id], |row| {
                Ok(ChecklistItem {
                    id: row.get(0)?,
                    note_id: row.get(1)?,
                    content: row.get(2)?,
                    checked: row.get::<_, i32>(3)? != 0,
                    index: row.get(4)?,
                    vertical_position: row.get(5)?,
                    responsibility_range_location: row.get(6)?,
                    responsibility_range_length: row.get(7)?,
                    is_hidden: row.get::<_, i32>(8)? != 0,
                })
            })?;

            let mut items = Vec::new();
            for r in rows {
                items.push(r?);
            }
            Ok(items)
        })
    }

    /// Replaces all checklist items for a given note inside an atomic transaction.
    pub fn replace_checklist_items(
        &self,
        note_id: &str,
        items: &[ChecklistItem],
    ) -> Result<(), StoreError> {
        self.with_conn_mut(|conn| {
            let tx = conn.transaction()?;
            tx.execute(
                "DELETE FROM checklist_items WHERE note_id = ?1",
                [note_id],
            )?;

            {
                let mut stmt = tx.prepare(
                    "INSERT INTO checklist_items (
                        id, note_id, content, checked, \"index\",
                        vertical_position, responsibility_range_location,
                        responsibility_range_length, is_hidden
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
                )?;

                for item in items {
                    let id = if item.id.is_empty() {
                        uuid::Uuid::new_v4().to_string()
                    } else {
                        item.id.clone()
                    };

                    stmt.execute(rusqlite::params![
                        id,
                        note_id,
                        item.content,
                        item.checked as i32,
                        item.index,
                        item.vertical_position,
                        item.responsibility_range_location,
                        item.responsibility_range_length,
                        item.is_hidden as i32,
                    ])?;
                }
            }

            tx.commit()?;
            Ok(())
        })
    }

    /// Toggles or sets the checked status for an individual checklist item.
    pub fn toggle_checklist_item(&self, item_id: &str, checked: bool) -> Result<(), StoreError> {
        self.with_conn(|conn| {
            let affected = conn.execute(
                "UPDATE checklist_items SET checked = ?1 WHERE id = ?2",
                rusqlite::params![checked as i32, item_id],
            )?;
            if affected == 0 {
                return Err(StoreError::NotFound(item_id.to_string()));
            }
            Ok(())
        })
    }

    // -------------------------------------------------------------------------
    // Antilinks
    // -------------------------------------------------------------------------

    /// Retrieves all antilinks for a note.
    pub fn get_antilinks(&self, note_id: &str) -> Result<Vec<Antilink>, StoreError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, note_id, full_url, shortened_url,
                        current_range_location, current_range_length
                 FROM antilinks
                 WHERE note_id = ?1",
            )?;

            let rows = stmt.query_map([note_id], |row| {
                Ok(Antilink {
                    id: row.get(0)?,
                    note_id: row.get(1)?,
                    full_url: row.get(2)?,
                    shortened_url: row.get(3)?,
                    current_range_location: row.get(4)?,
                    current_range_length: row.get(5)?,
                })
            })?;

            let mut links = Vec::new();
            for r in rows {
                links.push(r?);
            }
            Ok(links)
        })
    }

    /// Replaces all antilink records for a note in an atomic transaction.
    pub fn save_antilinks(&self, note_id: &str, links: &[Antilink]) -> Result<(), StoreError> {
        self.with_conn_mut(|conn| {
            let tx = conn.transaction()?;
            tx.execute("DELETE FROM antilinks WHERE note_id = ?1", [note_id])?;

            {
                let mut stmt = tx.prepare(
                    "INSERT INTO antilinks (
                        id, note_id, full_url, shortened_url,
                        current_range_location, current_range_length
                    ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                )?;

                for link in links {
                    let id = if link.id.is_empty() {
                        uuid::Uuid::new_v4().to_string()
                    } else {
                        link.id.clone()
                    };

                    stmt.execute(rusqlite::params![
                        id,
                        note_id,
                        link.full_url,
                        link.shortened_url,
                        link.current_range_location,
                        link.current_range_length,
                    ])?;
                }
            }

            tx.commit()?;
            Ok(())
        })
    }

    // -------------------------------------------------------------------------
    // Attachments
    // -------------------------------------------------------------------------

    /// Retrieves all attachments for a note, ordered by `sort_order ASC`.
    pub fn get_attachments(&self, note_id: &str) -> Result<Vec<Attachment>, StoreError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id, note_id, file_name, file_url, file_size,
                        file_type, mime_type, location_type, sort_order, created
                 FROM attachments
                 WHERE note_id = ?1
                 ORDER BY sort_order ASC",
            )?;

            let rows = stmt.query_map([note_id], |row| {
                Ok(Attachment {
                    id: row.get(0)?,
                    note_id: row.get(1)?,
                    file_name: row.get(2)?,
                    file_url: row.get(3)?,
                    file_size: row.get(4)?,
                    file_type: row.get(5)?,
                    mime_type: row.get(6)?,
                    location_type: row.get(7)?,
                    sort_order: row.get(8)?,
                    created: row.get(9)?,
                })
            })?;

            let mut attachments = Vec::new();
            for r in rows {
                attachments.push(r?);
            }
            Ok(attachments)
        })
    }

    /// Adds a new attachment record.
    pub fn add_attachment(&self, attachment: &Attachment) -> Result<(), StoreError> {
        let id = if attachment.id.is_empty() {
            uuid::Uuid::new_v4().to_string()
        } else {
            attachment.id.clone()
        };

        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO attachments (
                    id, note_id, file_name, file_url, file_size,
                    file_type, mime_type, location_type, sort_order, created
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                rusqlite::params![
                    id,
                    attachment.note_id,
                    attachment.file_name,
                    attachment.file_url,
                    attachment.file_size,
                    attachment.file_type,
                    attachment.mime_type,
                    attachment.location_type,
                    attachment.sort_order,
                    attachment.created,
                ],
            )?;
            Ok(())
        })
    }

    /// Deletes a specific attachment by ID.
    pub fn delete_attachment(&self, id: &str) -> Result<(), StoreError> {
        self.with_conn(|conn| {
            let affected = conn.execute("DELETE FROM attachments WHERE id = ?1", [id])?;
            if affected == 0 {
                return Err(StoreError::NotFound(id.to_string()));
            }
            Ok(())
        })
    }

    // -------------------------------------------------------------------------
    // Sync State
    // -------------------------------------------------------------------------

    /// Retrieves sync state for a note.
    pub fn get_sync_state(&self, note_id: &str) -> Result<Option<SyncState>, StoreError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare(
                "SELECT note_id, base_content, is_positioned,
                        pending_edit_snapshot, system_fields, last_synced_at,
                        needs_upload, change_tag
                 FROM sync_state
                 WHERE note_id = ?1",
            )?;

            let mut rows = stmt.query([note_id])?;
            if let Some(row) = rows.next()? {
                Ok(Some(SyncState {
                    note_id: row.get(0)?,
                    base_content: row.get(1)?,
                    is_positioned: row.get::<_, i32>(2)? != 0,
                    pending_edit_snapshot: row.get(3)?,
                    system_fields: row.get(4)?,
                    last_synced_at: row.get(5)?,
                    needs_upload: row.get::<_, i32>(6)? != 0,
                    change_tag: row.get(7)?,
                }))
            } else {
                Ok(None)
            }
        })
    }

    /// Sets or updates the sync state for a note.
    pub fn set_sync_state(&self, state: &SyncState) -> Result<(), StoreError> {
        self.with_conn(|conn| {
            conn.execute(
                "INSERT OR REPLACE INTO sync_state (
                    note_id, base_content, is_positioned, pending_edit_snapshot,
                    system_fields, last_synced_at, needs_upload, change_tag
                ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                rusqlite::params![
                    state.note_id,
                    state.base_content,
                    state.is_positioned as i32,
                    state.pending_edit_snapshot,
                    state.system_fields,
                    state.last_synced_at,
                    state.needs_upload as i32,
                    state.change_tag,
                ],
            )?;
            Ok(())
        })
    }
}
