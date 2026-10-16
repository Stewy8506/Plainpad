---
phase: 8
plan: 3
wave: 3
gap_closure: false
depends_on:
  - "8.1"
  - "8.2"
files_modified:
  - crates/plainpad-core/src/export.rs
  - crates/plainpad-core/tests/export_tests.rs
  - src-tauri/src/main.rs
  - ui/src/search.ts
  - ui/src/export.ts
  - ui/src/main.ts
  - ui/src/settings.ts
  - ui/index.html
  - ui/settings.html
  - ui/styles.css
autonomous: true
must_haves:
  truths:
    - "FindAndReplaceManager supports 4 match criteria: Contains, Starts With, Ends With, Exact Match"
    - "Search modal provides case-sensitive toggle, regex toggle, and scope selection (current note vs all notes)"
    - "Active note search displays real-time match counters (X of Y), highlights matches in editor, and supports Next/Previous navigation"
    - "Replace and Replace All actions update note text cleanly and refresh editor highlights and match counters"
    - "All-notes search displays matching notes with highlighted excerpt snippets (SearchResultView) and jumps directly to the matching line"
    - "Core export engine in plainpad-core formats notes for Markdown, Obsidian, Bear, Apple Notes, Notion, and Custom URL templates"
    - "Custom URL scheme template substitutes {TITLE}, {CONTENT}, {DATE}, and {DATE_TIME} tokens with URL-safe percent-encoding"
    - "Export options respect exportGeneralUseFirstLineAsTitle and exportGeneralOmitKeyword (stripping math/sum triggers and :: commands)"
    - "Bulk archive bundles all active and slotted notes into a valid .zip archive of text or markdown files"
    - "Export modal (Ctrl+Shift+E) and Settings export tab provide full user controls for all export destinations"
  artifacts:
    - crates/plainpad-core/src/export.rs
    - crates/plainpad-core/tests/export_tests.rs
    - ui/src/search.ts
    - ui/src/export.ts
    - ui/settings.html
  key_links:
    - "ui/src/search.ts integrates with ui/src/main.ts editor for highlight rendering and replacement"
    - "crates/plainpad-core/src/export.rs formats notes and generates bulk zip archives"
    - "src-tauri/src/main.rs exposes export_note_to_target and export_all_notes_zip commands"
    - "ui/src/export.ts dispatches URI schemes via openExternalUrl or triggers file download"
---

# Plan 8.3: Find & Replace Search Engine and Third-Party Export Suite

<objective>
Implement the search engine (`FindAndReplaceManager`) and the complete third-party export suite:
1. Search and replace engine supporting `Contains`, `Starts With`, `Ends With`, and `Exact Match` criteria, regex and case toggles, live match counter, active note highlighting, single and bulk replacement, and multi-note excerpt search (`SearchResultView`).
2. Third-party export suite in `crates/plainpad-core/src/export.rs`: Plain Text, Markdown, Obsidian vault routing (`obsidian://new`), Bear app integration (`bear://x-callback-url`), Apple Notes, Notion, Custom URL Scheme templates (`myapp://create?title={TITLE}&text={CONTENT}&date={DATE}`), and bulk ZIP archive of all notes.
3. UI export modal (`#export-modal`), shortcut (`Ctrl+Shift+E`), and Settings configuration panel (`panel-export`).

Purpose: Instant search across thousands of notes and effortless interoperability with external knowledge vaults (Obsidian, Bear, Notion, Apple Notes, Custom URLs, ZIP backup) are core pillars of Antinote's frictionless workflow.
Output: `ui/src/search.ts`, `ui/src/export.ts`, `crates/plainpad-core/src/export.rs`, `crates/plainpad-core/tests/export_tests.rs`, and export settings in `ui/settings.html`.
</objective>

<context>
Load these files for context:
- .gsd/SPEC.md (§9.2 Search Engine & Match Types; §10 Third-Party Export Suite)
- .gsd/findings.md (§14 Export Integration Suite; §16.1 Search Engine)
- .gsd/REQUIREMENTS.md (REQ-29, REQ-30)
- crates/plainpad-core/src/export.rs
- crates/plainpad-core/src/model.rs
- src-tauri/src/main.rs
- ui/src/main.ts
- ui/src/settings.ts
- ui/index.html
- ui/settings.html
- ui/styles.css
</context>

<tasks>

<task type="auto">
  <name>Find & Replace Search Engine (FindAndReplaceManager) with Match Criteria, Excerpt Highlights, and Replacements</name>
  <files>
    ui/src/search.ts
    ui/src/main.ts
    ui/index.html
    ui/styles.css
  </files>
  <action>
    Create `ui/src/search.ts` implementing `FindAndReplaceManager` and mount the search panel in `ui/index.html`:
    1. Search Modal Structure in `ui/index.html`:
       - Add `#find-replace-container` (floating card summoned via `Ctrl+F` or search icon):
         - Row 1: Search input (`#search-input`), match type select (`#search-match-type`: "Contains", "Starts With", "Ends With", "Exact Match"), Case Sensitive toggle button (`#btn-search-case`), Regex toggle button (`#btn-search-regex`), Scope toggle ("Current Note" vs "All Notes"), Close button (`#btn-search-close`).
         - Row 2: Replace input (`#replace-input`), "Replace" button (`#btn-replace-one`), "Replace All" button (`#btn-replace-all`), match counter label (`#search-match-count`: e.g. "0/0", "3/12"), Next button (`#btn-search-next`, `Enter`), Previous button (`#btn-search-prev`, `Shift+Enter`).
         - Row 3: All-Notes Results Container (`#search-results-list`, visible when Scope is "All Notes").
    2. Search Matching Engine:
       - Implement pattern constructor based on selected criteria:
         - `Contains`: literal substring or unanchored regex.
         - `Starts With`: regex anchored to line/word start `(^|\\b)query`.
         - `Ends With`: regex anchored to line/word end `query($|\\b)`.
         - `Exact Match`: word boundary regex `\\bquery\\b`.
       - Wrap regex creation in safe `try/catch` to display red border / invalid regex indicator without crashing.
       - Respect `caseSensitive` toggle.
    3. Active Note Search & Replace:
       - When searching current note: find all occurrence ranges `[start, end]`.
       - Update match counter: e.g. "1 of 8 matches".
       - Highlight matches in `#editor-backdrop` or overlay using `.search-match` and `.search-match-active` pills.
       - Next Match (`Enter`) / Previous Match (`Shift+Enter`): cycle focused match, scroll editor to match position, set textarea selection.
       - "Replace": replace current focused match range with replacement text, preserve cursor, update subsequent match positions.
       - "Replace All": perform global string/regex replacement across active note text, update note via `handleTextChange`, announce count replaced (e.g. "Replaced 8 occurrences").
    4. All-Notes Multi-Note Search (`SearchResultView`):
       - When Scope is "All Notes": query all notes from store (`list_notes`).
       - For each note with matches, extract excerpt: 40 characters before match, highlighted match, 40 characters after.
       - Render list of results showing note title, match count, and excerpt snippets.
       - Clicking a result switches active note to target note, switches scope to current note, and focuses first match.
    5. Keyboard Shortcuts:
       - `Ctrl+F` (or `Cmd+F`): summon search bar and focus `#search-input` (prefill with selected text if selection exists).
       - `Esc`: dismiss search modal, clear in-editor highlights, and return focus to editor.

    AVOID: uncaught regex syntax exceptions crashing search; USE: safe validation before execution.
    AVOID: DOM-based find-and-replace corrupting editor caret position; USE: string index slicing with `textarea.setSelectionRange`.
  </action>
  <verify>
    npm run build:ui
  </verify>
  <done>
    Find & Replace modal opens via `Ctrl+F`, searches with all 4 match criteria, regex, and case sensitivity, highlights matches with live counters, cycles matches with Enter/Shift+Enter, executes single and bulk replacements, and searches across all notes with excerpt previews.
  </done>
</task>

<task type="auto">
  <name>Core Export Engine in Rust: Third-Party Formats, Custom URL Templates, Keyword Omission, and Bulk ZIP Archive</name>
  <files>
    crates/plainpad-core/src/export.rs
    crates/plainpad-core/tests/export_tests.rs
    src-tauri/src/main.rs
  </files>
  <action>
    Enhance `crates/plainpad-core/src/export.rs` to implement third-party export targets and formatting rules:
    1. Export Options & Processing:
       - Struct `ExportOptions`:
         ```rust
         #[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
         pub struct ExportOptions {
             pub use_first_line_as_title: bool,
             pub omit_keywords: bool,
             pub convert_to_markdown: bool,
             pub convert_to_rich_text: bool,
         }
         ```
       - Keyword Omission (`omit_keywords` / `exportGeneralOmitKeyword`):
         - Strips natural language triggers from exported lines (`sum:`, `sum `, `avg:`, `avg `, `math:`, `math `, `list:`, `count:`, `code:`, `paste:`, `timer:`, `check:`, `todo:`, and `::commandName(...)`).
       - First Line as Title (`use_first_line_as_title` / `exportGeneralUseFirstLineAsTitle`):
         - Extracts first non-empty line as note title, stripping it from the exported body.
    2. Third-Party Format Builders:
       - Markdown (`to_markdown`): formats title heading, GitHub task checkboxes (`- [ ]`, `- [x]`), bullet lists.
       - Obsidian URI (`format_obsidian_uri`):
         - Template: `obsidian://new?vault={VAULT}&name={TITLE}&content={CONTENT}`.
         - Percent-encodes vault name, note title, and content.
       - Bear URI (`format_bear_uri`):
         - Template: `bear://x-callback-url/create?title={TITLE}&text={CONTENT}`.
         - Percent-encodes title and markdown content.
       - Apple Notes (`format_apple_notes`):
         - Generates rich HTML/text or markdown suitable for system pasteboard / AppleScript transfer.
       - Notion (`format_notion`):
         - Formats page content with database title and markdown blocks.
       - Custom URL Scheme Template (`format_custom_url`):
         - Accepts user template: `myapp://create?title={TITLE}&text={CONTENT}&date={DATE}`.
         - Replaces tokens:
           - `{TITLE}`: percent-encoded note title.
           - `{CONTENT}`: percent-encoded note body.
           - `{DATE}`: ISO date string `YYYY-MM-DD`.
           - `{DATE_TIME}`: ISO date-time string.
    3. Bulk ZIP Archive Generator (`export_all_to_zip`):
       - Function: `export_all_to_zip(notes: &[Note], options: &ExportOptions) -> Result<Vec<u8>, String>`.
       - Generates a valid `.zip` binary archive containing text or markdown files for all given notes.
       - Generates unique filesystem-safe filenames with deduplication (e.g. `meeting.txt`, `meeting-1.txt`).
       - Uses pure Rust standard ZIP file structure (Local File Headers + Central Directory + End of Central Directory) or lightweight zip writer.
    4. Tauri IPC Commands in `src-tauri/src/main.rs`:
       - `export_note_to_target(id: String, target: String, options: ExportOptions)`: resolves note and formats output string or URI.
       - `export_all_notes_zip(options: ExportOptions) -> Result<Vec<u8>, String>`: loads all non-void notes and produces bulk zip bytes.
       - `resolve_custom_url(text: String, template: String) -> String`.
    5. Unit Tests:
       - Create `crates/plainpad-core/tests/export_tests.rs`:
         - Test Markdown checkbox and heading conversion.
         - Test Obsidian URI generation with special characters.
         - Test Bear URI generation.
         - Test Custom URL template token replacement (`{TITLE}`, `{CONTENT}`, `{DATE}`).
         - Test `omit_keywords` stripping math triggers.
         - Test `export_all_to_zip` produces a valid ZIP archive containing multiple entries.

    AVOID: unescaped characters in URI schemes breaking browser/OS dispatch; USE: robust percent-encoding for all URL components.
    AVOID: external heavy zip dependencies; USE: clean, zero-dependency zip archive generation in pure Rust.
  </action>
  <verify>
    cargo test -p plainpad-core --test export_tests
  </verify>
  <done>
    All unit tests pass verifying Plaintext, Markdown, Obsidian URI, Bear URI, Custom URL template token replacement, keyword omission, and bulk ZIP archive generation.
  </done>
</task>

<task type="auto">
  <name>Third-Party Export UI Dispatchers, Modal Menu, and Export Settings Configuration Panel</name>
  <files>
    ui/src/export.ts
    ui/src/main.ts
    ui/src/settings.ts
    ui/settings.html
    ui/index.html
    ui/styles.css
  </files>
  <action>
    Create `ui/src/export.ts` and integrate export UI actions and settings:
    1. Export Menu / Modal in `ui/index.html` (`#export-modal`):
       - Summoned via `Ctrl+Shift+E` or bottom-bar `#btn-export`.
       - Clean modal card displaying available export destinations:
         - "Plain Text (.txt)"
         - "Markdown (.md)"
         - "Obsidian" (with vault label if configured)
         - "Bear"
         - "Apple Notes"
         - "Notion"
         - "Custom URL Scheme" (with template name)
         - Separator
         - "Export All Notes as ZIP (.zip)"
       - Esc closes export modal.
    2. Export Dispatch Logic in `ui/src/export.ts`:
       - For URI-based targets (Obsidian, Bear, Custom URL):
         - Invoke backend to format target URI with active note title, content, and date.
         - Dispatch URI using `openExternalUrl(url)` (which invokes Tauri shell open).
         - Show temporary toast notification: "Dispatched to Obsidian / Bear".
       - For File-based targets (Plain Text, Markdown):
         - Get exported content and safe filename from backend.
         - Trigger native file save dialog or browser download blob (`a.download = filename; a.href = blobUrl; a.click()`).
       - For Bulk ZIP:
         - Invoke `export_all_notes_zip`.
         - Trigger download blob with filename `plainpad-notes-export-YYYY-MM-DD.zip`.
         - Show toast: "Exported %d notes to ZIP archive".
    3. Export Settings Tab in `ui/settings.html` (`panel-export`):
       - Add "Export" navigation tab in settings sidebar with export icon.
       - Add settings panel `#panel-export`:
         - General Export Options:
           - Checkbox `#setting-export-first-line-title`: "Use first non-empty line as document title (`exportGeneralUseFirstLineAsTitle`)".
           - Checkbox `#setting-export-omit-keywords`: "Omit trigger keywords like `math`, `sum`, `avg`, `::command` from export (`exportGeneralOmitKeyword`)".
         - Provider Configurations:
           - Plain Text toggle (`exportPlainTextEnabled`).
           - Markdown toggle (`exportMarkdownEnabled`).
           - Obsidian section: toggle (`exportObsidianEnabled`), Vault name input (`exportObsidianVault`, default `"Notes"`), Convert to Markdown toggle (`exportObsidianConvertToMarkdown`).
           - Bear section: toggle (`exportBearEnabled`), Convert to Markdown toggle (`exportBearConvertToMarkdown`).
           - Apple Notes section: toggle (`exportAppleNotesEnabled`), Convert to Rich Text toggle (`exportAppleNotesConvertToRichText`).
           - Notion section: toggle (`exportNotionEnabled`), Save in Database toggle (`exportNotionSaveInDatabase`), Database name input (`exportNotionDatabaseName`).
           - Custom URL Scheme section: toggle (`exportCustomEnabled`), Custom Name input (`exportCustomName`, default `"My App"`), URL Template input (`exportCustomURLTemplate`, placeholder `"myapp://create?title={TITLE}&text={CONTENT}&date={DATE}"`).
         - Bulk Backup section:
           - Description: "Download all active and slotted notes in an uncompressed .zip archive of text files."
           - Action button: "Download All Notes as ZIP" (`#btn-settings-export-zip`).
    4. Settings Storage Wiring in `ui/src/settings.ts`:
       - Load and persist all export preferences in SQLite `app_state`.
       - Wire "Download All Notes as ZIP" button in settings panel.

    AVOID: forcing external app exports without checking enabled status; USE: grey out or omit disabled providers in export menu based on settings.
    AVOID: losing user-configured vault names or custom templates; USE: persistent SQLite storage via `set_setting` / `get_setting`.
  </action>
  <verify>
    npm run build:ui && cargo check -p plainpad
  </verify>
  <done>
    Export modal opens via `Ctrl+Shift+E` or button, all export targets dispatch cleanly (file download or URI scheme), bulk ZIP downloads all notes, and Settings export tab persists all configuration fields.
  </done>
</task>

</tasks>

<verification>
After all tasks complete, verify:
- [ ] `cargo test -p plainpad-core --test export_tests` passes all tests
- [ ] `npm run typecheck` passes with zero errors
- [ ] `npm run build:ui` bundles UI assets with `search.ts` and `export.ts`
- [ ] `cargo check -p plainpad` compiles without errors
- [ ] `FindAndReplaceManager` handles Contains, Starts With, Ends With, and Exact Match queries
- [ ] In-editor highlights, navigation, single replace, and replace all function as specified
- [ ] Multi-note search returns excerpt previews with direct jump navigation
- [ ] Third-party formatters construct valid Obsidian, Bear, and Custom URL templates with percent-encoding
- [ ] Keyword omission strips triggers and double-colon commands from export text
- [ ] Bulk ZIP archive bundles all active and slotted notes into a valid `.zip` file
- [ ] Settings Export tab provides full controls for all 8 export configurations
</verification>

<success_criteria>
- [ ] All tasks verified passing
- [ ] Must-haves confirmed
- [ ] No regressions in test suites
</success_criteria>
