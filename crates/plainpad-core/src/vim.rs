//! Vim modal editing engine and boundary calculation utilities for Plainpad.
//! Implements NORMAL, INSERT, VISUAL, and VISUAL LINE modes with word boundary,
//! motion navigation, delete/yank registers, and paste semantics.

use serde::{Deserialize, Serialize};

/// Vim modal state enum.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VimMode {
    Normal,
    Insert,
    Visual,
    VisualLine,
}

impl Default for VimMode {
    fn default() -> Self {
        Self::Normal
    }
}

/// Vim editor state tracking active mode, cursor position, selection anchor, and registers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct VimState {
    pub mode: VimMode,
    pub cursor_pos: usize,
    pub anchor_pos: usize,
    pub yank_register: String,
    pub is_line_yank: bool,
    pub command_buffer: String,
}

impl Default for VimState {
    fn default() -> Self {
        Self::new()
    }
}

impl VimState {
    pub fn new() -> Self {
        Self {
            mode: VimMode::Normal,
            cursor_pos: 0,
            anchor_pos: 0,
            yank_register: String::new(),
            is_line_yank: false,
            command_buffer: String::new(),
        }
    }

    pub fn set_mode(&mut self, mode: VimMode, text: &str) {
        let prev = self.mode;
        self.mode = mode;
        self.command_buffer.clear();

        match mode {
            VimMode::Normal => {
                if prev == VimMode::Insert {
                    let line_start = get_line_start(text, self.cursor_pos);
                    if self.cursor_pos > line_start {
                        self.cursor_pos -= 1;
                    }
                }
                self.cursor_pos = clamp_cursor_normal(text, self.cursor_pos);
            }
            VimMode::Visual | VimMode::VisualLine => {
                self.anchor_pos = self.cursor_pos;
            }
            VimMode::Insert => {}
        }
    }

    pub fn execute_motion(&mut self, text: &str, motion: &str, count: usize) {
        let mut target = self.cursor_pos;
        for _ in 0..count.max(1) {
            match motion {
                "h" => {
                    let line_start = get_line_start(text, target);
                    if target > line_start {
                        target -= 1;
                    }
                }
                "l" => {
                    let line_start = get_line_start(text, target);
                    let line_end = get_line_end(text, target);
                    let max_target = if self.mode == VimMode::Normal {
                        if line_end > line_start {
                            line_end - 1
                        } else {
                            line_start
                        }
                    } else {
                        line_end
                    };
                    if target < max_target {
                        target += 1;
                    }
                }
                "j" => {
                    target = get_line_down(text, target, 1, self.mode == VimMode::Normal);
                }
                "k" => {
                    target = get_line_up(text, target, 1, self.mode == VimMode::Normal);
                }
                "w" => {
                    target = get_next_word_start(text, target);
                }
                "b" => {
                    target = get_prev_word_start(text, target);
                }
                "e" => {
                    target = get_word_end(text, target);
                }
                "0" => {
                    target = get_line_start(text, target);
                }
                "^" => {
                    target = get_first_non_whitespace(text, target);
                }
                "$" => {
                    let line_start = get_line_start(text, target);
                    let line_end = get_line_end(text, target);
                    target = if self.mode == VimMode::Normal {
                        if line_end > line_start {
                            line_end - 1
                        } else {
                            line_start
                        }
                    } else {
                        line_end
                    };
                }
                "gg" => {
                    target = 0;
                }
                "G" => {
                    target = get_line_start(text, text.len());
                }
                _ => {}
            }
        }
        self.cursor_pos = if self.mode == VimMode::Normal {
            clamp_cursor_normal(text, target)
        } else {
            target.min(text.len())
        };
    }

    pub fn execute_delete_char(&mut self, text: &mut String, count: usize) {
        if text.is_empty() || self.cursor_pos >= text.len() {
            return;
        }
        let line_end = get_line_end(text, self.cursor_pos);
        let chars_to_delete = count.max(1).min(line_end.saturating_sub(self.cursor_pos).max(1));
        let deleted: String = text[self.cursor_pos..self.cursor_pos + chars_to_delete].to_string();

        self.yank_register = deleted;
        self.is_line_yank = false;

        text.drain(self.cursor_pos..self.cursor_pos + chars_to_delete);
        self.cursor_pos = clamp_cursor_normal(text, self.cursor_pos);
    }

    pub fn execute_replace_char(&mut self, text: &mut String, c: char) {
        if text.is_empty() || self.cursor_pos >= text.len() {
            return;
        }
        let next_char_end = self.cursor_pos + 1;
        text.replace_range(self.cursor_pos..next_char_end, &c.to_string());
    }

    pub fn execute_delete_lines(&mut self, text: &mut String, count: usize) {
        if text.is_empty() {
            return;
        }
        let mut start = get_line_start(text, self.cursor_pos);
        let mut end = start;

        for _ in 0..count.max(1) {
            match text[end..].find('\n') {
                Some(idx) => {
                    end += idx + 1;
                }
                None => {
                    end = text.len();
                    break;
                }
            }
        }

        let mut deleted = text[start..end].to_string();
        if !deleted.ends_with('\n') && start > 0 && text.as_bytes()[start - 1] == b'\n' {
            start -= 1;
            deleted.push('\n');
        }

        self.yank_register = deleted;
        self.is_line_yank = true;

        text.drain(start..end);
        self.cursor_pos = clamp_cursor_normal(text, start);
    }

    pub fn execute_yank_lines(&mut self, text: &str, count: usize) {
        if text.is_empty() {
            return;
        }
        let start = get_line_start(text, self.cursor_pos);
        let mut end = start;

        for _ in 0..count.max(1) {
            match text[end..].find('\n') {
                Some(idx) => {
                    end += idx + 1;
                }
                None => {
                    end = text.len();
                    break;
                }
            }
        }

        let mut yanked = text[start..end].to_string();
        if !yanked.ends_with('\n') {
            yanked.push('\n');
        }
        self.yank_register = yanked;
        self.is_line_yank = true;
    }

    pub fn execute_delete_to_end_of_line(&mut self, text: &mut String) {
        let line_end = get_line_end(text, self.cursor_pos);
        if self.cursor_pos >= line_end {
            return;
        }
        let deleted = text[self.cursor_pos..line_end].to_string();
        self.yank_register = deleted;
        self.is_line_yank = false;

        text.drain(self.cursor_pos..line_end);
        self.cursor_pos = clamp_cursor_normal(text, self.cursor_pos);
    }

    pub fn execute_paste(&mut self, text: &mut String, after: bool) {
        if self.yank_register.is_empty() {
            return;
        }

        if self.is_line_yank {
            if after {
                let line_end = get_line_end(text, self.cursor_pos);
                if line_end == text.len() {
                    let prefix = if text.ends_with('\n') || text.is_empty() {
                        ""
                    } else {
                        "\n"
                    };
                    text.push_str(prefix);
                    let pos = text.len();
                    text.push_str(&self.yank_register);
                    self.cursor_pos = pos;
                } else {
                    let insert_pos = line_end + 1;
                    text.insert_str(insert_pos, &self.yank_register);
                    self.cursor_pos = insert_pos;
                }
            } else {
                let line_start = get_line_start(text, self.cursor_pos);
                text.insert_str(line_start, &self.yank_register);
                self.cursor_pos = line_start;
            }
        } else {
            let insert_pos = if after {
                (self.cursor_pos + 1).min(text.len())
            } else {
                self.cursor_pos.min(text.len())
            };
            text.insert_str(insert_pos, &self.yank_register);
            self.cursor_pos = insert_pos + self.yank_register.len().saturating_sub(1);
        }

        self.cursor_pos = clamp_cursor_normal(text, self.cursor_pos);
    }

    pub fn get_visual_selection(&self, text: &str) -> (usize, usize) {
        match self.mode {
            VimMode::Visual => {
                let start = self.anchor_pos.min(self.cursor_pos);
                let end = (self.anchor_pos.max(self.cursor_pos) + 1).min(text.len());
                (start, end)
            }
            VimMode::VisualLine => {
                let min_pos = self.anchor_pos.min(self.cursor_pos);
                let max_pos = self.anchor_pos.max(self.cursor_pos);
                let start = get_line_start(text, min_pos);
                let line_end = get_line_end(text, max_pos);
                let end = if line_end < text.len() {
                    line_end + 1
                } else {
                    line_end
                };
                (start, end)
            }
            _ => (self.cursor_pos, self.cursor_pos),
        }
    }
}

// ---------- Line and Word Boundary Helpers ----------

pub fn get_line_start(text: &str, pos: usize) -> usize {
    let clamped = pos.min(text.len());
    match text[..clamped].rfind('\n') {
        Some(idx) => idx + 1,
        None => 0,
    }
}

pub fn get_line_end(text: &str, pos: usize) -> usize {
    let clamped = pos.min(text.len());
    match text[clamped..].find('\n') {
        Some(idx) => clamped + idx,
        None => text.len(),
    }
}

pub fn get_line_and_col(text: &str, pos: usize) -> (usize, usize) {
    let clamped = pos.min(text.len());
    let before = &text[..clamped];
    let line = before.matches('\n').count() + 1;
    let line_start = get_line_start(text, clamped);
    let col = clamped - line_start + 1;
    (line, col)
}

pub fn get_first_non_whitespace(text: &str, pos: usize) -> usize {
    let start = get_line_start(text, pos);
    let end = get_line_end(text, pos);
    for (i, c) in text[start..end].char_indices() {
        if !c.is_whitespace() {
            return start + i;
        }
    }
    start
}

pub fn clamp_cursor_normal(text: &str, pos: usize) -> usize {
    if text.is_empty() {
        return 0;
    }
    let start = get_line_start(text, pos);
    let end = get_line_end(text, pos);
    if end > start {
        pos.clamp(start, end - 1)
    } else {
        start
    }
}

pub fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

pub fn get_next_word_start(text: &str, pos: usize) -> usize {
    let mut chars = text[pos..].char_indices().peekable();
    let first = match chars.peek() {
        Some(&(_, c)) => c,
        None => return text.len(),
    };

    let on_word = is_word_char(first);
    let on_punct = !on_word && !first.is_whitespace();

    if on_word {
        while let Some(&(_, c)) = chars.peek() {
            if is_word_char(c) {
                chars.next();
            } else {
                break;
            }
        }
    } else if on_punct {
        while let Some(&(_, c)) = chars.peek() {
            if !is_word_char(c) && !c.is_whitespace() {
                chars.next();
            } else {
                break;
            }
        }
    }

    // Skip whitespace
    while let Some(&(_, c)) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
        } else {
            break;
        }
    }

    match chars.peek() {
        Some(&(idx, _)) => pos + idx,
        None => text.len(),
    }
}

pub fn get_prev_word_start(text: &str, pos: usize) -> usize {
    if pos == 0 || text.is_empty() {
        return 0;
    }
    let clamped = pos.min(text.len());
    let bytes = text[..clamped].as_bytes();
    let mut i = clamped - 1;

    // Skip whitespace backwards
    while i > 0 && (bytes[i] as char).is_whitespace() {
        i -= 1;
    }

    let c = bytes[i] as char;
    if is_word_char(c) {
        while i > 0 && is_word_char(bytes[i - 1] as char) {
            i -= 1;
        }
    } else if !c.is_whitespace() {
        while i > 0 && !is_word_char(bytes[i - 1] as char) && !(bytes[i - 1] as char).is_whitespace() {
            i -= 1;
        }
    }

    i
}

pub fn get_word_end(text: &str, pos: usize) -> usize {
    if text.is_empty() || pos >= text.len() {
        return text.len().saturating_sub(1);
    }
    let mut chars = text[pos..].char_indices().peekable();
    // Skip at least 1 character forward
    chars.next();

    // Skip whitespace forwards
    while let Some(&(_, c)) = chars.peek() {
        if c.is_whitespace() {
            chars.next();
        } else {
            break;
        }
    }

    let first = match chars.peek() {
        Some(&(_, c)) => c,
        None => return text.len().saturating_sub(1),
    };

    let on_word = is_word_char(first);
    let mut last_idx = 0;

    if on_word {
        while let Some(&(idx, c)) = chars.peek() {
            if is_word_char(c) {
                last_idx = idx;
                chars.next();
            } else {
                break;
            }
        }
    } else {
        while let Some(&(idx, c)) = chars.peek() {
            if !is_word_char(c) && !c.is_whitespace() {
                last_idx = idx;
                chars.next();
            } else {
                break;
            }
        }
    }

    pos + last_idx
}

pub fn get_line_down(text: &str, pos: usize, count: usize, clamp_normal: bool) -> usize {
    let lines: Vec<&str> = text.split('\n').collect();
    let (cur_line, cur_col) = get_line_and_col(text, pos);
    let target_line_idx = (cur_line - 1 + count).min(lines.len().saturating_sub(1));

    let mut char_offset = 0;
    for i in 0..target_line_idx {
        char_offset += lines[i].len() + 1;
    }

    let target_line_len = lines[target_line_idx].len();
    let max_col = if clamp_normal {
        target_line_len.saturating_sub(1)
    } else {
        target_line_len
    };
    let target_col = (cur_col - 1).min(max_col);
    char_offset + target_col
}

pub fn get_line_up(text: &str, pos: usize, count: usize, clamp_normal: bool) -> usize {
    let lines: Vec<&str> = text.split('\n').collect();
    let (cur_line, cur_col) = get_line_and_col(text, pos);
    let target_line_idx = cur_line.saturating_sub(1 + count);

    let mut char_offset = 0;
    for i in 0..target_line_idx {
        char_offset += lines[i].len() + 1;
    }

    let target_line_len = lines[target_line_idx].len();
    let max_col = if clamp_normal {
        target_line_len.saturating_sub(1)
    } else {
        target_line_len
    };
    let target_col = (cur_col - 1).min(max_col);
    char_offset + target_col
}
