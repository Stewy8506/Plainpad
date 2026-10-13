use plainpad_core::intent::analyze_with_config;
use plainpad_core::trigger_config::TriggerConfig;

#[test]
fn test_sum_over_preceding_currency_lines() {
    let text = "$10.50\n$20.25\ntotal";
    let analysis = analyze_with_config(text, false, &TriggerConfig::default());
    let anno = analysis.annotations.iter().find(|a| a.line == 2).expect("sum annotation");
    assert_eq!(anno.kind, "math");
    assert_eq!(anno.value, "$30.75");
}

#[test]
fn test_sum_with_aliases_add_and_sum() {
    let text_add = "15\n25\nadd";
    let analysis_add = analyze_with_config(text_add, false, &TriggerConfig::default());
    let anno_add = analysis_add.annotations.iter().find(|a| a.line == 2).expect("add annotation");
    assert_eq!(anno_add.kind, "math");
    assert_eq!(anno_add.value, "40");

    let text_sum = "100\n200\nsum";
    let analysis_sum = analyze_with_config(text_sum, false, &TriggerConfig::default());
    let anno_sum = analysis_sum.annotations.iter().find(|a| a.line == 2).expect("sum annotation");
    assert_eq!(anno_sum.kind, "math");
    assert_eq!(anno_sum.value, "300");
}

#[test]
fn test_avg_over_preceding_lines() {
    let text = "10\n20\n30\navg";
    let analysis = analyze_with_config(text, false, &TriggerConfig::default());
    let anno = analysis.annotations.iter().find(|a| a.line == 3).expect("avg annotation");
    assert_eq!(anno.kind, "math");
    assert_eq!(anno.value, "20");

    let text_avg_cur = "$10.00\n$20.00\n$30.00\naverage";
    let analysis_cur = analyze_with_config(text_avg_cur, false, &TriggerConfig::default());
    let anno_cur = analysis_cur.annotations.iter().find(|a| a.line == 3).expect("average annotation");
    assert_eq!(anno_cur.kind, "math");
    assert_eq!(anno_cur.value, "$20");
}

#[test]
fn test_count_on_list_items() {
    let text = "Task A\nTask B\nTask C\nTask D\ncount";
    let analysis = analyze_with_config(text, false, &TriggerConfig::default());
    let anno = analysis.annotations.iter().find(|a| a.line == 4).expect("count annotation");
    assert_eq!(anno.kind, "stats");
    assert_eq!(anno.value, "4 items");

    // Single comma-separated line count
    let text_single = "apples, oranges, bananas\ncount";
    let analysis_single = analyze_with_config(text_single, false, &TriggerConfig::default());
    let anno_single = analysis_single.annotations.iter().find(|a| a.line == 1).expect("count annotation");
    assert_eq!(anno_single.kind, "stats");
    assert_eq!(anno_single.value, "3 items");
}

#[test]
fn test_paragraph_boundary_isolation() {
    let text = "100\n200\n\n10\n20\nsum";
    let analysis = analyze_with_config(text, false, &TriggerConfig::default());
    let anno = analysis.annotations.iter().find(|a| a.line == 5).expect("sum annotation");
    assert_eq!(anno.kind, "math");
    // Should ONLY sum 10 + 20 = 30, isolated from the preceding paragraph (100, 200)
    assert_eq!(anno.value, "30");

    let text_heading = "# Previous Section\n100\n200\n# Current Section\n15\n25\ntotal";
    let analysis_h = analyze_with_config(text_heading, false, &TriggerConfig::default());
    let anno_h = analysis_h.annotations.iter().find(|a| a.line == 6).expect("total annotation");
    assert_eq!(anno_h.kind, "math");
    assert_eq!(anno_h.value, "40");
}

#[test]
fn test_math_trigger_with_trailing_expression() {
    let text = "math 15 * 6";
    let analysis = analyze_with_config(text, false, &TriggerConfig::default());
    let anno = analysis.annotations.iter().find(|a| a.line == 0).expect("math trigger annotation");
    assert_eq!(anno.kind, "math");
    assert_eq!(anno.value, "90");

    let text_colon = "math: 100 / 4";
    let analysis_colon = analyze_with_config(text_colon, false, &TriggerConfig::default());
    let anno_colon = analysis_colon.annotations.iter().find(|a| a.line == 0).expect("math colon annotation");
    assert_eq!(anno_colon.kind, "math");
    assert_eq!(anno_colon.value, "25");
}

#[test]
fn test_disable_all_keywords_master_toggle() {
    let mut config = TriggerConfig::default();
    config.disable_all_keywords = true;

    let text = "10\n20\nsum\nmath 15 * 6\nlist\ncheck\ncode\npaste\ntimer 5";
    let analysis = analyze_with_config(text, false, &config);

    // No annotations should be produced for trigger keyword lines
    assert!(!analysis.annotations.iter().any(|a| a.line == 2)); // sum
    assert!(!analysis.annotations.iter().any(|a| a.line == 3)); // math 15 * 6
    assert!(!analysis.annotations.iter().any(|a| a.line == 4)); // list
    assert!(!analysis.annotations.iter().any(|a| a.line == 5)); // check
    assert!(!analysis.annotations.iter().any(|a| a.line == 6)); // code
    assert!(!analysis.annotations.iter().any(|a| a.line == 7)); // paste
    assert!(!analysis.annotations.iter().any(|a| a.line == 8)); // timer 5
}

#[test]
fn test_allow_arguments_after_keyword_toggle() {
    let mut config_allowed = TriggerConfig::default();
    config_allowed.allow_arguments_after_keyword = true;

    let text = "math 15 * 6\ncode: rust\ntimer 10m";
    let a_allowed = analyze_with_config(text, false, &config_allowed);
    assert!(a_allowed.annotations.iter().any(|a| a.line == 0 && a.kind == "math"));
    assert!(a_allowed.annotations.iter().any(|a| a.line == 1 && a.kind == "code" && a.value == "rust"));
    assert!(a_allowed.annotations.iter().any(|a| a.line == 2 && a.kind == "timer"));

    let mut config_disallowed = TriggerConfig::default();
    config_disallowed.allow_arguments_after_keyword = false;

    let a_disallowed = analyze_with_config(text, false, &config_disallowed);
    // Arguments are disallowed, so inline trailing arguments do NOT trigger
    assert!(!a_disallowed.annotations.iter().any(|a| a.line == 0 && a.kind == "math" && a.value == "90"));
    assert!(!a_disallowed.annotations.iter().any(|a| a.line == 1 && a.kind == "code"));
}

#[test]
fn test_custom_user_aliases() {
    let mut config = TriggerConfig::default();
    config.sum_triggers.push("tally".to_string());
    config.count_triggers.push("howmany".to_string());

    let text = "12\n18\ntally\nItem 1\nItem 2\nhowmany";
    let analysis = analyze_with_config(text, false, &config);

    let anno_tally = analysis.annotations.iter().find(|a| a.line == 2).expect("tally annotation");
    assert_eq!(anno_tally.kind, "math");
    assert_eq!(anno_tally.value, "30");

    let anno_howmany = analysis.annotations.iter().find(|a| a.line == 5).expect("howmany annotation");
    assert_eq!(anno_howmany.kind, "stats");
    assert_eq!(anno_howmany.value, "2 items");
}

#[test]
fn test_all_nine_trigger_categories_recognized() {
    let config = TriggerConfig::default();

    // 1. Sum
    let a1 = analyze_with_config("10\n20\nsum", false, &config);
    assert_eq!(a1.annotations.iter().find(|a| a.line == 2).unwrap().kind, "math");

    // 2. Avg
    let a2 = analyze_with_config("10\n20\navg", false, &config);
    assert_eq!(a2.annotations.iter().find(|a| a.line == 2).unwrap().kind, "math");

    // 3. Count
    let a3 = analyze_with_config("A\nB\ncount", false, &config);
    assert_eq!(a3.annotations.iter().find(|a| a.line == 2).unwrap().kind, "stats");

    // 4. Math
    let a4 = analyze_with_config("math 7 * 8", false, &config);
    assert_eq!(a4.annotations.iter().find(|a| a.line == 0).unwrap().value, "56");

    // 5. List
    let a5 = analyze_with_config("list", false, &config);
    let anno_list = a5.annotations.iter().find(|a| a.line == 0).unwrap();
    assert_eq!(anno_list.kind, "list");
    assert!(anno_list.chip.contains("bullet list"));

    // 6. Check
    let a6 = analyze_with_config("check", false, &config);
    let anno_check = a6.annotations.iter().find(|a| a.line == 0).unwrap();
    assert_eq!(anno_check.kind, "checklist");
    assert!(anno_check.chip.contains("checklist"));

    // 7. Code
    let a7 = analyze_with_config("code: python", false, &config);
    let anno_code = a7.annotations.iter().find(|a| a.line == 0).unwrap();
    assert_eq!(anno_code.kind, "code");
    assert_eq!(anno_code.value, "python");
    assert!(anno_code.chip.contains("code block"));

    // 8. Paste
    let a8 = analyze_with_config("paste", false, &config);
    let anno_paste = a8.annotations.iter().find(|a| a.line == 0).unwrap();
    assert_eq!(anno_paste.kind, "paste");
    assert!(anno_paste.chip.contains("AutoPaste"));

    // 9. Timer
    let a9 = analyze_with_config("timer 5m", false, &config);
    let anno_timer = a9.annotations.iter().find(|a| a.line == 0).unwrap();
    assert_eq!(anno_timer.kind, "timer");
    assert_eq!(anno_timer.value, "5 min");
}
