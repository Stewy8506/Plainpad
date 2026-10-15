use plainpad_core::model::{Antilink, ChecklistItem};
use plainpad_core::store::{extract_antilinks, extract_checklist_items, shorten_url, SqliteStore};

#[test]
fn test_url_shortening() {
    assert_eq!(
        shorten_url("https://github.com/rust-lang/rust"),
        "github.com/..."
    );
    assert_eq!(
        shorten_url("http://news.ycombinator.com/item?id=12345"),
        "news.ycombinator.com/..."
    );
    assert_eq!(
        shorten_url("https://www.example.com/blog/article"),
        "example.com/..."
    );
    assert_eq!(shorten_url("https://plainpad.app/"), "plainpad.app");
    assert_eq!(shorten_url("https://plainpad.app"), "plainpad.app");
}

#[test]
fn test_checklist_and_antilink_extraction() {
    let note_id = "test-note-1";
    let content = "Project roadmap:
- [ ] Implement parser
- [x] Write initial spec
* [ ] Review documentation
+ [x] Setup CI pipeline
1. Unrelated ordered item
Check repo at https://github.com/rust-lang/rust and website https://rust-lang.org.
Done!";

    let items = extract_checklist_items(note_id, content);
    assert_eq!(items.len(), 4);

    assert_eq!(items[0].content, "Implement parser");
    assert!(!items[0].checked);
    assert_eq!(items[0].index, 0);

    assert_eq!(items[1].content, "Write initial spec");
    assert!(items[1].checked);
    assert_eq!(items[1].index, 1);

    assert_eq!(items[2].content, "Review documentation");
    assert!(!items[2].checked);
    assert_eq!(items[2].index, 2);

    assert_eq!(items[3].content, "Setup CI pipeline");
    assert!(items[3].checked);
    assert_eq!(items[3].index, 3);

    let links = extract_antilinks(note_id, content);
    assert_eq!(links.len(), 2);
    assert_eq!(links[0].full_url, "https://github.com/rust-lang/rust");
    assert_eq!(links[0].shortened_url, "github.com/...");
    assert_eq!(links[1].full_url, "https://rust-lang.org");
    assert_eq!(links[1].shortened_url, "rust-lang.org");
}

#[test]
fn test_save_note_synchronizes_relational_entities() {
    let store = SqliteStore::open_in_memory().expect("open in-memory sqlite store");

    // 1. Create a note
    let note = store.create_note("Initial note").expect("create note");
    let note_id = &note.id;

    // Initially 0 child entities
    let items = store.get_checklist_items(note_id).expect("get checklist items");
    assert!(items.is_empty());
    let links = store.get_antilinks(note_id).expect("get antilinks");
    assert!(links.is_empty());

    // 2. Save note with rich content
    let content_v1 = "# Sprint Goals
- [ ] Build parser
- [x] Define SQLite migrations
Reference: https://sqlite.org/wal.html for details.";

    store.save_note(note_id, content_v1).expect("save note v1");

    let items_v1 = store.get_checklist_items(note_id).expect("get items v1");
    assert_eq!(items_v1.len(), 2);
    assert_eq!(items_v1[0].content, "Build parser");
    assert!(!items_v1[0].checked);
    assert_eq!(items_v1[1].content, "Define SQLite migrations");
    assert!(items_v1[1].checked);

    let links_v1 = store.get_antilinks(note_id).expect("get links v1");
    assert_eq!(links_v1.len(), 1);
    assert_eq!(links_v1[0].full_url, "https://sqlite.org/wal.html");
    assert_eq!(links_v1[0].shortened_url, "sqlite.org/...");

    // 3. Update note content: toggle first item, remove second item, add another URL
    let content_v2 = "# Sprint Goals
- [x] Build parser
Reference: https://sqlite.org/wal.html and https://crates.io/crates/rusqlite";

    store.save_note(note_id, content_v2).expect("save note v2");

    let items_v2 = store.get_checklist_items(note_id).expect("get items v2");
    assert_eq!(items_v2.len(), 1);
    assert_eq!(items_v2[0].content, "Build parser");
    assert!(items_v2[0].checked);

    let links_v2 = store.get_antilinks(note_id).expect("get links v2");
    assert_eq!(links_v2.len(), 2);
    assert_eq!(links_v2[0].full_url, "https://sqlite.org/wal.html");
    assert_eq!(links_v2[1].full_url, "https://crates.io/crates/rusqlite");
    assert_eq!(links_v2[1].shortened_url, "crates.io/...");

    // 4. Test explicit replace_checklist_items and replace_antilinks
    let custom_items = vec![ChecklistItem {
        id: "custom-chk-1".to_string(),
        note_id: note_id.to_string(),
        content: "Custom task".to_string(),
        checked: false,
        index: 0,
        vertical_position: 1.0,
        responsibility_range_location: 0,
        responsibility_range_length: 11,
        is_hidden: false,
    }];
    store
        .replace_checklist_items(note_id, &custom_items)
        .expect("replace checklist items");
    let fetched_items = store.get_checklist_items(note_id).expect("get items");
    assert_eq!(fetched_items.len(), 1);
    assert_eq!(fetched_items[0].id, "custom-chk-1");

    let custom_links = vec![Antilink {
        id: "custom-link-1".to_string(),
        note_id: note_id.to_string(),
        full_url: "https://plainpad.io".to_string(),
        shortened_url: "plainpad.io".to_string(),
        current_range_location: 0,
        current_range_length: 18,
    }];
    store
        .replace_antilinks(note_id, &custom_links)
        .expect("replace antilinks");
    let fetched_links = store.get_antilinks(note_id).expect("get links");
    assert_eq!(fetched_links.len(), 1);
    assert_eq!(fetched_links[0].id, "custom-link-1");

    // 5. Foreign key cascade test: deleting note cascades to child tables
    store.soft_delete_note(note_id).expect("soft delete note");
    store.permanently_delete_note(note_id).expect("delete note");
    let items_after_del = store.get_checklist_items(note_id).expect("get items");
    assert!(items_after_del.is_empty());
    let links_after_del = store.get_antilinks(note_id).expect("get links");
    assert!(links_after_del.is_empty());
}
