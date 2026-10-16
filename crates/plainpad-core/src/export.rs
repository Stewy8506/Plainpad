//! Export: plain text, Markdown, third-party note formats, URI schemes,
//! and bulk ZIP backup archive.

use crate::model::Note;
use crate::title_from_text;
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportKind {
    Txt,
    Markdown,
}

#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ExportOptions {
    pub use_first_line_as_title: bool,
    pub omit_keywords: bool,
    pub convert_to_markdown: bool,
    pub convert_to_rich_text: bool,
}

/// Sanitizes a string into a filesystem-safe base name.
pub fn sanitize_filename(name: &str) -> String {
    let safe: String = name
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => c,
            ' ' => '-',
            _ => '_',
        })
        .collect();
    let safe = safe.trim_matches(['-', '_']).to_string();
    if safe.is_empty() {
        "note".to_string()
    } else {
        safe.to_lowercase()
    }
}

/// Suggested file name for an export (title-derived, filesystem-safe).
pub fn export_file_name(text: &str, kind: ExportKind) -> String {
    let title = title_from_text(text);
    let base = sanitize_filename(&title);
    match kind {
        ExportKind::Txt => format!("{base}.txt"),
        ExportKind::Markdown => format!("{base}.md"),
    }
}

/// Extracts title and body from note text based on `use_first_line_as_title`.
pub fn extract_title_and_body(text: &str, use_first_line_as_title: bool) -> (String, String) {
    if !use_first_line_as_title {
        return (title_from_text(text), text.to_string());
    }

    let mut title = String::new();
    let mut body_lines = Vec::new();
    let mut found_title = false;

    for line in text.lines() {
        if !found_title && !line.trim().is_empty() {
            title = line.trim().to_string();
            found_title = true;
        } else if found_title {
            body_lines.push(line);
        }
    }

    if title.is_empty() {
        title = title_from_text(text);
    }

    (title, body_lines.join("\n"))
}

/// Strips natural language triggers and double-colon commands from exported text.
pub fn omit_keywords_from_text(text: &str) -> String {
    let triggers = [
        "sum:", "sum ", "total:", "total ", "avg:", "avg ", "average:", "average ",
        "math:", "math ", "calcs:", "calcs ", "list:", "list ", "count:", "count ",
        "code:", "code ", "paste:", "paste ", "timer:", "timer ", "check:", "check ",
        "todo:", "todo ",
    ];

    let mut out_lines = Vec::new();
    for line in text.lines() {
        let trimmed_start = line.trim_start();
        let lower = trimmed_start.to_lowercase();

        // Check command trigger ::commandName(...) or ::commandName
        if let Some(cmd_idx) = trimmed_start.find("::") {
            if cmd_idx == 0 {
                if let Some(paren_close) = trimmed_start.find(')') {
                    let after = trimmed_start[paren_close + 1..].trim_start();
                    out_lines.push(after.to_string());
                    continue;
                } else {
                    let after = trimmed_start[2..].trim_start();
                    let space_idx = after.find(' ').unwrap_or(after.len());
                    let remaining = after[space_idx..].trim_start();
                    out_lines.push(remaining.to_string());
                    continue;
                }
            }
        }

        let mut stripped = false;
        for trig in &triggers {
            if lower.starts_with(trig) {
                let rest = trimmed_start[trig.len()..].trim_start();
                out_lines.push(rest.to_string());
                stripped = true;
                break;
            }
        }

        if !stripped {
            let single_words = [
                "math", "calcs", "sum", "avg", "list", "code", "paste", "timer", "check", "todo",
            ];
            if single_words.contains(&lower.trim()) {
                out_lines.push(String::new());
            } else {
                out_lines.push(line.to_string());
            }
        }
    }

    out_lines.join("\n")
}

/// Convert note text to Markdown with structural conversions and options.
pub fn to_markdown_with_options(text: &str, options: &ExportOptions) -> String {
    let processed_text = if options.omit_keywords {
        omit_keywords_from_text(text)
    } else {
        text.to_string()
    };

    let (title, body) = if options.use_first_line_as_title {
        extract_title_and_body(&processed_text, true)
    } else {
        (String::new(), processed_text)
    };

    let mut out = String::new();
    if !title.is_empty() {
        out.push_str(&format!("# {}\n\n", title));
    }

    for line in body.lines() {
        let t = line.trim_start();
        if let Some(rest) = t.strip_prefix("[ ] ") {
            out.push_str(&format!("- [ ] {rest}\n"));
        } else if let Some(rest) = t.strip_prefix("[x] ").or_else(|| t.strip_prefix("[X] ")) {
            out.push_str(&format!("- [x] {rest}\n"));
        } else if let Some(rest) = t.strip_prefix("todo ").or_else(|| t.strip_prefix("Todo ")) {
            out.push_str(&format!("- [ ] {rest}\n"));
        } else if let Some(rest) = t.strip_prefix("- ").or_else(|| t.strip_prefix("* ")).or_else(|| t.strip_prefix("• ")) {
            out.push_str(&format!("- {rest}\n"));
        } else {
            out.push_str(line.trim_end());
            out.push('\n');
        }
    }

    out
}

/// Convert a note to Markdown with default options.
pub fn to_markdown(text: &str) -> String {
    to_markdown_with_options(
        text,
        &ExportOptions {
            use_first_line_as_title: true,
            ..Default::default()
        },
    )
}

/// Export a note in the requested kind (Txt or Markdown).
pub fn export(text: &str, kind: ExportKind) -> String {
    match kind {
        ExportKind::Txt => text.to_string(),
        ExportKind::Markdown => to_markdown(text),
    }
}

/// Percent-encodes a string according to RFC 3986 unreserved characters.
pub fn percent_encode(input: &str) -> String {
    let mut out = String::with_capacity(input.len() * 3);
    for &b in input.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char);
            }
            _ => {
                out.push_str(&format!("%{:02X}", b));
            }
        }
    }
    out
}

/// Formats an Obsidian vault URL scheme: `obsidian://new?vault={VAULT}&name={TITLE}&content={CONTENT}`.
pub fn format_obsidian_uri(vault: &str, title: &str, content: &str) -> String {
    format!(
        "obsidian://new?vault={}&name={}&content={}",
        percent_encode(vault),
        percent_encode(title),
        percent_encode(content)
    )
}

/// Formats a Bear URL scheme: `bear://x-callback-url/create?title={TITLE}&text={CONTENT}`.
pub fn format_bear_uri(title: &str, content: &str) -> String {
    format!(
        "bear://x-callback-url/create?title={}&text={}",
        percent_encode(title),
        percent_encode(content)
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#039;")
}

/// Formats Apple Notes content (either rich HTML or text).
pub fn format_apple_notes(title: &str, content: &str, options: &ExportOptions) -> String {
    if options.convert_to_rich_text {
        let paragraphs = content
            .lines()
            .map(|l| format!("<p>{}</p>", html_escape(l)))
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "<!DOCTYPE html><html><head><meta charset=\"utf-8\"><title>{}</title></head><body><h1>{}</h1>\n{}</body></html>",
            html_escape(title),
            html_escape(title),
            paragraphs
        )
    } else {
        format!("# {}\n\n{}", title, content)
    }
}

/// Formats Notion-ready markdown representation.
pub fn format_notion(title: &str, content: &str, database_name: Option<&str>) -> String {
    if let Some(db) = database_name {
        if !db.trim().is_empty() {
            return format!("# {}\n\nDatabase: {}\n\n{}", title, db.trim(), content);
        }
    }
    format!("# {}\n\n{}", title, content)
}

/// Substitutes `{TITLE}`, `{CONTENT}`, `{DATE}`, and `{DATE_TIME}` tokens in user URL templates.
pub fn format_custom_url(
    template: &str,
    title: &str,
    content: &str,
    now: chrono::DateTime<chrono::Utc>,
) -> String {
    let date_str = now.format("%Y-%m-%d").to_string();
    let date_time_str = now.format("%Y-%m-%dT%H:%M:%SZ").to_string();

    template
        .replace("{TITLE}", &percent_encode(title))
        .replace("{CONTENT}", &percent_encode(content))
        .replace("{DATE}", &date_str)
        .replace("{DATE_TIME}", &percent_encode(&date_time_str))
}

/// Standard IEEE 802.3 CRC-32 checksum calculation.
pub fn crc32(data: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc ^= b as u32;
        for _ in 0..8 {
            if crc & 1 != 0 {
                crc = (crc >> 1) ^ 0xEDB8_8320;
            } else {
                crc >>= 1;
            }
        }
    }
    !crc
}

/// Pure Rust lightweight standard ZIP archive writer (compression method 0: Stored).
pub struct ZipArchiveWriter {
    buffer: Vec<u8>,
    entries: Vec<ZipEntry>,
}

struct ZipEntry {
    filename: String,
    crc: u32,
    uncompressed_size: u32,
    offset: u32,
}

impl Default for ZipArchiveWriter {
    fn default() -> Self {
        Self::new()
    }
}

impl ZipArchiveWriter {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            entries: Vec::new(),
        }
    }

    pub fn add_file(&mut self, filename: &str, data: &[u8]) {
        let offset = self.buffer.len() as u32;
        let crc = crc32(data);
        let size = data.len() as u32;
        let fn_bytes = filename.as_bytes();
        let fn_len = fn_bytes.len() as u16;

        // Local file header: signature 0x04034b50
        self.buffer.extend_from_slice(&[0x50, 0x4b, 0x03, 0x04]);
        self.buffer.extend_from_slice(&[0x14, 0x00]); // version needed (2.0)
        self.buffer.extend_from_slice(&[0x00, 0x08]); // flags (bit 11 = UTF-8)
        self.buffer.extend_from_slice(&[0x00, 0x00]); // compression = 0 (stored)
        self.buffer.extend_from_slice(&[0x00, 0x00]); // mod time
        self.buffer.extend_from_slice(&[0x21, 0x5d]); // mod date (2026-10-16)
        self.buffer.extend_from_slice(&crc.to_le_bytes());
        self.buffer.extend_from_slice(&size.to_le_bytes()); // compressed size
        self.buffer.extend_from_slice(&size.to_le_bytes()); // uncompressed size
        self.buffer.extend_from_slice(&fn_len.to_le_bytes());
        self.buffer.extend_from_slice(&[0x00, 0x00]); // extra field length
        self.buffer.extend_from_slice(fn_bytes);
        self.buffer.extend_from_slice(data);

        self.entries.push(ZipEntry {
            filename: filename.to_string(),
            crc,
            uncompressed_size: size,
            offset,
        });
    }

    pub fn finish(mut self) -> Vec<u8> {
        let central_dir_start = self.buffer.len() as u32;

        for entry in &self.entries {
            let fn_bytes = entry.filename.as_bytes();
            let fn_len = fn_bytes.len() as u16;

            // Central directory file header: signature 0x02014b50
            self.buffer.extend_from_slice(&[0x50, 0x4b, 0x01, 0x02]);
            self.buffer.extend_from_slice(&[0x14, 0x00]); // version made by
            self.buffer.extend_from_slice(&[0x14, 0x00]); // version needed
            self.buffer.extend_from_slice(&[0x00, 0x08]); // flags (UTF-8)
            self.buffer.extend_from_slice(&[0x00, 0x00]); // compression = 0 (stored)
            self.buffer.extend_from_slice(&[0x00, 0x00]); // mod time
            self.buffer.extend_from_slice(&[0x21, 0x5d]); // mod date
            self.buffer.extend_from_slice(&entry.crc.to_le_bytes());
            self.buffer.extend_from_slice(&entry.uncompressed_size.to_le_bytes());
            self.buffer.extend_from_slice(&entry.uncompressed_size.to_le_bytes());
            self.buffer.extend_from_slice(&fn_len.to_le_bytes());
            self.buffer.extend_from_slice(&[0x00, 0x00]); // extra field length
            self.buffer.extend_from_slice(&[0x00, 0x00]); // comment length
            self.buffer.extend_from_slice(&[0x00, 0x00]); // disk number start
            self.buffer.extend_from_slice(&[0x00, 0x00]); // internal attributes
            self.buffer.extend_from_slice(&[0x00, 0x00, 0xa4, 0x81]); // external attributes (0100644)
            self.buffer.extend_from_slice(&entry.offset.to_le_bytes());
            self.buffer.extend_from_slice(fn_bytes);
        }

        let central_dir_size = (self.buffer.len() as u32) - central_dir_start;
        let count = self.entries.len() as u16;

        // End of central directory record (EOCD): signature 0x06054b50
        self.buffer.extend_from_slice(&[0x50, 0x4b, 0x05, 0x06]);
        self.buffer.extend_from_slice(&[0x00, 0x00]); // number of this disk
        self.buffer.extend_from_slice(&[0x00, 0x00]); // disk where central directory starts
        self.buffer.extend_from_slice(&count.to_le_bytes()); // entries on this disk
        self.buffer.extend_from_slice(&count.to_le_bytes()); // total entries
        self.buffer.extend_from_slice(&central_dir_size.to_le_bytes());
        self.buffer.extend_from_slice(&central_dir_start.to_le_bytes());
        self.buffer.extend_from_slice(&[0x00, 0x00]); // comment length

        self.buffer
    }
}

/// Bundles all non-void notes into a standard ZIP archive of text or markdown files.
pub fn export_all_to_zip(notes: &[Note], options: &ExportOptions) -> Result<Vec<u8>, String> {
    let mut zip_writer = ZipArchiveWriter::new();
    let mut seen_names = HashSet::new();

    for note in notes {
        if note.soft_deleted {
            continue;
        }

        let (title, _) = extract_title_and_body(&note.content, options.use_first_line_as_title);
        let content_to_export = if options.convert_to_markdown {
            to_markdown_with_options(&note.content, options)
        } else if options.omit_keywords {
            omit_keywords_from_text(&note.content)
        } else {
            note.content.clone()
        };

        let ext = if options.convert_to_markdown { "md" } else { "txt" };
        let base_title = if title.is_empty() {
            "note".to_string()
        } else {
            title
        };
        let safe_base = sanitize_filename(&base_title);

        let mut candidate = format!("{safe_base}.{ext}");
        let mut counter = 1;
        while seen_names.contains(&candidate) {
            candidate = format!("{safe_base}-{counter}.{ext}");
            counter += 1;
        }
        seen_names.insert(candidate.clone());

        zip_writer.add_file(&candidate, content_to_export.as_bytes());
    }

    Ok(zip_writer.finish())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_converts_structure() {
        let md = to_markdown("Groceries\n\nbuy milk\n[ ] eggs\n[x] bread\n- butter\n");
        assert!(md.starts_with("# Groceries\n"));
        assert!(md.contains("- [ ] eggs"));
        assert!(md.contains("- [x] bread"));
        assert!(md.contains("- butter"));
    }

    #[test]
    fn txt_is_identity_and_name_is_safe() {
        let text = "My Note!\nbody";
        assert_eq!(export(text, ExportKind::Txt), text);
        assert_eq!(export_file_name(text, ExportKind::Txt), "my-note.txt");
        assert_eq!(export_file_name("", ExportKind::Markdown), "note.md");
    }

    #[test]
    fn crc32_standard_check() {
        // Standard check: "123456789" => 0xCBF43926
        assert_eq!(crc32(b"123456789"), 0xCBF43926);
    }
}
