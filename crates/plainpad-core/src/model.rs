use serde::{Deserialize, Serialize};

/// Returns the current UTC time in ISO 8601 format with millisecond precision (e.g. `2026-10-13T14:02:10.123Z`).
pub fn now_iso() -> String {
    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}

/// Derive note title from text: first non-empty line, trimmed, max 60 chars.
pub fn title_from_text(text: &str) -> String {
    let first = text.lines().find(|l| !l.trim().is_empty()).unwrap_or("");
    let t = first.trim();
    if t.chars().count() <= 60 {
        t.to_string()
    } else {
        let cut: String = t.chars().take(57).collect();
        format!("{}…", cut.trim_end())
    }
}

/// Core Data Model 9 Note entity (CDNote) with 17 attributes.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Note {
    pub id: String,
    pub content: String,
    pub created: String,
    pub last_modified: String,
    pub db_index: i32,
    pub is_slotted: bool,
    pub slot_index: i32,
    pub is_locked: bool,
    pub is_archived: bool,
    pub is_private: bool,
    pub is_tutorial: bool,
    pub soft_deleted: bool,
    pub deleted_at: Option<String>,
    pub skipped_urls: Vec<String>,
    pub ck_change_tag: Option<String>,
    pub ck_system_fields: Option<Vec<u8>>,
}

impl Default for Note {
    fn default() -> Self {
        let now = now_iso();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            content: String::new(),
            created: now.clone(),
            last_modified: now,
            db_index: 0,
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
        }
    }
}

impl Note {
    /// Creates a new note with default attributes and given content.
    pub fn new(content: impl Into<String>) -> Self {
        let mut note = Self::default();
        note.content = content.into();
        note
    }

    /// Derives title from first non-empty line (capped at 60 characters).
    pub fn title(&self) -> String {
        title_from_text(&self.content)
    }
}

/// Compact note metadata representation for listing and stack ordering.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct NoteMeta {
    pub id: String,
    pub title: String,
    pub last_modified: String,
    pub db_index: i32,
    pub is_slotted: bool,
    pub slot_index: i32,
    pub is_locked: bool,
    pub is_archived: bool,
    pub is_private: bool,
    pub is_tutorial: bool,
    pub soft_deleted: bool,
    pub deleted_at: Option<String>,
}

impl From<&Note> for NoteMeta {
    fn from(note: &Note) -> Self {
        Self {
            id: note.id.clone(),
            title: note.title(),
            last_modified: note.last_modified.clone(),
            db_index: note.db_index,
            is_slotted: note.is_slotted,
            slot_index: note.slot_index,
            is_locked: note.is_locked,
            is_archived: note.is_archived,
            is_private: note.is_private,
            is_tutorial: note.is_tutorial,
            soft_deleted: note.soft_deleted,
            deleted_at: note.deleted_at.clone(),
        }
    }
}

/// Core Data Model 9 ChecklistItem entity (CDChecklistItem).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct ChecklistItem {
    pub id: String,
    pub note_id: String,
    pub content: String,
    pub checked: bool,
    pub index: i32,
    pub vertical_position: f64,
    pub responsibility_range_location: i32,
    pub responsibility_range_length: i32,
    pub is_hidden: bool,
}

impl Default for ChecklistItem {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            note_id: String::new(),
            content: String::new(),
            checked: false,
            index: 0,
            vertical_position: 0.0,
            responsibility_range_location: 0,
            responsibility_range_length: 0,
            is_hidden: false,
        }
    }
}

/// Core Data Model 9 Antilink entity (CDAntilink).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Antilink {
    pub id: String,
    pub note_id: String,
    pub full_url: String,
    pub shortened_url: String,
    pub current_range_location: i32,
    pub current_range_length: i32,
}

impl Default for Antilink {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            note_id: String::new(),
            full_url: String::new(),
            shortened_url: String::new(),
            current_range_location: 0,
            current_range_length: 0,
        }
    }
}

/// Core Data Model 9 Attachment entity (CDAttachment).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct Attachment {
    pub id: String,
    pub note_id: String,
    pub file_name: String,
    pub file_url: String,
    pub file_size: i64,
    pub file_type: String,
    pub mime_type: String,
    pub location_type: String,
    pub sort_order: i32,
    pub created: String,
}

impl Default for Attachment {
    fn default() -> Self {
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            note_id: String::new(),
            file_name: String::new(),
            file_url: String::new(),
            file_size: 0,
            file_type: String::new(),
            mime_type: String::new(),
            location_type: "local".to_string(),
            sort_order: 0,
            created: now_iso(),
        }
    }
}

/// Core Data Model 9 SyncState entity (CDSyncState).
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct SyncState {
    pub note_id: String,
    pub base_content: String,
    pub is_positioned: bool,
    pub pending_edit_snapshot: Option<Vec<u8>>,
    pub system_fields: Option<Vec<u8>>,
    pub last_synced_at: Option<String>,
    pub needs_upload: bool,
    pub change_tag: Option<String>,
}

impl Default for SyncState {
    fn default() -> Self {
        Self {
            note_id: String::new(),
            base_content: String::new(),
            is_positioned: false,
            pending_edit_snapshot: None,
            system_fields: None,
            last_synced_at: None,
            needs_upload: false,
            change_tag: None,
        }
    }
}
