use plainpad_core::model::{now_iso, Antilink, Attachment, ChecklistItem, SyncState};
use plainpad_core::store::SqliteStore;
use std::path::PathBuf;

fn temp_db_path(name: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(format!("plainpad_test_{}_{}", name, uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    dir.join("test.db")
}

#[test]
fn test_relational_crud_and_cascade() {
    let store = SqliteStore::open_in_memory().expect("open store");

    // 1. Insert a note
    let note = store.create_note("Meeting notes with tasks").expect("create note");
    let note_id = &note.id;

    // 2. Add 2 checklist items
    let item1 = ChecklistItem {
        id: "chk-1".to_string(),
        note_id: note_id.clone(),
        content: "Buy milk".to_string(),
        checked: false,
        index: 0,
        vertical_position: 10.0,
        responsibility_range_location: 0,
        responsibility_range_length: 8,
        is_hidden: false,
    };
    let item2 = ChecklistItem {
        id: "chk-2".to_string(),
        note_id: note_id.clone(),
        content: "Call plumber".to_string(),
        checked: true,
        index: 1,
        vertical_position: 30.0,
        responsibility_range_location: 9,
        responsibility_range_length: 12,
        is_hidden: false,
    };
    store
        .replace_checklist_items(note_id, &[item1, item2])
        .expect("replace checklist items");

    let chk_items = store.get_checklist_items(note_id).expect("get checklist items");
    assert_eq!(chk_items.len(), 2);
    assert_eq!(chk_items[0].id, "chk-1");
    assert!(!chk_items[0].checked);
    assert_eq!(chk_items[1].id, "chk-2");
    assert!(chk_items[1].checked);

    // Toggle checklist item
    store.toggle_checklist_item("chk-1", true).expect("toggle checklist item");
    let chk_items_updated = store.get_checklist_items(note_id).expect("get checklist items");
    assert!(chk_items_updated[0].checked);

    // 3. Add 1 antilink
    let link = Antilink {
        id: "link-1".to_string(),
        note_id: note_id.clone(),
        full_url: "https://example.com/very/long/url".to_string(),
        shortened_url: "https://ex.mp/1".to_string(),
        current_range_location: 15,
        current_range_length: 14,
    };
    store.save_antilinks(note_id, &[link]).expect("save antilinks");
    let links = store.get_antilinks(note_id).expect("get antilinks");
    assert_eq!(links.len(), 1);
    assert_eq!(links[0].full_url, "https://example.com/very/long/url");

    // 4. Add 1 attachment
    let attachment = Attachment {
        id: "att-1".to_string(),
        note_id: note_id.clone(),
        file_name: "diagram.png".to_string(),
        file_url: "local://diagram.png".to_string(),
        file_size: 2048,
        file_type: "png".to_string(),
        mime_type: "image/png".to_string(),
        location_type: "local".to_string(),
        sort_order: 0,
        created: now_iso(),
    };
    store.add_attachment(&attachment).expect("add attachment");
    let attachments = store.get_attachments(note_id).expect("get attachments");
    assert_eq!(attachments.len(), 1);
    assert_eq!(attachments[0].file_name, "diagram.png");

    // 5. Add 1 sync state
    let sync_state = SyncState {
        note_id: note_id.clone(),
        base_content: "Meeting notes with tasks".to_string(),
        is_positioned: true,
        pending_edit_snapshot: Some(vec![1, 2, 3]),
        system_fields: Some(vec![4, 5, 6]),
        last_synced_at: Some(now_iso()),
        needs_upload: false,
        change_tag: Some("tag-123".to_string()),
    };
    store.set_sync_state(&sync_state).expect("set sync state");
    let retrieved_sync = store.get_sync_state(note_id).expect("get sync state");
    assert!(retrieved_sync.is_some());
    let retrieved_sync = retrieved_sync.unwrap();
    assert_eq!(retrieved_sync.base_content, "Meeting notes with tasks");
    assert!(retrieved_sync.is_positioned);
    assert_eq!(retrieved_sync.change_tag.as_deref(), Some("tag-123"));

    // 6. Delete note permanently (soft delete then purge void, or direct delete)
    store.soft_delete_note(note_id).expect("soft delete note");
    store.permanently_delete_note(note_id).expect("permanently delete note");

    // Verify all child tables cascaded to empty
    let chk_items_after = store.get_checklist_items(note_id).expect("get checklist items");
    assert_eq!(chk_items_after.len(), 0);

    let links_after = store.get_antilinks(note_id).expect("get antilinks");
    assert_eq!(links_after.len(), 0);

    let attachments_after = store.get_attachments(note_id).expect("get attachments");
    assert_eq!(attachments_after.len(), 0);

    let sync_after = store.get_sync_state(note_id).expect("get sync state");
    assert!(sync_after.is_none());
}

#[test]
fn test_snapshot_and_safe_restore() {
    let db_path = temp_db_path("snapshot_restore");
    let store = SqliteStore::open(&db_path).expect("open on-disk store");

    // 1. Insert note with child entities
    let note = store.create_note("Initial note content").expect("create note");
    let note_id = &note.id;

    let attachment = Attachment {
        id: "att-orig".to_string(),
        note_id: note_id.clone(),
        file_name: "report.pdf".to_string(),
        file_url: "local://report.pdf".to_string(),
        file_size: 4096,
        file_type: "pdf".to_string(),
        mime_type: "application/pdf".to_string(),
        location_type: "local".to_string(),
        sort_order: 0,
        created: now_iso(),
    };
    store.add_attachment(&attachment).expect("add attachment");

    // 2. Create pre-restore snapshot
    let snapshot_file = store
        .create_pre_restore_snapshot("test_backup")
        .expect("create snapshot");
    assert!(snapshot_file.exists());

    // Verify list_snapshots sees it
    let snapshots = store.list_snapshots().expect("list snapshots");
    assert!(!snapshots.is_empty());
    assert_eq!(snapshots[0].reason, "test_backup");

    // 3. Modify note content and delete attachment
    store
        .save_note(note_id, "Heavily modified content that should be reverted")
        .expect("save modified note");
    store.delete_attachment("att-orig").expect("delete attachment");

    let current_note = store.load_note(note_id).expect("load").unwrap();
    assert_eq!(current_note.content, "Heavily modified content that should be reverted");
    let current_atts = store.get_attachments(note_id).expect("get attachments");
    assert_eq!(current_atts.len(), 0);

    // Initial sync_isolated state should be false
    assert!(!store.is_sync_isolated());

    // 4. Restore from snapshot
    store.restore_from_snapshot(&snapshot_file).expect("restore from snapshot");

    // 5. Verify pre-restore safety backup was automatically created in snapshots/
    let snapshots_after = store.list_snapshots().expect("list snapshots after restore");
    assert!(
        snapshots_after.iter().any(|s| s.reason == "pre_restore_safety"),
        "pre_restore_safety snapshot must be automatically generated"
    );

    // 6. Verify database restored to original state
    let restored_note = store.load_note(note_id).expect("load restored note").unwrap();
    assert_eq!(restored_note.content, "Initial note content");

    let restored_atts = store.get_attachments(note_id).expect("get restored attachments");
    assert_eq!(restored_atts.len(), 1);
    assert_eq!(restored_atts[0].id, "att-orig");
    assert_eq!(restored_atts[0].file_name, "report.pdf");

    // 7. Verify sync_isolated is set to true
    assert!(store.is_sync_isolated());
}
