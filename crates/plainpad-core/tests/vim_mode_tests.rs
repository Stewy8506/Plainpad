use plainpad_core::store::SqliteStore;
use plainpad_core::vim::{
    get_first_non_whitespace, get_line_and_col, get_line_end, get_line_start,
    get_next_word_start, get_prev_word_start, get_word_end, VimMode, VimState,
};

#[test]
fn test_vim_mode_transitions_and_normal_clamping() {
    let text = "Hello world\nSecond line";
    let mut state = VimState::new();
    assert_eq!(state.mode, VimMode::Normal);

    // Enter INSERT mode at index 5
    state.cursor_pos = 5;
    state.set_mode(VimMode::Insert, text);
    assert_eq!(state.mode, VimMode::Insert);

    // Return to NORMAL mode: cursor moves 1 left (standard Vim behavior)
    state.set_mode(VimMode::Normal, text);
    assert_eq!(state.mode, VimMode::Normal);
    assert_eq!(state.cursor_pos, 4);

    // In NORMAL mode, cursor cannot be positioned on the newline char (must clamp to last char of line)
    state.cursor_pos = 11; // index of '\n'
    state.set_mode(VimMode::Normal, text);
    assert_eq!(state.cursor_pos, 10); // 'd' in world

    // Switch to VISUAL and VISUAL_LINE
    state.set_mode(VimMode::Visual, text);
    assert_eq!(state.mode, VimMode::Visual);
    assert_eq!(state.anchor_pos, 10);

    state.set_mode(VimMode::VisualLine, text);
    assert_eq!(state.mode, VimMode::VisualLine);
    assert_eq!(state.anchor_pos, 10);
}

#[test]
fn test_vim_motion_navigation_and_word_boundaries() {
    let text = "alpha beta.gamma\n   delta epsilon\nzeta";
    let mut state = VimState::new();

    // Line start / end
    assert_eq!(get_line_start(text, 0), 0);
    assert_eq!(get_line_end(text, 0), 16);
    assert_eq!(get_first_non_whitespace(text, 17), 20); // after \n and 3 spaces

    // Word motions: w, e, b
    // "alpha beta.gamma"
    // pos 0 ('a') -> 'w' jumps to pos 6 ('b')
    let w1 = get_next_word_start(text, 0);
    assert_eq!(w1, 6);
    // pos 6 ('b') -> 'w' jumps to pos 10 ('.')
    let w2 = get_next_word_start(text, w1);
    assert_eq!(w2, 10);
    // pos 10 ('.') -> 'w' jumps to pos 11 ('g')
    let w3 = get_next_word_start(text, w2);
    assert_eq!(w3, 11);

    // Word end: 'e'
    let e1 = get_word_end(text, 0);
    assert_eq!(e1, 4); // 'a' at end of alpha

    // Prev word: 'b'
    let b1 = get_prev_word_start(text, w2);
    assert_eq!(b1, 6); // 'b' of beta

    // Line and column coordinates
    let (line, col) = get_line_and_col(text, 0);
    assert_eq!(line, 1);
    assert_eq!(col, 1);

    let (line2, col2) = get_line_and_col(text, 20);
    assert_eq!(line2, 2);
    assert_eq!(col2, 4); // 3 spaces + 1

    // Vertical motions: j and k
    state.cursor_pos = 2; // 'p' in alpha
    state.execute_motion(text, "j", 1);
    assert_eq!(state.cursor_pos, 19); // line 2, clamped col

    state.execute_motion(text, "k", 1);
    assert_eq!(state.cursor_pos, 2); // returns to line 1 col 3

    // Jump to bottom G and top gg
    state.execute_motion(text, "G", 1);
    assert_eq!(state.cursor_pos, 34); // start of "zeta"
    state.execute_motion(text, "gg", 1);
    assert_eq!(state.cursor_pos, 0); // start of "alpha"
}

#[test]
fn test_vim_action_operators_and_registers() {
    let mut doc = String::from("First line\nSecond line\nThird line");
    let mut state = VimState::new();

    // 1. Delete char 'x'
    state.cursor_pos = 0;
    state.execute_delete_char(&mut doc, 1);
    assert_eq!(state.yank_register, "F");
    assert!(!state.is_line_yank);
    assert!(doc.starts_with("irst line"));

    // 2. Replace char 'r'
    state.cursor_pos = 0;
    state.execute_replace_char(&mut doc, 'B');
    assert!(doc.starts_with("Brst line"));

    // 3. Yank current line 'yy'
    state.execute_yank_lines(&doc, 1);
    assert_eq!(state.yank_register, "Brst line\n");
    assert!(state.is_line_yank);

    // 4. Paste line below 'p'
    state.execute_paste(&mut doc, true);
    assert_eq!(
        doc,
        "Brst line\nBrst line\nSecond line\nThird line"
    );

    // 5. Delete entire line 'dd'
    state.cursor_pos = 0;
    state.execute_delete_lines(&mut doc, 1);
    assert_eq!(state.yank_register, "Brst line\n");
    assert_eq!(doc, "Brst line\nSecond line\nThird line");

    // 6. Delete to end of line 'D'
    state.cursor_pos = 4; // at " line"
    state.execute_delete_to_end_of_line(&mut doc);
    assert_eq!(state.yank_register, " line");
    assert!(!state.is_line_yank);
    assert!(doc.starts_with("Brst\nSecond"));
}

#[test]
fn test_vim_visual_selection_modes() {
    let text = "Line one\nLine two\nLine three";
    let mut state = VimState::new();

    // Visual mode selection: characters 2 through 6
    state.cursor_pos = 2;
    state.set_mode(VimMode::Visual, text);
    state.cursor_pos = 6;
    let (start, end) = state.get_visual_selection(text);
    assert_eq!(start, 2);
    assert_eq!(end, 7);
    assert_eq!(&text[start..end], "ne on");

    // Visual line mode selection: select line 1 and line 2
    state.cursor_pos = 3; // line 1
    state.set_mode(VimMode::VisualLine, text);
    state.cursor_pos = 12; // line 2
    let (vl_start, vl_end) = state.get_visual_selection(text);
    assert_eq!(vl_start, 0); // start of Line one
    assert_eq!(vl_end, 18); // end of Line two + \n
    assert_eq!(&text[vl_start..vl_end], "Line one\nLine two\n");
}

#[test]
fn test_vim_preference_persistence() {
    let store = SqliteStore::open_in_memory().expect("open in-memory sqlite store");

    // Default: preference not set
    let default_val = store.get_setting("activateVimMode").expect("query setting");
    assert!(default_val.is_none());

    // Enable Vim mode
    store
        .set_setting("activateVimMode", serde_json::json!(true))
        .expect("save activateVimMode true");

    let val = store
        .get_setting("activateVimMode")
        .expect("query setting")
        .expect("value present");
    assert_eq!(val, serde_json::json!(true));

    // Disable Vim mode
    store
        .set_setting("activateVimMode", serde_json::json!(false))
        .expect("save activateVimMode false");

    let val2 = store
        .get_setting("activateVimMode")
        .expect("query setting")
        .expect("value present");
    assert_eq!(val2, serde_json::json!(false));

    // Verify in full get_settings() map
    let all = store.get_settings().expect("get all settings");
    assert_eq!(all.get("activateVimMode"), Some(&serde_json::json!(false)));
}
