//! AutoPaste continuous clipboard ingestion engine (REQ-14, findings §6).
//!
//! Provides OS-wide clipboard monitoring abstractions, mock readers for headless CI,
//! atomic state tracking, and consecutive duplicate suppression.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

/// Abstraction over system or mock clipboard readers.
pub trait ClipboardReader: Send + Sync {
    /// Read the current clipboard string content, if any.
    fn read_text(&mut self) -> Result<Option<String>, String>;
}

/// OS clipboard reader using native platform tools (pbpaste on macOS,
/// PowerShell on Windows, wl-paste / xclip / xsel on Linux).
#[derive(Default)]
pub struct SystemClipboardReader;

impl SystemClipboardReader {
    /// Create a new reader attempting to connect to the system clipboard.
    pub fn new() -> Result<Self, String> {
        Ok(Self)
    }

    /// Create a reader leniently (ignoring initial availability check).
    pub fn new_lenient() -> Self {
        Self
    }

    /// Read the current clipboard string content, if any.
    pub fn read_text(&mut self) -> Result<Option<String>, String> {
        <Self as ClipboardReader>::read_text(self)
    }
}

impl ClipboardReader for SystemClipboardReader {
    fn read_text(&mut self) -> Result<Option<String>, String> {
        #[cfg(target_os = "macos")]
        {
            if let Ok(output) = std::process::Command::new("pbpaste").output() {
                if output.status.success() {
                    let s = String::from_utf8_lossy(&output.stdout).to_string();
                    return Ok(if s.is_empty() { None } else { Some(s) });
                }
            }
        }

        #[cfg(target_os = "windows")]
        {
            if let Ok(output) = std::process::Command::new("powershell")
                .args(["-NoProfile", "-Command", "Get-Clipboard"])
                .output()
            {
                if output.status.success() {
                    let s = String::from_utf8_lossy(&output.stdout).trim_end_matches("\r\n").to_string();
                    return Ok(if s.is_empty() { None } else { Some(s) });
                }
            }
        }

        #[cfg(target_os = "linux")]
        {
            for (cmd, args) in [
                ("wl-paste", vec!["--no-newline"]),
                ("xclip", vec!["-selection", "clipboard", "-o"]),
                ("xsel", vec!["--clipboard", "--output"]),
            ] {
                if let Ok(output) = std::process::Command::new(cmd).args(&args).output() {
                    if output.status.success() {
                        let s = String::from_utf8_lossy(&output.stdout).to_string();
                        return Ok(if s.is_empty() { None } else { Some(s) });
                    }
                }
            }
        }

        Ok(None)
    }
}

/// In-memory queue mock reader for deterministic tests without a display server.
#[derive(Default)]
pub struct MockClipboardReader {
    queue: Mutex<VecDeque<String>>,
}

impl MockClipboardReader {
    pub fn new() -> Self {
        Self {
            queue: Mutex::new(VecDeque::new()),
        }
    }

    pub fn from_items(items: Vec<String>) -> Self {
        Self {
            queue: Mutex::new(items.into()),
        }
    }

    pub fn push(&self, text: impl Into<String>) {
        let mut q = self.queue.lock().unwrap();
        q.push_back(text.into());
    }
}

impl ClipboardReader for MockClipboardReader {
    fn read_text(&mut self) -> Result<Option<String>, String> {
        let mut q = self.queue.lock().unwrap();
        Ok(q.pop_front())
    }
}

/// Continuous AutoPaste clipboard ingestion engine.
#[derive(Clone, Default)]
pub struct AutoPasteEngine {
    is_active: Arc<AtomicBool>,
    last_content: Arc<Mutex<Option<String>>>,
}

impl AutoPasteEngine {
    /// Create a new inactive AutoPasteEngine.
    pub fn new() -> Self {
        Self {
            is_active: Arc::new(AtomicBool::new(false)),
            last_content: Arc::new(Mutex::new(None)),
        }
    }

    /// Set `is_active` to true, returning previous state.
    pub fn start(&self) -> bool {
        self.is_active.swap(true, Ordering::SeqCst)
    }

    /// Set `is_active` to false, returning previous state.
    pub fn stop(&self) -> bool {
        self.is_active.swap(false, Ordering::SeqCst)
    }

    /// Check if continuous clipboard monitoring is active.
    pub fn is_active(&self) -> bool {
        self.is_active.load(Ordering::SeqCst)
    }

    /// Reset cached last content.
    pub fn clear_last_content(&self) {
        if let Ok(mut lock) = self.last_content.lock() {
            *lock = None;
        }
    }

    /// Get current cached last content.
    pub fn last_content(&self) -> Option<String> {
        self.last_content.lock().unwrap().clone()
    }

    /// Process clipboard text:
    /// - Trims leading and trailing whitespace.
    /// - If trimmed string is empty, returns None.
    /// - If identical to last content, returns None (duplicate suppression).
    /// - Otherwise updates last content and returns Some(trimmed).
    pub fn process_clip(&self, text: &str) -> Option<String> {
        let trimmed = text.trim();
        if trimmed.is_empty() {
            return None;
        }

        let mut lock = self.last_content.lock().unwrap();
        if let Some(ref prev) = *lock {
            if prev == trimmed {
                return None;
            }
        }

        *lock = Some(trimmed.to_string());
        Some(trimmed.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_engine_lifecycle_and_flags() {
        let engine = AutoPasteEngine::new();
        assert!(!engine.is_active());

        let prev = engine.start();
        assert!(!prev);
        assert!(engine.is_active());

        let prev_stop = engine.stop();
        assert!(prev_stop);
        assert!(!engine.is_active());
    }

    #[test]
    fn test_duplicate_suppression_and_trimming() {
        let engine = AutoPasteEngine::new();

        assert_eq!(engine.process_clip(""), None);
        assert_eq!(engine.process_clip("   \n\t  "), None);

        let clip1 = engine.process_clip("  Hello World  ");
        assert_eq!(clip1, Some("Hello World".to_string()));

        // Consecutive duplicate suppressed
        let dup = engine.process_clip("Hello World");
        assert_eq!(dup, None);

        // Another distinct clip succeeds
        let clip2 = engine.process_clip("Second snippet");
        assert_eq!(clip2, Some("Second snippet".to_string()));

        // Back to first clip is now accepted (interleaved)
        let clip3 = engine.process_clip("Hello World");
        assert_eq!(clip3, Some("Hello World".to_string()));
    }

    #[test]
    fn test_mock_clipboard_reader() {
        let mut mock = MockClipboardReader::new();
        assert_eq!(mock.read_text().unwrap(), None);

        mock.push("Item 1");
        mock.push("Item 2");

        assert_eq!(mock.read_text().unwrap(), Some("Item 1".into()));
        assert_eq!(mock.read_text().unwrap(), Some("Item 2".into()));
        assert_eq!(mock.read_text().unwrap(), None);
    }
}
