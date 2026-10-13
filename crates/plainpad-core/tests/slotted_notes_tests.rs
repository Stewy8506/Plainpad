use plainpad_core::store::{
    deity_for_slot, SqliteStore, StoreError, GREEK_DEITIES, HERMES_DEFAULT_CONTENT,
};

#[test]
fn test_slotted_notes_initialization_count_and_deities() {
    let store = SqliteStore::open_in_memory().expect("Failed to open in-memory store");

    let slotted_notes = store
        .list_slotted_notes()
        .expect("Failed to list slotted notes");

    assert_eq!(
        slotted_notes.len(),
        9,
        "Exactly 9 slotted notes must be initialized"
    );

    for (i, note) in slotted_notes.iter().enumerate() {
        assert!(note.is_slotted, "Note must have is_slotted == true");
        assert_eq!(
            note.slot_index, i as i32,
            "Slot index must match position {} in 0..8",
            i
        );
        assert_eq!(
            note.db_index, i as i32,
            "Slotted notes must have initial db_index matching slot_index"
        );
        assert!(
            !note.soft_deleted,
            "Slotted notes must not be marked soft_deleted"
        );
        assert!(!note.is_archived, "Slotted notes must not be archived");
    }

    // Slot 0 Hermes default content check
    let hermes_note = &slotted_notes[0];
    assert_eq!(hermes_note.content, HERMES_DEFAULT_CONTENT);
    assert_eq!(
        hermes_note.content,
        "This is your first of nine slotted notes. It'll stay with you."
    );

    // Verify deity mapping helper
    assert_eq!(GREEK_DEITIES.len(), 9);
    for (i, deity) in GREEK_DEITIES.iter().enumerate() {
        assert_eq!(deity_for_slot(i as i32), Some(*deity));
    }
    assert_eq!(deity_for_slot(-1), None);
    assert_eq!(deity_for_slot(9), None);
}

#[test]
fn test_slotted_notes_idempotence() {
    let store = SqliteStore::open_in_memory().expect("Failed to open in-memory store");

    let hermes = store
        .get_slotted_note(0)
        .expect("Failed to get slot 0")
        .expect("Slot 0 must exist");

    // Modify Hermes note with custom user text
    let custom_text = "Hermes custom notes: groceries, reading list, ideas.";
    let updated = store
        .save_note(&hermes.id, custom_text)
        .expect("Failed to save custom note text");
    assert_eq!(updated.content, custom_text);

    // Run ensure_slotted_notes again (simulating subsequent app launches)
    let re_ensured = store
        .ensure_slotted_notes()
        .expect("Failed to re-ensure slotted notes");
    assert_eq!(re_ensured.len(), 9);

    let hermes_after = store
        .get_slotted_note(0)
        .expect("Failed to get slot 0 after re-ensure")
        .expect("Slot 0 must exist");

    assert_eq!(
        hermes_after.content, custom_text,
        "Subsequent initialization must preserve existing user content and not reset to default text"
    );
    assert_eq!(hermes_after.id, hermes.id);
}

#[test]
fn test_slotted_notes_bounds_and_query() {
    let store = SqliteStore::open_in_memory().expect("Failed to open in-memory store");

    // All slots 0..=8 must succeed and return Some(note)
    for i in 0..=8 {
        let note = store
            .get_slotted_note(i)
            .expect("Valid slot query should succeed");
        assert!(note.is_some(), "Slot {i} must exist");
        let note = note.unwrap();
        assert_eq!(note.slot_index, i);
        assert!(note.is_slotted);
    }

    // Out of bounds (<0 or >8) must fail with InvalidOperation
    let lower_bound_err = store.get_slotted_note(-1);
    assert!(
        matches!(lower_bound_err, Err(StoreError::InvalidOperation(_))),
        "Slot -1 must return InvalidOperation"
    );

    let upper_bound_err = store.get_slotted_note(9);
    assert!(
        matches!(upper_bound_err, Err(StoreError::InvalidOperation(_))),
        "Slot 9 must return InvalidOperation"
    );

    let far_bound_err = store.get_slotted_note(100);
    assert!(
        matches!(far_bound_err, Err(StoreError::InvalidOperation(_))),
        "Slot 100 must return InvalidOperation"
    );
}

#[test]
fn test_slotted_notes_invariance_rules() {
    let store = SqliteStore::open_in_memory().expect("Failed to open in-memory store");

    // 1. Calling soft_delete_note on any slotted note must return SlottedNoteCannotBeDeleted
    for i in 0..=8 {
        let note = store
            .get_slotted_note(i)
            .expect("query slot")
            .expect("slot exists");

        let del_res = store.soft_delete_note(&note.id);
        assert!(
            matches!(del_res, Err(StoreError::SlottedNoteCannotBeDeleted)),
            "Slot {i} ({}) must be immune to soft deletion into The Void",
            deity_for_slot(i).unwrap_or("Unknown")
        );
    }

    // 2. Saving empty string "" to a slotted note is fully permitted and retains is_slotted
    let ares = store
        .get_slotted_note(2)
        .expect("query Ares")
        .expect("Ares exists");
    let saved_empty = store
        .save_note(&ares.id, "")
        .expect("Saving empty string to slotted note must succeed");
    assert_eq!(saved_empty.content, "");
    assert!(saved_empty.is_slotted);
    assert_eq!(saved_empty.slot_index, 2);

    let ares_reloaded = store
        .get_slotted_note(2)
        .expect("reload Ares")
        .expect("Ares exists");
    assert_eq!(ares_reloaded.content, "");
    assert!(ares_reloaded.is_slotted);

    // 3. Creating unslotted scratch notes via create_note() coexists with slotted notes
    let scratch1 = store
        .create_note("Scratch note 1")
        .expect("create scratch note 1");
    assert!(!scratch1.is_slotted, "Scratch note must not be slotted");
    assert_eq!(scratch1.slot_index, -1, "Scratch note slot_index must be -1");
    assert_eq!(scratch1.db_index, 0, "First scratch note db_index should be 0");

    let scratch2 = store
        .create_note("Scratch note 2")
        .expect("create scratch note 2");
    assert!(!scratch2.is_slotted);
    assert_eq!(scratch2.slot_index, -1);
    assert_eq!(scratch2.db_index, 1, "Second scratch note db_index should be 1");

    let active_scratch = store
        .list_active_notes()
        .expect("list active scratch notes");
    assert_eq!(
        active_scratch.len(),
        2,
        "list_active_notes should contain only unslotted scratch notes"
    );
    assert_eq!(active_scratch[0].id, scratch1.id);
    assert_eq!(active_scratch[1].id, scratch2.id);

    // Slotted notes still count exactly 9
    let slotted_after = store.list_slotted_notes().expect("list slotted notes");
    assert_eq!(slotted_after.len(), 9);

    // 4. Reordering scratch notes must not mutate or displace slotted notes
    store
        .reorder_notes(&[scratch2.id.clone(), scratch1.id.clone()])
        .expect("reorder scratch notes");

    let active_after_reorder = store
        .list_active_notes()
        .expect("list active after reorder");
    assert_eq!(active_after_reorder[0].id, scratch2.id);
    assert_eq!(active_after_reorder[1].id, scratch1.id);

    // Verify all 9 slotted notes still retain unchanged slot_index and is_slotted
    for i in 0..=8 {
        let slotted = store
            .get_slotted_note(i)
            .expect("query slot")
            .expect("slot exists");
        assert!(slotted.is_slotted);
        assert_eq!(slotted.slot_index, i);
    }
}
