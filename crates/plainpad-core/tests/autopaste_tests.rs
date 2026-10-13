//! Integration tests for AutoPaste continuous clipboard ingestion engine (REQ-14).

use plainpad_core::clipboard::{AutoPasteEngine, ClipboardReader, MockClipboardReader};
use std::sync::Arc;
use std::thread;

#[test]
fn test_engine_initial_state_and_lifecycle() {
    let engine = AutoPasteEngine::new();
    assert!(!engine.is_active(), "Engine should start inactive");

    // Start engine
    let was_active = engine.start();
    assert!(!was_active, "Previous active state should be false");
    assert!(engine.is_active(), "Engine should be active after start()");

    // Repeated start returns true
    let was_active_2 = engine.start();
    assert!(was_active_2, "Previous active state should be true");
    assert!(engine.is_active());

    // Stop engine
    let was_active_stop = engine.stop();
    assert!(was_active_stop, "Previous active state should be true");
    assert!(!engine.is_active(), "Engine should be inactive after stop()");

    // Repeated stop returns false
    let was_active_stop_2 = engine.stop();
    assert!(!was_active_stop_2, "Previous active state should be false");
    assert!(!engine.is_active());
}

#[test]
fn test_clip_ingestion_and_whitespace_trimming() {
    let engine = AutoPasteEngine::new();

    // Standard clip
    assert_eq!(
        engine.process_clip("First clip"),
        Some("First clip".to_string())
    );

    // Whitespace trimming
    assert_eq!(
        engine.process_clip("  Second clip with padding   \n\t"),
        Some("Second clip with padding".to_string())
    );

    // Empty and whitespace-only clips are dropped
    assert_eq!(engine.process_clip(""), None);
    assert_eq!(engine.process_clip("   "), None);
    assert_eq!(engine.process_clip("\n\t\r"), None);
    assert_eq!(engine.process_clip("   \n   \t  "), None);
}

#[test]
fn test_consecutive_duplicate_suppression() {
    let engine = AutoPasteEngine::new();

    // First ingestion succeeds
    assert_eq!(
        engine.process_clip("Unique content"),
        Some("Unique content".to_string())
    );

    // Immediate duplicate suppressed
    assert_eq!(
        engine.process_clip("Unique content"),
        None,
        "Consecutive duplicate copy must be suppressed"
    );

    // Duplicate with varying surrounding whitespace is also suppressed
    assert_eq!(
        engine.process_clip("  Unique content \n"),
        None,
        "Whitespace-trimmed duplicate must be suppressed"
    );

    // Interleaved distinct clip succeeds
    assert_eq!(
        engine.process_clip("Different content"),
        Some("Different content".to_string())
    );

    // Now previous content is accepted again because it is not consecutively duplicate
    assert_eq!(
        engine.process_clip("Unique content"),
        Some("Unique content".to_string())
    );
}

#[test]
fn test_restart_lifecycle_resets_or_preserves_state() {
    let engine = AutoPasteEngine::new();

    assert!(engine.start() == false);
    assert_eq!(
        engine.process_clip("Session 1 snippet"),
        Some("Session 1 snippet".to_string())
    );

    assert!(engine.stop() == true);
    assert!(!engine.is_active());

    // Restart engine
    assert!(engine.start() == false);
    assert!(engine.is_active());

    // Process new snippet in session 2
    assert_eq!(
        engine.process_clip("Session 2 snippet"),
        Some("Session 2 snippet".to_string())
    );

    // Clear last content manually if desired
    engine.clear_last_content();
    assert_eq!(
        engine.process_clip("Session 2 snippet"),
        Some("Session 2 snippet".to_string())
    );
}

#[test]
fn test_mock_clipboard_reader_queue_consumption() {
    let mut reader = MockClipboardReader::from_items(vec![
        "Clip 1".to_string(),
        "Clip 2".to_string(),
        "Clip 3".to_string(),
    ]);

    assert_eq!(reader.read_text().unwrap(), Some("Clip 1".to_string()));
    assert_eq!(reader.read_text().unwrap(), Some("Clip 2".to_string()));
    assert_eq!(reader.read_text().unwrap(), Some("Clip 3".to_string()));
    assert_eq!(reader.read_text().unwrap(), None);

    // Push new item while running
    reader.push("Late arrival");
    assert_eq!(reader.read_text().unwrap(), Some("Late arrival".to_string()));
    assert_eq!(reader.read_text().unwrap(), None);
}

#[test]
fn test_concurrent_multithreaded_ingestion_safety() {
    let engine = Arc::new(AutoPasteEngine::new());
    engine.start();

    let mut handles = Vec::new();
    for thread_idx in 0..8 {
        let engine_clone = Arc::clone(&engine);
        let handle = thread::spawn(move || {
            for i in 0..100 {
                let clip_text = format!("Thread {} clip {}", thread_idx, i);
                let _ = engine_clone.process_clip(&clip_text);
            }
        });
        handles.push(handle);
    }

    for h in handles {
        h.join().expect("Worker thread panicked");
    }

    assert!(engine.is_active());
    engine.stop();
    assert!(!engine.is_active());
}
