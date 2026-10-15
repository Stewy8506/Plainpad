//! Plainpad core — typography formatting, heading cycling, caret baseline adjustment, and RTL support.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeadingLevel {
    Body,
    H1,
    H2,
    H3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum InlineFormat {
    Bold,
    Italic,
    Strike,
    Underline,
}

impl InlineFormat {
    pub fn tags(&self) -> (&'static str, &'static str) {
        match self {
            InlineFormat::Bold => ("**", "**"),
            InlineFormat::Italic => ("*", "*"),
            InlineFormat::Strike => ("~~", "~~"),
            InlineFormat::Underline => ("<u>", "</u>"),
        }
    }
}

/// Cycle line prefix: Body -> # (H1) -> ## (H2) -> ### (H3) -> Body.
pub fn cycle_heading(line: &str) -> String {
    if let Some(rest) = line.strip_prefix("### ") {
        rest.to_string()
    } else if let Some(rest) = line.strip_prefix("## ") {
        format!("### {}", rest)
    } else if let Some(rest) = line.strip_prefix("# ") {
        format!("## {}", rest)
    } else {
        format!("# {}", line)
    }
}

/// Cycle heading level across an entire text string at the specified character index.
/// Returns (new_text, new_cursor_pos).
pub fn cycle_heading_at_cursor(text: &str, cursor_pos: usize) -> (String, usize) {
    let cursor_pos = cursor_pos.min(text.len());
    let line_start = text[..cursor_pos].rfind('\n').map(|idx| idx + 1).unwrap_or(0);
    let line_end = text[cursor_pos..].find('\n').map(|idx| cursor_pos + idx).unwrap_or(text.len());
    let line_text = &text[line_start..line_end];

    let (new_line, delta) = if let Some(rest) = line_text.strip_prefix("### ") {
        (rest.to_string(), -4isize)
    } else if let Some(rest) = line_text.strip_prefix("## ") {
        (format!("### {}", rest), 1isize)
    } else if let Some(rest) = line_text.strip_prefix("# ") {
        (format!("## {}", rest), 1isize)
    } else {
        (format!("# {}", line_text), 2isize)
    };

    let new_pos = if delta < 0 {
        cursor_pos.saturating_sub((-delta) as usize).max(line_start)
    } else {
        (cursor_pos + delta as usize).min(line_start + new_line.len())
    };

    let mut result = String::with_capacity(text.len() + 4);
    result.push_str(&text[..line_start]);
    result.push_str(&new_line);
    result.push_str(&text[line_end..]);

    (result, new_pos)
}

/// Apply inline formatting: wrap, unwrap if wrapped, or insert empty tokens if selection is collapsed.
/// Returns (new_text, new_start, new_end).
pub fn apply_inline_format(
    text: &str,
    sel_start: usize,
    sel_end: usize,
    format: InlineFormat,
) -> (String, usize, usize) {
    let (open, close) = format.tags();
    let text_len = text.len();
    let start = sel_start.min(text_len);
    let end = sel_end.min(text_len).max(start);

    // Empty selection
    if start == end {
        let mut res = String::with_capacity(text_len + open.len() + close.len());
        res.push_str(&text[..start]);
        res.push_str(open);
        res.push_str(close);
        res.push_str(&text[start..]);
        let cursor = start + open.len();
        return (res, cursor, cursor);
    }

    let selected = &text[start..end];

    // Inside wrapped
    if selected.len() >= open.len() + close.len()
        && selected.starts_with(open)
        && selected.ends_with(close)
    {
        let unwrapped = &selected[open.len()..selected.len() - close.len()];
        let mut res = String::with_capacity(text_len);
        res.push_str(&text[..start]);
        res.push_str(unwrapped);
        res.push_str(&text[end..]);
        return (res, start, start + unwrapped.len());
    }

    // Outside wrapped
    let has_outer_before = start >= open.len() && &text[start - open.len()..start] == open;
    let has_outer_after = end + close.len() <= text_len && &text[end..end + close.len()] == close;
    if has_outer_before && has_outer_after {
        let mut res = String::with_capacity(text_len);
        res.push_str(&text[..start - open.len()]);
        res.push_str(selected);
        res.push_str(&text[end + close.len()..]);
        let new_start = start - open.len();
        return (res, new_start, new_start + selected.len());
    }

    // Default: wrap selection
    let mut res = String::with_capacity(text_len + open.len() + close.len());
    res.push_str(&text[..start]);
    res.push_str(open);
    res.push_str(selected);
    res.push_str(close);
    res.push_str(&text[end..]);
    (res, start + open.len(), start + open.len() + selected.len())
}

/// Determines whether a line is a list item (checklist, bullet, numbered).
pub fn is_list_line(line: &str) -> bool {
    let trimmed = line.trim_start();
    if trimmed.starts_with("- [ ]")
        || trimmed.starts_with("- [x]")
        || trimmed.starts_with("- [X]")
        || trimmed.starts_with("* [ ]")
        || trimmed.starts_with("* [x]")
        || trimmed.starts_with("* [X]")
        || trimmed.starts_with("[ ]")
        || trimmed.starts_with("[x]")
        || trimmed.starts_with("[X]")
    {
        return true;
    }
    if trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ ") {
        return true;
    }
    // Numbered list: digits followed by dot and space
    if let Some((num_part, rest)) = trimmed.split_once('.') {
        if !num_part.is_empty()
            && num_part.chars().all(|c| c.is_ascii_digit())
            && rest.starts_with(' ')
        {
            return true;
        }
    }
    false
}

/// Typography and Caret Nudge configuration.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CaretNudgeConfig {
    pub base_nudge_px: f32,
    pub list_nudge_px: f32,
}

impl Default for CaretNudgeConfig {
    fn default() -> Self {
        Self {
            base_nudge_px: 0.0,
            list_nudge_px: 0.0,
        }
    }
}

impl CaretNudgeConfig {
    pub fn active_nudge(&self, on_list_line: bool) -> f32 {
        if on_list_line {
            self.base_nudge_px + self.list_nudge_px
        } else {
            self.base_nudge_px
        }
    }
}

/// Right-To-Left (RTL) configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RtlConfig {
    pub force_rtl: bool,
}

impl Default for RtlConfig {
    fn default() -> Self {
        Self { force_rtl: false }
    }
}
