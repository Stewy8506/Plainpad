// Integration tests for Plainpad third-party export suite (REQ-30)

use chrono::TimeZone;
use plainpad_core::export::{
    export_all_to_zip, format_apple_notes, format_bear_uri, format_custom_url, format_notion,
    format_obsidian_uri, omit_keywords_from_text, to_markdown_with_options, ExportOptions,
};
use plainpad_core::Note;

#[test]
fn test_markdown_conversion_headings_and_tasks() {
    let raw = "Sprint Backlog\n\n[ ] Implement auth\n[x] Setup SQLite\n[X] Initial commit\ntodo Add tests\n- Review PR\n* Polish styling\n• Check docs\nNormal text line";
    let opts = ExportOptions {
        use_first_line_as_title: true,
        omit_keywords: false,
        convert_to_markdown: true,
        convert_to_rich_text: false,
    };
    let md = to_markdown_with_options(raw, &opts);

    assert!(md.starts_with("# Sprint Backlog\n\n"));
    assert!(md.contains("- [ ] Implement auth\n"));
    assert!(md.contains("- [x] Setup SQLite\n"));
    assert!(md.contains("- [x] Initial commit\n"));
    assert!(md.contains("- [ ] Add tests\n"));
    assert!(md.contains("- Review PR\n"));
    assert!(md.contains("- Polish styling\n"));
    assert!(md.contains("- Check docs\n"));
    assert!(md.contains("Normal text line\n"));
}

#[test]
fn test_obsidian_uri_formatting_and_escaping() {
    let uri = format_obsidian_uri(
        "Work & Personal",
        "Meeting with Alice & Bob",
        "Agenda:\n1. 100% completed\n2. Next steps!",
    );

    assert!(uri.starts_with("obsidian://new?"));
    assert!(uri.contains("vault=Work%20%26%20Personal"));
    assert!(uri.contains("name=Meeting%20with%20Alice%20%26%20Bob"));
    assert!(uri.contains("content=Agenda%3A%0A1.%20100%25%20completed%0A2.%20Next%20steps%21"));
}

#[test]
fn test_bear_uri_formatting() {
    let uri = format_bear_uri("Quick Note", "# Heading\nSome text");
    assert!(uri.starts_with("bear://x-callback-url/create?"));
    assert!(uri.contains("title=Quick%20Note"));
    assert!(uri.contains("text=%23%20Heading%0ASome%20text"));
}

#[test]
fn test_custom_url_template_token_replacement() {
    let template = "myapp://create?title={TITLE}&text={CONTENT}&date={DATE}&datetime={DATE_TIME}";
    let dt = chrono::Utc.with_ymd_and_hms(2026, 10, 16, 14, 30, 0).unwrap();

    let resolved = format_custom_url(
        template,
        "Project Plan / Roadmap",
        "Line 1\nLine 2",
        dt,
    );

    assert!(resolved.contains("title=Project%20Plan%20%2F%20Roadmap"));
    assert!(resolved.contains("text=Line%201%0ALine%202"));
    assert!(resolved.contains("date=2026-10-16"));
    assert!(resolved.contains("datetime=2026-10-16T14%3A30%3A00Z"));
}

#[test]
fn test_keyword_omission() {
    let raw = "sum: 12 + 34\nmath 50 * 2\navg: 10, 20, 30\nlist: buy groceries\n::gpt(\"explain quantum physics\")\nregular line without triggers";
    let stripped = omit_keywords_from_text(raw);

    assert!(stripped.contains("12 + 34"));
    assert!(!stripped.contains("sum:"));
    assert!(stripped.contains("50 * 2"));
    assert!(!stripped.contains("math "));
    assert!(stripped.contains("10, 20, 30"));
    assert!(!stripped.contains("avg:"));
    assert!(stripped.contains("buy groceries"));
    assert!(!stripped.contains("list:"));
    assert!(!stripped.contains("::gpt"));
    assert!(stripped.contains("regular line without triggers"));
}

#[test]
fn test_apple_notes_and_notion_formats() {
    let opts_rich = ExportOptions {
        convert_to_rich_text: true,
        ..Default::default()
    };
    let apple_html = format_apple_notes("My Title", "Paragraph 1\nParagraph 2", &opts_rich);
    assert!(apple_html.contains("<!DOCTYPE html>"));
    assert!(apple_html.contains("<h1>My Title</h1>"));
    assert!(apple_html.contains("<p>Paragraph 1</p>"));

    let notion_out = format_notion("My Note", "Some content", Some("Projects DB"));
    assert!(notion_out.contains("# My Note"));
    assert!(notion_out.contains("Database: Projects DB"));
    assert!(notion_out.contains("Some content"));
}

#[test]
fn test_bulk_zip_archive_generation() {
    let note1 = Note::new("Project Alpha\nInitial design notes");
    let note2 = Note::new("Project Alpha\nFollow up notes"); // duplicate title testing
    let mut note3 = Note::new("Deleted Note\nShould not be exported");
    note3.soft_deleted = true;

    let notes = vec![note1, note2, note3];
    let opts = ExportOptions {
        use_first_line_as_title: true,
        convert_to_markdown: false,
        omit_keywords: false,
        convert_to_rich_text: false,
    };

    let zip_bytes = export_all_to_zip(&notes, &opts).expect("ZIP export should succeed");

    // Standard ZIP signature: PK\x03\x04
    assert_eq!(&zip_bytes[0..4], &[0x50, 0x4b, 0x03, 0x04]);

    // End of Central Directory signature: PK\x05\x06
    let eocd_sig = [0x50, 0x4b, 0x05, 0x06];
    assert!(zip_bytes.windows(4).any(|w| w == eocd_sig));

    // Check entry filenames exist inside archive
    let content_str = String::from_utf8_lossy(&zip_bytes);
    assert!(content_str.contains("project-alpha.txt"));
    assert!(content_str.contains("project-alpha-1.txt"));
    assert!(!content_str.contains("deleted-note"));
}
