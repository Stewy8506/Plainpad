---
phase: 4
plan: 1
wave: 1
depends_on: []
files_modified:
  - crates/plainpad-core/src/trigger_config.rs
  - crates/plainpad-core/src/lib.rs
  - crates/plainpad-core/src/intent.rs
  - src-tauri/src/main.rs
  - ui/src/main.ts
  - ui/src/settings.ts
  - ui/settings.html
  - crates/plainpad-core/tests/trigger_evaluator_tests.rs
autonomous: true
gap_closure: false
must_haves:
  truths:
    - "Trigger evaluation accurately processes all 9 categories: sum, avg, math, count, list, check, code, paste, and timer"
    - "Preceding numeric lines followed by sum, total, or add aggregate numbers and currencies accurately at paragraph/list scope"
    - "Preceding numeric lines followed by avg or average compute the arithmetic mean at paragraph/list scope"
    - "Preceding items followed by count compute the total item count accurately"
    - "Master toggle disableAllKeywords disables all trigger keyword processing when set to true"
    - "allowArgumentsAfterKeyword permits inline trailing arguments when true and rejects them when false"
    - "Configurable aliases map custom keyword strings to their target trigger categories"
    - "Structural trigger chips for list, check, and code transform target text blocks into markdown lists, checkboxes, and code blocks upon Tab/Enter"
  artifacts:
    - "crates/plainpad-core/src/trigger_config.rs"
    - "crates/plainpad-core/src/intent.rs"
    - "src-tauri/src/main.rs"
    - "ui/src/main.ts"
    - "ui/src/settings.ts"
    - "ui/settings.html"
    - "crates/plainpad-core/tests/trigger_evaluator_tests.rs"
  key_links:
    - "intent::analyze_with_config evaluates notes using TriggerConfig loaded from store"
    - "src-tauri/src/main.rs exposes get_trigger_config and set_trigger_config IPC commands"
    - "ui/src/settings.ts loads and persists trigger alias preferences through Tauri IPC"
    - "ui/src/main.ts handles interactive chip acceptance for list, check, and code transformations"
---

# Plan 4.1: Sentence-Level Trigger Evaluator (All 9 Categories), Alias Registry, and Master Controls

<objective>
Implement the sentence-level trigger evaluator covering all 9 natural-language trigger categories (`sum`, `avg`, `math`, `count`, `list`, `check`, `code`, `paste`, `timer`) with paragraph/list backward scoping, configurable alias mappings (`sumTriggers`, `avgTriggers`, etc.), master toggle `disableAllKeywords`, inline arguments toggle `allowArgumentsAfterKeyword`, and interactive text block transformations (`list`, `check`, `code`) in the editor.

Purpose: Antinote allows typing natural trigger words anywhere in a note to calculate running totals, averages, item counts, math expressions, and markdown structural conversions with configurable aliases.
Output: `trigger_config.rs` registry, enhanced `intent.rs` evaluator, Tauri IPC commands, UI settings & transformation handlers, and integration test suite `trigger_evaluator_tests.rs`.
</objective>

<context>
Load these files for context:
- .gsd/SPEC.md (§5 Natural-Language Triggers & Keyword Taxonomy, §11 Preferences table)
- .gsd/findings.md (§10 Natural-Language Trigger Engine & Taxonomy)
- .gsd/REQUIREMENTS.md (REQ-12, REQ-13)
- crates/plainpad-core/src/intent.rs
- crates/plainpad-core/src/lib.rs
- src-tauri/src/main.rs
- ui/src/main.ts
- ui/src/settings.ts
- ui/settings.html
</context>

<tasks>

<task type="auto">
  <name>Create trigger configuration registry, alias storage, and Tauri IPC commands</name>
  <files>
    crates/plainpad-core/src/trigger_config.rs
    crates/plainpad-core/src/lib.rs
    src-tauri/src/main.rs
  </files>
  <action>
    1. Create `crates/plainpad-core/src/trigger_config.rs`:
       - Define `#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)] pub enum TriggerCategory { Sum, Avg, Math, Count, List, Check, Code, Paste, Timer }`.
       - Define `#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)] pub struct TriggerConfig`:
         - `pub disable_all_keywords: bool` (default `false`)
         - `pub allow_arguments_after_keyword: bool` (default `true`)
         - `pub sum_triggers: Vec<String>` (defaults: `vec!["sum", "total", "add"]`)
         - `pub avg_triggers: Vec<String>` (defaults: `vec!["avg", "average"]`)
         - `pub math_triggers: Vec<String>` (defaults: `vec!["math", "convert", "model"]`)
         - `pub count_triggers: Vec<String>` (defaults: `vec!["count"]`)
         - `pub list_triggers: Vec<String>` (defaults: `vec!["list"]`)
         - `pub check_triggers: Vec<String>` (defaults: `vec!["check", "todo"]`)
         - `pub code_triggers: Vec<String>` (defaults: `vec!["code"]`)
         - `pub paste_triggers: Vec<String>` (defaults: `vec!["paste"]`)
         - `pub timer_triggers: Vec<String>` (defaults: `vec!["timer"]`)
         - `pub custom_keyword_label: Option<String>`
         - `pub custom_keyword_description: Option<String>`
       - Implement `Default` for `TriggerConfig`.
       - Implement `match_trigger<'a>(&self, line: &'a str) -> Option<(TriggerCategory, &'a str, &'a str)>`:
         - Returns `(category, matched_keyword, remaining_args)`.
         - When `self.disable_all_keywords` is true, immediately returns `None`.
         - Checks word boundaries: if `self.allow_arguments_after_keyword` is false, only matches if the trimmed line equals an alias with no extra text.
         - If `allow_arguments_after_keyword` is true, matches if line starts with alias followed by space, colon, or end-of-line.
       - Implement `from_settings(map: &serde_json::Map<String, serde_json::Value>) -> Self` and `to_settings(&self) -> serde_json::Map<String, serde_json::Value>` mapping to SPEC §5 storage keys:
         - `sumTriggers`, `sumMainTrigger`, `avgTriggers`, `avgMainTrigger`, `mathTriggers`, `mathMainTrigger`, `countTriggers`, `countMainTrigger`, `listTriggers`, `listMainTrigger`, `checkTriggers`, `checkMainTrigger`, `codeTriggers`, `codeMainTrigger`, `pasteTriggers`, `pasteMainTrigger`, `timerTriggers`, `timerMainTrigger`, `disableAllKeywords`, `allowArgumentsAfterKeyword`, `customKeywordLabel`, `customKeywordDescription`.
    2. In `crates/plainpad-core/src/lib.rs`: export `pub mod trigger_config;` and re-export `TriggerConfig`, `TriggerCategory`.
    3. In `src-tauri/src/main.rs`:
       - Add Tauri command `get_trigger_config(state: tauri::State<'_, AppStateLock>) -> Result<TriggerConfig, String>` reading persisted settings from store.
       - Add Tauri command `set_trigger_config(state: tauri::State<'_, AppStateLock>, config: TriggerConfig) -> Result<(), String>` persisting settings into store.
       - Update `analyze_note(text: String, plain: bool, state: tauri::State<'_, AppStateLock>) -> intent::Analysis` to load current `TriggerConfig` from store and call `intent::analyze_with_config(&text, plain, &config)`.
       - Register `get_trigger_config` and `set_trigger_config` in `generate_handler![...]`.

    AVOID: Hardcoding keyword string checks in scattered match statements.
    USE: Centralized `TriggerConfig::match_trigger` with alias lookup and store persistence.
  </action>
  <verify>
    cargo test -p plainpad-core --lib trigger_config::
  </verify>
  <done>
    `TriggerConfig` parses and matches all 9 trigger categories, respects `disable_all_keywords` and `allow_arguments_after_keyword`, and Tauri IPC commands get/set trigger settings cleanly.
  </done>
</task>

<task type="auto">
  <name>Implement sentence-level trigger evaluator for all 9 categories with paragraph scoping and integration tests</name>
  <files>
    crates/plainpad-core/src/intent.rs
    crates/plainpad-core/tests/trigger_evaluator_tests.rs
  </files>
  <action>
    1. In `crates/plainpad-core/src/intent.rs`:
       - Import `crate::trigger_config::{TriggerCategory, TriggerConfig}`.
       - Implement `pub fn analyze_with_config(text: &str, plain: bool, config: &TriggerConfig) -> Analysis`.
       - Retain `pub fn analyze(text: &str, plain: bool) -> Analysis` as `analyze_with_config(text, plain, &TriggerConfig::default())` for full backward compatibility.
       - If `plain || config.disable_all_keywords`: short-circuit trigger evaluation and emit no trigger annotations.
       - Implement `extract_preceding_block(lines: &[&str], trigger_idx: usize) -> Vec<(usize, &str)>`:
         - Scans upward from `trigger_idx - 1` until reaching an empty line, a markdown heading (`#`), or another trigger line.
         - Returns all non-empty lines in the preceding paragraph/list slice with their line indices.
       - For each line `i`:
         - Call `config.match_trigger(line)`. If a trigger matches:
           - **`TriggerCategory::Sum`**:
             - Collect numeric and currency amounts from preceding block lines (e.g. `$25.50 lunch`, `100 books`, `40`, `-15 refund`, and evaluated math results on preceding lines).
             - Calculate sum. If items share a currency symbol (`$`, `€`, `£`, `₹`, `¥`), format result with the currency symbol.
             - Emit `Annotation { line: i, kind: "math", confidence: 0.95, value: formatted_sum, chip: String::new(), why: "sum of numbers and currencies in preceding paragraph or list".into() }`.
           - **`TriggerCategory::Avg`**:
             - Calculate arithmetic mean (`sum / count`) across preceding block numbers.
             - Emit `Annotation { line: i, kind: "math", confidence: 0.95, value: formatted_avg, chip: String::new(), why: "arithmetic mean of values in preceding paragraph or list".into() }`.
           - **`TriggerCategory::Count`**:
             - If preceding block has multiple lines, count the lines. If a single line has comma-separated items, count the items.
             - Emit `Annotation { line: i, kind: "stats", confidence: 0.95, value: format!("{count} items"), chip: String::new(), why: "count of items in preceding list".into() }`.
           - **`TriggerCategory::Math`**:
             - If `remaining_args` is non-empty (e.g. `math 12 * 4` or `convert 50 USD in EUR`), evaluate the expression with `mathwrap` / `currency` / `units`.
             - Emit `Annotation { line: i, kind: "math", confidence: 0.95, value: result, chip: String::new(), why: "evaluated math expression".into() }`.
           - **`TriggerCategory::List`**:
             - Emit `Annotation { line: i, kind: "list", confidence: 0.85, value: String::new(), chip: "Convert to bullet list (Tab)".into(), why: "list trigger keyword".into() }`.
           - **`TriggerCategory::Check`**:
             - Emit `Annotation { line: i, kind: "checklist", confidence: 0.85, value: String::new(), chip: "Convert to checklist (Tab)".into(), why: "check trigger keyword".into() }`.
           - **`TriggerCategory::Code`**:
             - Emit `Annotation { line: i, kind: "code", confidence: 0.85, value: remaining_args.trim().to_string(), chip: "Format as code block (Tab)".into(), why: "code trigger keyword".into() }`.
           - **`TriggerCategory::Paste`**:
             - Emit `Annotation { line: i, kind: "paste", confidence: 0.95, value: String::new(), chip: "Start AutoPaste (Enter)".into(), why: "trigger for continuous clipboard capture".into() }`.
           - **`TriggerCategory::Timer`**:
             - If `remaining_args` is non-empty (e.g. `timer 5`, `timer pomo`, `timer 3:30`), parse timer duration/mode, emit `Annotation { line: i, kind: "timer", ... }`.
    2. Create `crates/plainpad-core/tests/trigger_evaluator_tests.rs`:
       - Test sum over preceding lines: `$10.50` + `$20.25` + `total` -> `$30.75`.
       - Test sum with alias `add` and `sum`.
       - Test avg over preceding lines: `10`, `20`, `30`, `avg` -> `20`.
       - Test count on list items: 4 lines followed by `count` -> `4 items`.
       - Test paragraph boundary: empty line isolates previous paragraphs so sum/avg only aggregate current block.
       - Test math trigger with trailing expression: `math 15 * 6` -> `90`.
       - Test `disableAllKeywords`: assert no trigger annotations emitted when set to true.
       - Test `allowArgumentsAfterKeyword`: test both `true` (e.g. `sum items:`, `count entries`) and `false` (only standalone matches).
       - Test custom user aliases (e.g. adding `"tally"` to `sum_triggers`).

    AVOID: Corrupting line indexing or breaking non-keyword math lines (e.g. `5 + 5 = 10`).
    USE: Paragraph-scoped slice scanning and robust currency formatting.
  </action>
  <verify>
    cargo test -p plainpad-core --test trigger_evaluator_tests
  </verify>
  <done>
    All 9 trigger categories evaluate accurately at paragraph scope, custom aliases and master toggles are verified, and all integration tests pass.
  </done>
</task>

<task type="auto">
  <name>Wire trigger chips, text block transformations, and trigger preferences in UI</name>
  <files>
    ui/src/main.ts
    ui/src/settings.ts
    ui/settings.html
  </files>
  <action>
    1. In `ui/src/main.ts`:
       - Enhance `acceptChip(lineIndex: number, text: string)`:
         - Handle `list` chip: convert preceding lines of the paragraph block into bullet list items (`- <item>`).
         - Handle `checklist` chip from `check` / `todo`: convert preceding lines of the paragraph block into checkboxes (`- [ ] <item>`).
         - Handle `code` chip: format target lines into fenced code block ```` ```<lang>\n...\n``` ```` using language from annotation value.
         - Handle `math` / `sum` / `avg` / `count` annotation chips or Tab acceptance: append evaluated result onto line.
         - Handle `paste` chip: trigger AutoPaste activation (prepared for Plan 4.2).
       - In `updateBackdrop()`: highlight recognized trigger words on lines with `.hl-mode-trigger` styling.
    2. In `ui/settings.html`:
       - In the "Intelligent Detection Words" tab:
         - Add master toggle `#setting-disable-all-keywords` ("Disable All Keywords").
         - Add inline arguments toggle `#setting-allow-args-after-keyword` ("Allow Arguments After Keywords").
         - Add keyword alias input fields for each category:
           - Sum triggers (`#setting-triggers-sum`, default: `sum, total, add`)
           - Avg triggers (`#setting-triggers-avg`, default: `avg, average`)
           - Math triggers (`#setting-triggers-math`, default: `math, convert, model`)
           - Count triggers (`#setting-triggers-count`, default: `count`)
           - List triggers (`#setting-triggers-list`, default: `list`)
           - Check triggers (`#setting-triggers-check`, default: `check, todo`)
           - Code triggers (`#setting-triggers-code`, default: `code`)
           - Paste triggers (`#setting-triggers-paste`, default: `paste`)
           - Timer triggers (`#setting-triggers-timer`, default: `timer`)
    3. In `ui/src/settings.ts`:
       - Add `loadTriggerSettings()`: call `get_trigger_config` and populate form elements.
       - Add `saveTriggerSettings()`: read form values, parse comma-separated alias lists, and call `set_trigger_config`.
       - Connect change event listeners on all trigger input controls.

    AVOID: Unbounded line mutations that wipe notes outside the current paragraph.
    USE: Paragraph-bounded line splicing and live preference synchronization via Tauri IPC.
  </action>
  <verify>
    cargo check --manifest-path src-tauri/Cargo.toml
  </verify>
  <done>
    Trigger chips for list, check, and code transform blocks cleanly, settings window configures aliases and master toggles, and src-tauri builds without warnings.
  </done>
</task>

</tasks>

<verification>
After all tasks complete, verify:
- [ ] `cargo test -p plainpad-core --test trigger_evaluator_tests` passes with 0 failures
- [ ] All 9 trigger categories (`sum`, `avg`, `math`, `count`, `list`, `check`, `code`, `paste`, `timer`) are recognized
- [ ] Sum, avg, and count aggregate numbers and items at paragraph and list scope
- [ ] `disableAllKeywords` disables all trigger keyword processing when true
- [ ] `allowArgumentsAfterKeyword` allows inline arguments when true and rejects when false
- [ ] Configurable aliases are persisted and loaded via Tauri IPC
- [ ] Interactive trigger chips transform text blocks into bullet lists, checklists, and code blocks
</verification>

<success_criteria>
- [ ] All 3 tasks verified passing
- [ ] Must-haves confirmed
- [ ] Zero test regressions across `plainpad-core`
</success_criteria>
