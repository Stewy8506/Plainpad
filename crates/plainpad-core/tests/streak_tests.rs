use plainpad_core::get_break_facts;
use plainpad_core::store::SqliteStore;

#[test]
fn test_break_facts_catalog() {
    let facts = get_break_facts();
    assert!(!facts.is_empty(), "Break facts catalog should not be empty");
    
    // Check categories exist
    let categories: Vec<&str> = facts.iter().map(|f| f.category.as_str()).collect();
    assert!(categories.contains(&"WILDLIFE FACT-FILE"));
    assert!(categories.contains(&"SPACE TRIVIA"));
    assert!(categories.contains(&"MINDFUL MOMENT"));

    // Check every fact has non-empty text
    for fact in &facts {
        assert!(!fact.text.trim().is_empty());
    }
}

#[test]
fn test_no_peek_streak_lifecycle() {
    let store = SqliteStore::open_in_memory().expect("Failed to open in-memory store");

    // 1. Initial streak is (0, 0)
    let (current, best) = store.get_no_peek_streak().expect("Failed to get initial streak");
    assert_eq!(current, 0);
    assert_eq!(best, 0);

    // 2. Increment streak once
    let (current, best) = store.increment_no_peek_streak().expect("Failed to increment streak");
    assert_eq!(current, 1);
    assert_eq!(best, 1);

    // 3. Increment streak two more times (streak = 3, best = 3)
    let (current, best) = store.increment_no_peek_streak().unwrap();
    assert_eq!(current, 2);
    assert_eq!(best, 2);

    let (current, best) = store.increment_no_peek_streak().unwrap();
    assert_eq!(current, 3);
    assert_eq!(best, 3);

    // Verify get returns (3, 3)
    let (current, best) = store.get_no_peek_streak().unwrap();
    assert_eq!(current, 3);
    assert_eq!(best, 3);

    // 4. Peeking resets current streak to 0, but preserves best streak of 3
    let (current, best) = store.reset_no_peek_streak().expect("Failed to reset streak");
    assert_eq!(current, 0);
    assert_eq!(best, 3);

    let (current, best) = store.get_no_peek_streak().unwrap();
    assert_eq!(current, 0);
    assert_eq!(best, 3);

    // 5. Incrementing again increments current to 1, best remains 3
    let (current, best) = store.increment_no_peek_streak().unwrap();
    assert_eq!(current, 1);
    assert_eq!(best, 3);

    let (current, best) = store.increment_no_peek_streak().unwrap();
    assert_eq!(current, 2);
    assert_eq!(best, 3);

    let (current, best) = store.increment_no_peek_streak().unwrap();
    assert_eq!(current, 3);
    assert_eq!(best, 3);

    // 6. Breaking record (4 > 3) updates both current and best to 4
    let (current, best) = store.increment_no_peek_streak().unwrap();
    assert_eq!(current, 4);
    assert_eq!(best, 4);

    let (current, best) = store.get_no_peek_streak().unwrap();
    assert_eq!(current, 4);
    assert_eq!(best, 4);
}
