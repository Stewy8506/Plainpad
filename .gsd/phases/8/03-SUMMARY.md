# Phase 8 Plan 3: Find & Replace Search Engine & Third-Party Export Suite Summary

## Overview

Plan 8.3 delivered the full-featured Find & Replace Search Engine (`FindAndReplaceManager`) and the Third-Party Export Suite:
1. Multi-criteria search and replace engine with live match counters, in-editor backdrop highlight pills, navigation, single and bulk replacement, and multi-note excerpt query (`SearchResultView`).
2. Core Rust export engine (`crates/plainpad-core/src/export.rs`) supporting Markdown conversion, Obsidian vault routing (`obsidian://new`), Bear app integration (`bear://x-callback-url`), Apple Notes, Notion, Custom URL Scheme templates (`myapp://create?title={TITLE}&text={CONTENT}&date={DATE}`), first-line title extraction, natural language trigger / keyword omission, and pure Rust zero-dependency bulk ZIP archive generator.
3. UI export modal (`#export-modal`), global keyboard shortcuts (`Ctrl+Shift+E`, `Ctrl+F`), and comprehensive settings panel (`#panel-export` in `ui/settings.html` and `ui/src/settings.ts`).

## Deliverables & Key Changes

1. **Find & Replace Search Engine (`ui/src/search.ts`, `ui/src/main.ts`, `ui/index.html`, `ui/styles.css`)**:
   - Implemented `FindAndReplaceManager` handling 4 match criteria: `Contains`, `Starts With`, `Ends With`, and `Exact Match`.
   - Safe regex compilation preventing malformed user pattern crashes, with case-sensitive toggle.
   - Dual search scopes: "Current Note" and "All Notes".
   - Current note search highlights all matches in `#editor-backdrop` with `.search-match` and active match with `.search-match-active` pills without interfering with syntax highlighting.
   - Match counter display (`X of Y matches`), Next (`Enter`) and Previous (`Shift+Enter`) cycling with automatic textarea selection and scroll centering.
   - Single "Replace" and bulk "Replace All" actions with cursor retention and auto-refresh.
   - Multi-note search extracts 40-character context window excerpts around matches and jumps directly to the matching note and line upon selection.
   - Global shortcuts: `Ctrl+F` / `Cmd+F` opens search modal (prefilling selected text); `Esc` dismisses modal, clears highlight pills, and refocuses the editor.

2. **Core Rust Export Engine & ZIP Generator (`crates/plainpad-core/src/export.rs`, `tests/export_tests.rs`)**:
   - Struct `ExportOptions` controlling first-line title extraction, keyword omission (`sum:`, `avg:`, `math:`, `count:`, `timer:`, `::command`), markdown conversion, and rich-text mode.
   - Title & body extraction (`extract_title_and_body`) stripping title lines from exported body when requested.
   - Markdown exporter (`to_markdown_with_options`) converting headings, bullet lists, and GitHub-style checkboxes (`- [ ]`, `- [x]`).
   - Obsidian URI generator (`format_obsidian_uri`) formatting `obsidian://new?vault={VAULT}&name={TITLE}&content={CONTENT}` with robust percent-encoding.
   - Bear URI generator (`format_bear_uri`) formatting `bear://x-callback-url/create?title={TITLE}&text={CONTENT}`.
   - Apple Notes (`format_apple_notes`) and Notion (`format_notion`) formatters.
   - Custom URL template resolver (`format_custom_url`) substituting `{TITLE}`, `{CONTENT}`, `{DATE}` (`YYYY-MM-DD`), and `{DATE_TIME}` with URL-safe percent-encoding.
   - Pure Rust zero-dependency ZIP archive generator (`ZipArchiveWriter`, `export_all_to_zip`) implementing Local File Headers, Central Directory, and standard IEEE 802.3 CRC32 checksums for uncompressed (Stored, method 0) archives. Fully compatible with native unzip tools.
   - Comprehensive test suite in `crates/plainpad-core/tests/export_tests.rs` with 7 integration tests covering all formatters and ZIP generation.

3. **Tauri IPC Commands (`src-tauri/src/main.rs`)**:
   - `export_note_to_target`: exports note to plaintext, markdown, obsidian URI, bear URI, apple notes, notion, or custom URL.
   - `export_all_notes_zip`: bundles all non-void active and slotted notes into a valid `.zip` byte array.
   - `resolve_custom_url`: resolves custom URL scheme tokens.
   - `get_all_notes`: queries all notes across active and slotted slots for multi-note search and bulk operations.

4. **Third-Party Export UI & Settings Tab (`ui/src/export.ts`, `ui/index.html`, `ui/settings.html`, `ui/src/settings.ts`, `ui/styles.css`)**:
   - Floating `#export-modal` summoned via `Ctrl+Shift+E` or bottom-bar `#btn-export`, showing Plain Text, Markdown, Obsidian, Bear, Apple Notes, Notion, Custom URL, and Bulk ZIP options.
   - Disabled provider states visually indicated based on user preferences.
   - Dedicated "Export" tab in settings (`#panel-export` in `ui/settings.html`) configuring all 16 export preferences.
   - "Download All Notes as ZIP" action button wired in both the export modal and settings tab, generating dynamic timestamped downloads (`plainpad-notes-export-YYYY-MM-DD.zip`).

## Verification Results

- `npm run typecheck`: Passed with 0 errors.
- `npm run build:ui`: Cleanly compiled TypeScript and copied UI assets.
- `cargo check -p plainpad`: Cleanly compiled with 0 errors.
- `cargo test -p plainpad-core --test export_tests`: 7/7 tests passed.
- `cargo test -p plainpad-core`: 153/153 tests passed across all 15 test suites.
- Git commit deferred per user instruction.
