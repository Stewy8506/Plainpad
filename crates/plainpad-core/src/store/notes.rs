//! Note lifecycle CRUD, stack ordering, lock protection, archiving, and app state settings.

use crate::model::{now_iso, Note};
use crate::store::{SqliteStore, StoreError};

pub(crate) const NOTE_COLUMNS: &str = "\
    id, content, created, last_modified, db_index, \
    is_slotted, slot_index, is_locked, is_archived, is_private, \
    is_tutorial, soft_deleted, deleted_at, skipped_urls, \
    ck_change_tag, ck_system_fields";

/// The 9 Greek deities mapped to permanent scratchpad slots (0..8).
pub const GREEK_DEITIES: [&str; 9] = [
    "Hermes",
    "Apollo",
    "Ares",
    "Cronus",
    "Gaia",
    "Hades",
    "Ouranos",
    "Poseidon",
    "Zeus",
];

/// Default initial content for slot 0 (Hermes).
pub const HERMES_DEFAULT_CONTENT: &str =
    "This is your first of nine slotted notes. It'll stay with you.";

/// Returns default initial content for a given slot index (0..8).
pub fn default_content_for_slot(slot_index: i32) -> &'static str {
    match slot_index {
        0 => HERMES_DEFAULT_CONTENT,
        _ => "",
    }
}

/// Returns the Greek deity associated with a slot index (0..8).
pub fn deity_for_slot(slot_index: i32) -> Option<&'static str> {
    if (0..=8).contains(&slot_index) {
        Some(GREEK_DEITIES[slot_index as usize])
    } else {
        None
    }
}

pub(crate) fn parse_note_row(row: &rusqlite::Row) -> rusqlite::Result<Note> {
    let id: String = row.get(0)?;
    let content: String = row.get(1)?;
    let created: String = row.get(2)?;
    let last_modified: String = row.get(3)?;
    let db_index: i32 = row.get(4)?;
    let is_slotted: bool = row.get::<_, i32>(5)? != 0;
    let slot_index: i32 = row.get(6)?;
    let is_locked: bool = row.get::<_, i32>(7)? != 0;
    let is_archived: bool = row.get::<_, i32>(8)? != 0;
    let is_private: bool = row.get::<_, i32>(9)? != 0;
    let is_tutorial: bool = row.get::<_, i32>(10)? != 0;
    let soft_deleted: bool = row.get::<_, i32>(11)? != 0;
    let deleted_at: Option<String> = row.get(12)?;
    let skipped_urls_str: String = row
        .get::<_, Option<String>>(13)?
        .unwrap_or_else(|| "[]".to_string());
    let skipped_urls: Vec<String> = serde_json::from_str(&skipped_urls_str).unwrap_or_default();
    let ck_change_tag: Option<String> = row.get(14)?;
    let ck_system_fields: Option<Vec<u8>> = row.get(15)?;

    Ok(Note {
        id,
        content,
        created,
        last_modified,
        db_index,
        is_slotted,
        slot_index,
        is_locked,
        is_archived,
        is_private,
        is_tutorial,
        soft_deleted,
        deleted_at,
        skipped_urls,
        ck_change_tag,
        ck_system_fields,
    })
}

impl SqliteStore {
    /// Creates a new note with sequential stack db_index and default attributes.
    pub fn create_note(&self, content: &str) -> Result<Note, StoreError> {
        let id = uuid::Uuid::new_v4().to_string();
        let now = now_iso();
        self.with_conn_mut(|conn| {
            let tx = conn.transaction()?;
            let next_index: i32 = tx.query_row(
                "SELECT COALESCE(MAX(db_index), -1) + 1 FROM notes WHERE is_slotted = 0",
                [],
                |row| row.get(0),
            )?;
            tx.execute(
                "INSERT INTO notes (
                    id, content, created, last_modified, db_index,
                    is_slotted, slot_index, is_locked, is_archived, is_private,
                    is_tutorial, soft_deleted, deleted_at, skipped_urls,
                    ck_change_tag, ck_system_fields
                ) VALUES (
                    ?1, ?2, ?3, ?4, ?5,
                    0, -1, 0, 0, 0,
                    0, 0, NULL, '[]',
                    NULL, NULL
                )",
                rusqlite::params![&id, content, &now, &now, next_index],
            )?;
            tx.commit()?;
            Ok(Note {
                id,
                content: content.to_string(),
                created: now.clone(),
                last_modified: now,
                db_index: next_index,
                is_slotted: false,
                slot_index: -1,
                is_locked: false,
                is_archived: false,
                is_private: false,
                is_tutorial: false,
                soft_deleted: false,
                deleted_at: None,
                skipped_urls: Vec::new(),
                ck_change_tag: None,
                ck_system_fields: None,
            })
        })
    }

    /// Loads a note by ID, extracting all 17 Core Data Model attributes.
    pub fn load_note(&self, id: &str) -> Result<Option<Note>, StoreError> {
        self.with_conn(|conn| {
            let sql = format!("SELECT {NOTE_COLUMNS} FROM notes WHERE id = ?1");
            let mut stmt = conn.prepare(&sql)?;
            let mut rows = stmt.query([id])?;
            if let Some(row) = rows.next()? {
                let note = parse_note_row(row)?;
                Ok(Some(note))
            } else {
                Ok(None)
            }
        })
    }

    /// Updates content of an existing note.
    /// Fails if the note is locked (`is_locked == true`) or in The Void (`soft_deleted == true`).
    pub fn save_note(&self, id: &str, content: &str) -> Result<Note, StoreError> {
        let note = self
            .load_note(id)?
            .ok_or_else(|| StoreError::NotFound(id.to_string()))?;

        if note.is_locked {
            return Err(StoreError::NoteLocked);
        }
        if note.soft_deleted {
            return Err(StoreError::NoteInVoid);
        }

        let now = now_iso();
        self.with_conn(|conn| {
            conn.execute(
                "UPDATE notes SET content = ?1, last_modified = ?2 WHERE id = ?3",
                rusqlite::params![content, &now, id],
            )?;
            Ok(())
        })?;

        // Synchronize relational child entities (checklist items and antilinks)
        self.sync_relational_entities(id, content)?;

        let mut updated = note;
        updated.content = content.to_string();
        updated.last_modified = now;
        Ok(updated)
    }

    /// Lists active notes (excluding soft-deleted, archived, and slotted notes), ordered by `db_index ASC`.
    pub fn list_active_notes(&self) -> Result<Vec<Note>, StoreError> {
        self.with_conn(|conn| {
            let sql = format!(
                "SELECT {NOTE_COLUMNS} FROM notes WHERE soft_deleted = 0 AND is_archived = 0 AND is_slotted = 0 ORDER BY db_index ASC"
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

    /// Ensures that all 9 permanent slotted notes (Greek deities Hermes through Zeus, slot_index 0..8)
    /// exist in the store. Idempotently preserves existing user content on already created slots.
    pub fn ensure_slotted_notes(&self) -> Result<Vec<Note>, StoreError> {
        let now = now_iso();
        self.with_conn_mut(|conn| {
            let tx = conn.transaction()?;
            for slot_index in 0..=8 {
                let mut exists_stmt = tx.prepare(
                    "SELECT id FROM notes WHERE is_slotted = 1 AND slot_index = ?1",
                )?;
                let mut rows = exists_stmt.query([slot_index])?;
                let exists = rows.next()?.is_some();
                drop(rows);
                drop(exists_stmt);

                if !exists {
                    let id = uuid::Uuid::new_v4().to_string();
                    let default_text = default_content_for_slot(slot_index);
                    tx.execute(
                        "INSERT INTO notes (
                            id, content, created, last_modified, db_index,
                            is_slotted, slot_index, is_locked, is_archived, is_private,
                            is_tutorial, soft_deleted, deleted_at, skipped_urls,
                            ck_change_tag, ck_system_fields
                        ) VALUES (
                            ?1, ?2, ?3, ?4, ?5,
                            1, ?6, 0, 0, 0,
                            0, 0, NULL, '[]',
                            NULL, NULL
                        )",
                        rusqlite::params![&id, default_text, &now, &now, slot_index, slot_index],
                    )?;
                }
            }
            tx.commit()?;
            Ok(())
        })?;

        self.list_slotted_notes()
    }

    /// Lists all 9 permanent slotted notes, ordered by `slot_index ASC`.
    pub fn list_slotted_notes(&self) -> Result<Vec<Note>, StoreError> {
        self.with_conn(|conn| {
            let sql = format!(
                "SELECT {NOTE_COLUMNS} FROM notes WHERE is_slotted = 1 AND soft_deleted = 0 ORDER BY slot_index ASC"
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

    /// Retrieves a slotted note by its slot index (0..8).
    /// Returns `StoreError::InvalidOperation` if `slot_index < 0 || slot_index > 8`.
    pub fn get_slotted_note(&self, slot_index: i32) -> Result<Option<Note>, StoreError> {
        if !(0..=8).contains(&slot_index) {
            return Err(StoreError::InvalidOperation(
                "Slot index must be between 0 and 8".to_string(),
            ));
        }
        self.with_conn(|conn| {
            let sql = format!(
                "SELECT {NOTE_COLUMNS} FROM notes WHERE is_slotted = 1 AND slot_index = ?1 AND soft_deleted = 0"
            );
            let mut stmt = conn.prepare(&sql)?;
            let mut rows = stmt.query([slot_index])?;
            if let Some(row) = rows.next()? {
                let note = parse_note_row(row)?;
                Ok(Some(note))
            } else {
                Ok(None)
            }
        })
    }

    /// Updates `db_index` ordering for a sequence of note IDs in an atomic transaction.
    /// Only updates unslotted scratch notes (`is_slotted = 0`), preserving slotted notes intact.
    pub fn reorder_notes(&self, note_ids_in_order: &[String]) -> Result<(), StoreError> {
        self.with_conn_mut(|conn| {
            let tx = conn.transaction()?;
            {
                let mut stmt = tx.prepare("UPDATE notes SET db_index = ?1 WHERE id = ?2 AND is_slotted = 0")?;
                for (index, id) in note_ids_in_order.iter().enumerate() {
                    stmt.execute(rusqlite::params![index as i32, id])?;
                }
            }
            tx.commit()?;
            Ok(())
        })
    }

    /// Sets or clears lock protection (`is_locked`) on a note.
    pub fn set_locked(&self, id: &str, locked: bool) -> Result<(), StoreError> {
        let now = now_iso();
        self.with_conn(|conn| {
            let rows = conn.execute(
                "UPDATE notes SET is_locked = ?1, last_modified = ?2 WHERE id = ?3",
                rusqlite::params![locked as i32, &now, id],
            )?;
            if rows == 0 {
                return Err(StoreError::NotFound(id.to_string()));
            }
            Ok(())
        })
    }

    /// Sets or clears archive status (`is_archived`) on a note.
    pub fn set_archived(&self, id: &str, archived: bool) -> Result<(), StoreError> {
        let now = now_iso();
        self.with_conn(|conn| {
            let rows = conn.execute(
                "UPDATE notes SET is_archived = ?1, last_modified = ?2 WHERE id = ?3",
                rusqlite::params![archived as i32, &now, id],
            )?;
            if rows == 0 {
                return Err(StoreError::NotFound(id.to_string()));
            }
            Ok(())
        })
    }

    /// Sets or clears private status (`is_private`) on a note.
    pub fn set_private(&self, id: &str, private: bool) -> Result<(), StoreError> {
        let now = now_iso();
        self.with_conn(|conn| {
            let rows = conn.execute(
                "UPDATE notes SET is_private = ?1, last_modified = ?2 WHERE id = ?3",
                rusqlite::params![private as i32, &now, id],
            )?;
            if rows == 0 {
                return Err(StoreError::NotFound(id.to_string()));
            }
            Ok(())
        })
    }

    /// Lists archived notes, ordered by `last_modified DESC`.
    pub fn list_archived_notes(&self) -> Result<Vec<Note>, StoreError> {
        self.with_conn(|conn| {
            let sql = format!(
                "SELECT {NOTE_COLUMNS} FROM notes WHERE soft_deleted = 0 AND is_archived = 1 ORDER BY last_modified DESC"
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

    /// Retrieves an individual application setting value from `app_state`.
    pub fn get_setting(&self, key: &str) -> Result<Option<serde_json::Value>, StoreError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare("SELECT value FROM app_state WHERE key = ?1")?;
            let mut rows = stmt.query([key])?;
            if let Some(row) = rows.next()? {
                let val_str: String = row.get(0)?;
                let val: serde_json::Value = serde_json::from_str(&val_str)
                    .unwrap_or(serde_json::Value::String(val_str));
                Ok(Some(val))
            } else {
                Ok(None)
            }
        })
    }

    /// Inserts or updates an application setting in `app_state`.
    pub fn set_setting(&self, key: &str, value: serde_json::Value) -> Result<(), StoreError> {
        let val_str = serde_json::to_string(&value).unwrap_or_else(|_| value.to_string());
        self.with_conn(|conn| {
            conn.execute(
                "INSERT INTO app_state (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = ?2",
                rusqlite::params![key, &val_str],
            )?;
            Ok(())
        })
    }

    /// Retrieves all application settings from `app_state` as a JSON map.
    pub fn get_settings(&self) -> Result<serde_json::Map<String, serde_json::Value>, StoreError> {
        self.with_conn(|conn| {
            let mut stmt = conn.prepare("SELECT key, value FROM app_state")?;
            let rows = stmt.query_map([], |row| {
                let k: String = row.get(0)?;
                let v_str: String = row.get(1)?;
                let v: serde_json::Value = serde_json::from_str(&v_str)
                    .unwrap_or(serde_json::Value::String(v_str));
                Ok((k, v))
            })?;
            let mut map = serde_json::Map::new();
            for item in rows {
                let (k, v) = item?;
                map.insert(k, v);
            }
            Ok(map)
        })
    }

    /// Retrieves window geometry `[x, y, width, height]` if configured.
    pub fn get_geometry(&self) -> Option<[i32; 4]> {
        self.get_setting("geometry")
            .ok()
            .flatten()
            .and_then(|val| serde_json::from_value::<[i32; 4]>(val).ok())
    }

    /// Persists window geometry `[x, y, width, height]`.
    pub fn set_geometry(&self, geo: [i32; 4]) -> Result<(), StoreError> {
        self.set_setting("geometry", serde_json::json!(geo))
    }

    /// Stores an API key in the credential store (`app_state`).
    pub fn store_api_key(&self, key_name: &str, key_value: &str) -> Result<(), StoreError> {
        self.with_conn(|conn| crate::extensions::CredentialStore::store_api_key(conn, key_name, key_value))
    }

    /// Retrieves an API key from the credential store (`app_state`).
    pub fn get_api_key(&self, key_name: &str) -> Result<Option<String>, StoreError> {
        self.with_conn(|conn| crate::extensions::CredentialStore::get_api_key(conn, key_name))
    }

    /// Lists configured API keys with masked previews.
    pub fn list_api_keys(&self) -> Result<Vec<crate::extensions::ApiKeyMeta>, StoreError> {
        self.with_conn(|conn| crate::extensions::CredentialStore::list_api_keys(conn))
    }

    /// Deletes an API key from `app_state`.
    pub fn delete_api_key(&self, key_name: &str) -> Result<bool, StoreError> {
        self.with_conn(|conn| crate::extensions::CredentialStore::delete_api_key(conn, key_name))
    }

    /// Executes a native API call for an extension with validation and key substitution.
    pub fn call_api(&self, req: crate::extensions::CallApiRequest) -> Result<crate::extensions::CallApiResponse, StoreError> {
        self.with_conn(|conn| Ok(crate::extensions::call_api(conn, req)))
    }

    /// Retrieves an extension preference from `app_state`.
    pub fn get_extension_preference(
        &self,
        extension_name: &str,
        key: &str,
    ) -> Result<Option<String>, StoreError> {
        self.with_conn(|conn| crate::extensions::get_extension_preference(conn, extension_name, key))
    }

    /// Sets an extension preference in `app_state`.
    pub fn set_extension_preference(
        &self,
        extension_name: &str,
        key: &str,
        value: &str,
    ) -> Result<(), StoreError> {
        self.with_conn(|conn| crate::extensions::set_extension_preference(conn, extension_name, key, value))
    }

    /// Fetches AI models for a specified provider from online APIs or local Ollama.
    pub fn fetch_ai_models(&self, provider: &str) -> Result<Vec<crate::extensions::AiModel>, String> {
        self.with_conn(|conn| Ok(crate::extensions::fetch_ai_models_for_provider(conn, provider, None)))
            .map_err(|e| e.to_string())
            .and_then(|r| r)
    }
}
