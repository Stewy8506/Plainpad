use plainpad_core::typography::{
    apply_inline_format, cycle_heading, cycle_heading_at_cursor, is_list_line, CaretNudgeConfig,
    InlineFormat, RtlConfig,
};

#[test]
fn test_heading_cycle_transitions() {
    // 1. Single line cycles: Body -> # -> ## -> ### -> Body
    let body = "Plain text note title";
    let h1 = cycle_heading(body);
    assert_eq!(h1, "# Plain text note title");

    let h2 = cycle_heading(&h1);
    assert_eq!(h2, "## Plain text note title");

    let h3 = cycle_heading(&h2);
    assert_eq!(h3, "### Plain text note title");

    let back_to_body = cycle_heading(&h3);
    assert_eq!(back_to_body, "Plain text note title");

    // 2. Multiline document: cycle heading on target line while preserving other lines and cursor
    let doc = "First line\nTarget heading\nThird line";
    // Cursor at 'g' of Target heading:
    // "First line\n" is 11 chars. "Target headin" is 13 chars -> pos 24
    let (new_doc, new_pos) = cycle_heading_at_cursor(doc, 24);
    assert_eq!(new_doc, "First line\n# Target heading\nThird line");
    assert_eq!(new_pos, 26); // shifted by +2 ('# ')

    let (new_doc2, new_pos2) = cycle_heading_at_cursor(&new_doc, new_pos);
    assert_eq!(new_doc2, "First line\n## Target heading\nThird line");
    assert_eq!(new_pos2, 27); // shifted by +1 ('#' inserted)

    let (new_doc3, new_pos3) = cycle_heading_at_cursor(&new_doc2, new_pos2);
    assert_eq!(new_doc3, "First line\n### Target heading\nThird line");
    assert_eq!(new_pos3, 28); // shifted by +1 ('#' inserted)

    let (new_doc4, new_pos4) = cycle_heading_at_cursor(&new_doc3, new_pos3);
    assert_eq!(new_doc4, "First line\nTarget heading\nThird line");
    assert_eq!(new_pos4, 24); // shifted by -4 ('### ' removed)
}

#[test]
fn test_inline_formatting_shortcuts() {
    let doc = "The quick brown fox jumps";

    // 1. Bold wrap
    // "brown": index 10 to 15
    let (bold_doc, start, end) = apply_inline_format(doc, 10, 15, InlineFormat::Bold);
    assert_eq!(bold_doc, "The quick **brown** fox jumps");
    assert_eq!(start, 12);
    assert_eq!(end, 17);

    // 2. Bold unwrap (selection contains tags)
    let (unbold_doc, u_start, u_end) = apply_inline_format(&bold_doc, 10, 19, InlineFormat::Bold);
    assert_eq!(unbold_doc, "The quick brown fox jumps");
    assert_eq!(u_start, 10);
    assert_eq!(u_end, 15);

    // 3. Bold unwrap (selection inside outer tags)
    let (unbold_outer, o_start, o_end) = apply_inline_format(&bold_doc, 12, 17, InlineFormat::Bold);
    assert_eq!(unbold_outer, "The quick brown fox jumps");
    assert_eq!(o_start, 10);
    assert_eq!(o_end, 15);

    // 4. Empty selection -> insert token pair and place cursor between
    let (empty_doc, c_start, c_end) = apply_inline_format(doc, 4, 4, InlineFormat::Bold);
    assert_eq!(empty_doc, "The ****quick brown fox jumps");
    assert_eq!(c_start, 6);
    assert_eq!(c_end, 6);

    // 5. Italic (*...*)
    let (italic_doc, i_start, i_end) = apply_inline_format(doc, 4, 9, InlineFormat::Italic);
    assert_eq!(italic_doc, "The *quick* brown fox jumps");
    assert_eq!(i_start, 5);
    assert_eq!(i_end, 10);

    // 6. Strikethrough (~~...~~)
    let (strike_doc, s_start, s_end) = apply_inline_format(doc, 0, 3, InlineFormat::Strike);
    assert_eq!(strike_doc, "~~The~~ quick brown fox jumps");
    assert_eq!(s_start, 2);
    assert_eq!(s_end, 5);

    // 7. Underline (<u>...</u>)
    let (underline_doc, u_start2, u_end2) = apply_inline_format(doc, 20, 25, InlineFormat::Underline);
    assert_eq!(underline_doc, "The quick brown fox <u>jumps</u>");
    assert_eq!(u_start2, 23);
    assert_eq!(u_end2, 28);
}

#[test]
fn test_list_line_detection() {
    // Checklists
    assert!(is_list_line("- [ ] buy milk"));
    assert!(is_list_line("- [x] buy eggs"));
    assert!(is_list_line("- [X] completed task"));
    assert!(is_list_line("* [ ] star checklist"));
    assert!(is_list_line("[ ] square checklist"));
    assert!(is_list_line("   - [ ] indented checklist"));

    // Bullet lists
    assert!(is_list_line("- simple dash item"));
    assert!(is_list_line("* asterisk bullet item"));
    assert!(is_list_line("+ plus bullet item"));
    assert!(is_list_line("  - indented bullet"));

    // Numbered lists
    assert!(is_list_line("1. first step"));
    assert!(is_list_line("2. second step"));
    assert!(is_list_line("10. tenth step"));
    assert!(is_list_line("   3. indented numbered"));

    // Non-list regular lines
    assert!(!is_list_line("Regular prose line in a note"));
    assert!(!is_list_line("Not a list item: 3.14159 is pi"));
    assert!(!is_list_line("-not-a-list"));
    assert!(!is_list_line("# Heading"));
    assert!(!is_list_line(""));
}

#[test]
fn test_caret_nudge_config_and_active_calculation() {
    let mut config = CaretNudgeConfig::default();
    assert_eq!(config.base_nudge_px, 0.0);
    assert_eq!(config.list_nudge_px, 0.0);
    assert_eq!(config.active_nudge(false), 0.0);
    assert_eq!(config.active_nudge(true), 0.0);

    // Configure custom font baseline adjustment: +0.5px base, -0.75px for list bullets
    config.base_nudge_px = 0.5;
    config.list_nudge_px = -0.75;

    // Body line receives base nudge
    assert_eq!(config.active_nudge(false), 0.5);

    // List item receives combined compensation
    assert_eq!(config.active_nudge(true), -0.25);

    // JSON serialization / deserialization roundtrip
    let json = serde_json::to_string(&config).unwrap();
    let deserialized: CaretNudgeConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, config);
}

#[test]
fn test_rtl_config_and_persistence() {
    let rtl_default = RtlConfig::default();
    assert!(!rtl_default.force_rtl);

    let rtl_enabled = RtlConfig { force_rtl: true };
    assert!(rtl_enabled.force_rtl);

    let json = serde_json::to_string(&rtl_enabled).unwrap();
    assert_eq!(json, r#"{"force_rtl":true}"#);

    let deserialized: RtlConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized, rtl_enabled);
}
