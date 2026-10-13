use plainpad_core::store::{SqliteStore, StoreError};

#[test]
fn test_note_crud_and_stack_ordering() {
    let store = SqliteStore::open_in_memory().expect("open store");

    let n1 = store.create_note("First Note").expect("create note 1");
    let n2 = store.create_note("Second Note").expect("create note 2");
    let n3 = store.create_note("Third Note").expect("create note 3");

    assert_eq!(n1.db_index, 0);
    assert_eq!(n2.db_index, 1);
    assert_eq!(n3.db_index, 2);

    let active = store.list_active_notes().expect("list active notes");
    assert_eq!(active.len(), 3);
    assert_eq!(active[0].id, n1.id);
    assert_eq!(active[1].id, n2.id);
    assert_eq!(active[2].id, n3.id);

    // Reorder with reversed order
    store
        .reorder_notes(&[n3.id.clone(), n2.id.clone(), n1.id.clone()])
        .expect("reorder notes");

    let reordered = store.list_active_notes().expect("list active after reorder");
    assert_eq!(reordered.len(), 3);
    assert_eq!(reordered[0].id, n3.id);
    assert_eq!(reordered[1].id, n2.id);
    assert_eq!(reordered[2].id, n1.id);
    assert_eq!(reordered[0].db_index, 0);
    assert_eq!(reordered[1].db_index, 1);
    assert_eq!(reordered[2].db_index, 2);
}

#[test]
fn test_locked_note_protection() {
    let store = SqliteStore::open_in_memory().expect("open store");
    let note = store.create_note("Original text").expect("create note");

    store.set_locked(&note.id, true).expect("lock note");

    // Attempting edit must fail
    let edit_res = store.save_note(&note.id, "New text");
    assert!(matches!(edit_res, Err(StoreError::NoteLocked)));

    // Attempting soft delete must fail
    let delete_res = store.soft_delete_note(&note.id);
    assert!(matches!(delete_res, Err(StoreError::NoteLocked)));

    // Verify content remained unmodified
    let loaded = store.load_note(&note.id).expect("load note").unwrap();
    assert_eq!(loaded.content, "Original text");
    assert!(loaded.is_locked);

    // Unlock and retry
    store.set_locked(&note.id, false).expect("unlock note");

    let saved = store.save_note(&note.id, "Updated text").expect("save note");
    assert_eq!(saved.content, "Updated text");

    store.soft_delete_note(&note.id).expect("soft delete succeeds when unlocked");
    assert!(store.list_active_notes().expect("list active").is_empty());
}

#[test]
fn test_archive_filtering() {
    let store = SqliteStore::open_in_memory().expect("open store");
    let note = store.create_note("Archive Me").expect("create note");

    // Mark as archived
    store.set_archived(&note.id, true).expect("archive note");

    let active = store.list_active_notes().expect("list active");
    assert!(active.is_empty(), "Archived notes must not appear in active notes");

    let archived = store.list_archived_notes().expect("list archived");
    assert_eq!(archived.len(), 1);
    assert_eq!(archived[0].id, note.id);
    assert!(archived[0].is_archived);

    // Unarchive
    store.set_archived(&note.id, false).expect("unarchive note");

    let active_restored = store.list_active_notes().expect("list active after unarchive");
    assert_eq!(active_restored.len(), 1);
    assert_eq!(active_restored[0].id, note.id);

    let archived_now = store.list_archived_notes().expect("list archived after unarchive");
    assert!(archived_now.is_empty());
}

#[test]
fn test_the_void_soft_delete_and_restore() {
    let store = SqliteStore::open_in_memory().expect("open store");
    let note1 = store.create_note("Void Note 1").expect("create note 1");
    let _note2 = store.create_note("Void Note 2").expect("create note 2");

    // Soft delete note 1
    store.soft_delete_note(&note1.id).expect("soft delete note 1");

    let active = store.list_active_notes().expect("list active");
    assert_eq!(active.len(), 1);
    assert_ne!(active[0].id, note1.id);

    let void_list = store.list_void().expect("list void");
    assert_eq!(void_list.len(), 1);
    assert_eq!(void_list[0].id, note1.id);
    assert!(void_list[0].soft_deleted);
    assert!(void_list[0].deleted_at.is_some());

    // Edits to note in Void must fail
    let edit_res = store.save_note(&note1.id, "Try editing in Void");
    assert!(matches!(edit_res, Err(StoreError::NoteInVoid)));

    // Restore note 1 from Void
    let restored = store.restore_from_void(&note1.id).expect("restore from void");
    assert!(!restored.soft_deleted);
    assert!(restored.deleted_at.is_none());

    let void_now = store.list_void().expect("list void after restore");
    assert!(void_now.is_empty());

    let active_now = store.list_active_notes().expect("list active after restore");
    assert_eq!(active_now.len(), 2);
    // Restored note receives highest db_index (top of stack)
    assert_eq!(active_now.last().unwrap().id, note1.id);
}

#[test]
fn test_slotted_note_void_immunity() {
    let store = SqliteStore::open_in_memory().expect("open store");
    let note = store.create_note("Apollo Deity Slot Note").expect("create note");

    // Manually mark as slotted
    store
        .with_conn(|conn| {
            conn.execute(
                "UPDATE notes SET is_slotted = 1, slot_index = 1 WHERE id = ?1",
                [&note.id],
            )?;
            Ok(())
        })
        .expect("mark note slotted");

    let loaded = store.load_note(&note.id).expect("load note").unwrap();
    assert!(loaded.is_slotted);
    assert_eq!(loaded.slot_index, 1);

    // Soft delete must be rejected
    let res = store.soft_delete_note(&note.id);
    assert!(matches!(res, Err(StoreError::SlottedNoteCannotBeDeleted)));

    // Note remains active
    let active = store.list_active_notes().expect("list active");
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].id, note.id);
}

#[test]
fn test_purge_void_permanent() {
    let store = SqliteStore::open_in_memory().expect("open store");
    let n1 = store.create_note("Note 1").expect("create n1");
    let n2 = store.create_note("Note 2").expect("create n2");
    let n3 = store.create_note("Note 3").expect("create n3");

    store.soft_delete_note(&n1.id).expect("delete n1");
    store.soft_delete_note(&n2.id).expect("delete n2");

    let in_void = store.list_void().expect("list void");
    assert_eq!(in_void.len(), 2);

    let count = store.purge_void().expect("purge void");
    assert_eq!(count, 2);

    assert!(store.list_void().expect("list void empty").is_empty());

    let active = store.list_active_notes().expect("active notes remaining");
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].id, n3.id);
}

#[test]
fn test_settings_and_geometry() {
    let store = SqliteStore::open_in_memory().expect("open store");

    // Initially None
    assert_eq!(store.get_geometry(), None);

    store.set_geometry([100, 200, 800, 600]).expect("set geometry");
    assert_eq!(store.get_geometry(), Some([100, 200, 800, 600]));

    // Setting key-value store
    store
        .set_setting("theme", serde_json::json!("tokyo-night"))
        .expect("set theme");
    let theme = store.get_setting("theme").expect("get theme");
    assert_eq!(theme, Some(serde_json::json!("tokyo-night")));

    let all = store.get_settings().expect("get all settings");
    assert_eq!(all.get("theme"), Some(&serde_json::json!("tokyo-night")));
}

#[test]
fn test_purge_expired_void() {
    let store = SqliteStore::open_in_memory().expect("open store");
    let note = store.create_note("Old Note").expect("create note");
    store.soft_delete_note(&note.id).expect("delete note");

    // Manually backdate deleted_at to 40 days ago
    let old_date = (chrono::Utc::now() - chrono::Duration::days(40))
        .to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
    store
        .with_conn(|conn| {
            conn.execute(
                "UPDATE notes SET deleted_at = ?1 WHERE id = ?2",
                rusqlite::params![&old_date, &note.id],
            )?;
            Ok(())
        })
        .expect("backdate deleted_at");

    // 30 days retention policy: should delete 40-day old note
    let purged = store.purge_expired_void(30).expect("purge expired");
    assert_eq!(purged, 1);
    assert!(store.list_void().expect("void is empty").is_empty());
}
