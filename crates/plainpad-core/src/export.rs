//! Export: plain text (identity) and Markdown conversion of note structure
//! (PRD E-1). The note text is the source of truth; export only decorates.

use crate::title_from_text;

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ExportKind {
    Txt,
    Markdown,
}

/// Convert a note to Markdown: first non-empty line becomes the title heading,
/// `[ ]`/`[x]` prefixes become GitHub task-list bullets, dash/bullet/`todo`
/// prefixes become list items, everything else stays as written.
pub fn to_markdown(text: &str) -> String {
    let mut out = String::new();
    let mut title_emitted = false;
    for line in text.lines() {
        let t = line.trim_start();
        if !title_emitted && !t.is_empty() {
            out.push_str(&format!("# {}\n", t.trim_end()));
            title_emitted = true;
            continue;
        }
        if let Some(rest) = t.strip_prefix("[ ] ") {
            out.push_str(&format!("- [ ] {rest}\n"));
        } else if let Some(rest) = t.strip_prefix("[x] ").or_else(|| t.strip_prefix("[X] ")) {
            out.push_str(&format!("- [x] {rest}\n"));
        } else if let Some(rest) = t.strip_prefix("todo ").or_else(|| t.strip_prefix("Todo ")) {
            out.push_str(&format!("- [ ] {rest}\n"));
        } else if t.starts_with("- ") || t.starts_with("* ") || t.starts_with("• ") {
            out.push_str(&format!("- {}\n", &t[2..]));
        } else {
            out.push_str(line.trim_end());
            out.push('\n');
        }
    }
    out
}

/// Export a note in the requested kind. Txt is the identity (plain text IS the
/// note); Markdown applies the structural conversion above.
pub fn export(text: &str, kind: ExportKind) -> String {
    match kind {
        ExportKind::Txt => text.to_string(),
        ExportKind::Markdown => to_markdown(text),
    }
}

/// Suggested file name for an export (title-derived, filesystem-safe).
pub fn export_file_name(text: &str, kind: ExportKind) -> String {
    let title = title_from_text(text);
    let base: String = title
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' => c,
            ' ' => '-',
            _ => '_',
        })
        .collect();
    let base = base.trim_matches(['-', '_']).to_string();
    let base = if base.is_empty() { "note".to_string() } else { base.to_lowercase() };
    match kind {
        ExportKind::Txt => format!("{base}.txt"),
        ExportKind::Markdown => format!("{base}.md"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_converts_structure() {
        let md = to_markdown("Groceries\n\nbuy milk\n[ ] eggs\n[x] bread\n- butter\n");
        assert!(md.starts_with("# Groceries\n"));
        assert!(md.contains("- [ ] buy milk") || md.contains("buy milk"));
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
}
