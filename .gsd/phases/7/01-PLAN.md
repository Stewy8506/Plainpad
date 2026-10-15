---
phase: 7
plan: 1
wave: 1
gap_closure: false
depends_on: []
files_modified:
  - crates/plainpad-core/src/extensions.rs
  - crates/plainpad-core/src/lib.rs
  - crates/plainpad-core/tests/extensions_tests.rs
  - src-tauri/src/main.rs
  - ui/src/extensions.ts
autonomous: true
must_haves:
  truths:
    - "SQLite credential store securely saves, retrieves, masks, and deletes API keys (apikey_*) in app_state"
    - "Native call_api bridge verifies target endpoints against extension's approved HTTPS origins, rejecting unauthorized URLs"
    - "Native call_api bridge replaces {{API_KEY}} placeholders with actual secrets from storage and dispatches HTTP requests via native client"
    - "Sandboxed JS runtime executes antinote-extensions-base-v0.0.1.js, defining Extension, Command, Preference, Parameter, TutorialCommand, ReturnObject, and MathEvaluator"
    - "JS runtime exposes native callAPI bridge, getExtensionPreference bridge, and __evaluateMathExpression bridge"
    - "Data access scope (none, line, full) strictly controls note content passed to extension commands"
  artifacts:
    - crates/plainpad-core/src/extensions.rs
    - crates/plainpad-core/tests/extensions_tests.rs
    - ui/src/extensions.ts
  key_links:
    - "src-tauri/src/main.rs registers call_api and credential commands from crates/plainpad-core/src/extensions.rs"
    - "ui/src/extensions.ts binds window.callAPI to Tauri IPC call_api command"
    - "ui/src/extensions.ts loads crates/plainpad-core/resources/antinote-extensions-base-v0.0.1.js"
---

# Plan 7.1: Embedded JS Runtime executing antinote-extensions-base-v0.0.1.js with Secure callAPI Bridge

<objective>
Implement the foundational JavaScript extension runtime and secure native API bridge:
1. SQLite credential store in Rust for managing API keys (`apikey_openai`, `apikey_anthropic`, etc.) with endpoint domain whitelisting and automatic `{{API_KEY}}` placeholder substitution.
2. Sandboxed JS runtime running `antinote-extensions-base-v0.0.1.js` with global registries (`commandRegistry`, `extensionRegistry`), constructor compatibility (both positional arguments and config objects), and native bridge functions (`callAPI`, `getExtensionPreference`, `__evaluateMathExpression`).
3. Privacy data scoping (`none`, `line`, `full`) ensuring extensions only receive permitted note content.

Purpose: Antinote allows arbitrary user-authored extensions to execute while guaranteeing security: extensions never receive raw API keys (the native bridge injects them securely), extensions cannot contact unauthorized domains, and notes are not leaked beyond the extension's declared dataScope.
Output: `crates/plainpad-core/src/extensions.rs`, `src-tauri/src/main.rs` IPC commands, `ui/src/extensions.ts`, and `crates/plainpad-core/tests/extensions_tests.rs`.
</objective>

<context>
Load these files for context:
- .gsd/SPEC.md (§8 JavaScript Extension System & AI Providers)
- .gsd/findings.md (§12 JavaScript Extension Runtime & Bridge)
- .gsd/REQUIREMENTS.md (REQ-23, REQ-24)
- crates/plainpad-core/resources/antinote-extensions-base-v0.0.1.js
- crates/plainpad-core/resources/sample-openai-extension.js
- crates/plainpad-core/src/store/mod.rs
- crates/plainpad-core/src/store/notes.rs
- src-tauri/src/main.rs
</context>

<tasks>

<task type="auto">
  <name>Implement SQLite Credential Store, Security Policies, and Native call_api Bridge in Rust</name>
  <files>
    crates/plainpad-core/src/extensions.rs
    crates/plainpad-core/src/lib.rs
    src-tauri/src/main.rs
  </files>
  <action>
    Create `crates/plainpad-core/src/extensions.rs` and export it in `crates/plainpad-core/src/lib.rs`.
    
    Steps:
    1. Implement `CredentialStore` utilizing `SqliteStore`'s `app_state` table:
       - Keys stored with prefix `apikey_` (e.g. `apikey_openai`, `apikey_anthropic`).
       - `store_api_key(conn, key_name, key_value)`: Saves raw key in `app_state`.
       - `get_api_key(conn, key_name)`: Retrieves raw key for native bridge use only.
       - `list_api_keys(conn)`: Returns metadata list `Vec<ApiKeyMeta>` with `key_name`, `is_configured: bool`, and `masked_preview: String` (e.g. `sk-...abcd`). Raw keys are NEVER returned in list queries.
       - `delete_api_key(conn, key_name)`: Removes key from `app_state`.
    2. Implement `EndpointSecurity`:
       - `validate_endpoint(url: &str, approved_endpoints: &[String]) -> Result<(), SecurityError>`:
         - Checks that `url` matches at least one prefix/origin in `approved_endpoints` (e.g. `https://api.openai.com/v1/chat/completions`).
         - Validates that protocol is `https://`, with explicit exception for localhost/127.0.0.1 (`http://localhost:*` or `http://127.0.0.1:*` for local Ollama).
         - Rejects unauthorized domains or malformed URLs with descriptive error.
    3. Implement `DataScope` enum:
       - Variants: `None`, `Line`, `Full`.
       - `filter_content(scope: DataScope, line_text: &str, full_note: &str) -> (String, String)`:
         - `DataScope::None` -> `("", "")`
         - `DataScope::Line` -> `(line_text, "")`
         - `DataScope::Full` -> `(line_text, full_note)`
    4. Implement `call_api`:
       - Accepts `CallApiRequest`: `api_key_name: String`, `url: String`, `method: String`, `headers: HashMap<String, String>`, `body: Option<String>`, `approved_endpoints: Vec<String>`.
       - Validates endpoint via `EndpointSecurity`.
       - Fetches secret key for `api_key_name` from `CredentialStore`. If key is not configured, returns clear error `"API key '{api_key_name}' not configured in Settings"`.
       - Replaces `{{API_KEY}}` placeholders across URL, headers, and body with the secret key.
       - Dispatches request using `reqwest::blocking::Client` (or async client in Tauri).
       - Returns `CallApiResponse`: `success: bool`, `status_code: u16`, `data: Option<String>`, `error: Option<String>`.
    5. Register Tauri IPC commands in `src-tauri/src/main.rs`:
       - `store_api_key(key_name: String, key_value: String)`
       - `list_api_keys()`
       - `delete_api_key(key_name: String)`
       - `call_api(api_key_name: String, url: String, method: String, headers: String, body: Option<String>, approved_endpoints: Vec<String>) -> Result<CallApiResponse, String>`
       - `get_extension_preference(extension_name: String, key: String) -> Result<Option<String>, String>`
       - `set_extension_preference(extension_name: String, key: String, value: String) -> Result<(), String>` (saved in `app_state` under `ext_pref_{extension_name}_{key}`).

    AVOID: Exposing raw API keys to the frontend JavaScript runtime or extensions. All replacement must occur inside Rust native code before network dispatch.
    USE: Exact Antinote placeholder syntax `{{API_KEY}}` in headers and request bodies.
  </action>
  <verify>
    cargo check -p plainpad-core && cargo check -p plainpad
  </verify>
  <done>
    Rust core extensions module compiles cleanly, exports credential storage, endpoint security, and call_api IPC commands, with zero compiler warnings.
  </done>
</task>

<task type="auto">
  <name>Implement Sandboxed JS Extension Runtime and Native Bridge Hooks in TypeScript</name>
  <files>
    ui/src/extensions.ts
  </files>
  <action>
    Create `ui/src/extensions.ts` to manage the embedded JS runtime and load `antinote-extensions-base-v0.0.1.js`.

    Steps:
    1. Define base class interfaces and constructors matching `crates/plainpad-core/resources/antinote-extensions-base-v0.0.1.js`:
       - `Extension(config | name, version, endpoints, requiredAPIKeys, author, category, dataScope)`
       - `Command(config | name, parameters, type, helpText, tutorials, extension)`
       - `Preference(config | key, label, type, defaultValue, options, helpText)`
       - `Parameter(config | type, name, helpText, default, required)`
       - `TutorialCommand(config | command, description)`
       - `ReturnObject(config | status, message, payload)`
       - `MathEvaluator` with `eval`, `evalSafe`, `isMathExpression`, `parseNumeric`.
       IMPORTANT: Support BOTH object literal configuration `{ name: ... }` and positional arguments `(name, version, ...)` as used in `sample-openai-extension.js`.
    2. Setup global registries:
       - `global.commandRegistry = []` (or `window.commandRegistry`)
       - `global.extensionRegistry = []` (or `window.extensionRegistry`)
    3. Implement global bridge functions accessible to extension code:
       - `callAPI(apiKeyName: string, url: string, method: string, headers: string | Record<string, string>, body?: string)`:
         - Looks up current calling extension (or matches approved endpoints).
         - Invokes Tauri command `call_api`.
         - Returns `{ success: boolean, statusCode?: number, data?: string, error?: string }`.
       - `getExtensionPreference(extensionName: string, key: string)`:
         - Invokes Tauri command `get_extension_preference` (or reads cached preferences).
       - `__evaluateMathExpression(expression: string)`:
         - Calls Plainpad's math engine bridge and returns `{ success: true, value: number }` or `{ success: false, error: string }`.
    4. Implement `ExtensionRuntime` class:
       - `init()`: Injects base JS classes and bridges into evaluation environment.
       - `registerExtension(extensionScript: string)`: Evaluates extension script in isolated context and captures registered commands.
       - `getCommands(): Command[]`: Returns all registered commands.
       - `findCommand(name: string): Command | undefined`: Matches command by `name` or in `aliases`.
       - `buildPayload(command: Command, rawParams: any[], currentLine: string, fullNote: string)`: Applies extension's declared `dataScope`:
         - `"none"`: `line: "", note: ""`
         - `"line"`: `line: currentLine, note: ""`
         - `"full"`: `line: currentLine, note: fullNote`
         - Parses parameters via `command.getParsedParams(payload)`.
       - `executeCommand(name: string, rawParams: any[], currentLine: string, fullNote: string): Promise<ReturnObject>`:
         - Executes `command.execute(payload)`.
         - Ensures result is normalized to `ReturnObject { status: "success"|"error", message: string, payload: any }`.

    AVOID: Allowing user extension scripts to modify DOM or access local file system directly. All IO must go through the declared `callAPI` bridge.
    USE: Strict `dataScope` filtering before passing any note content to `payload`.
  </action>
  <verify>
    npm run typecheck
  </verify>
  <done>
    ui/src/extensions.ts compiles with zero TypeScript errors, properly sets up base classes and global registries, and provides callAPI bridge.
  </done>
</task>

<task type="auto">
  <name>Build Extension Runtime Integration Test Suite</name>
  <files>
    crates/plainpad-core/tests/extensions_tests.rs
  </files>
  <action>
    Create `crates/plainpad-core/tests/extensions_tests.rs` to empirically verify all backend security policies, credential storage, and API bridge logic.

    Steps:
    1. Test `CredentialStore`:
       - Store, retrieve, and delete API keys in in-memory `SqliteStore`.
       - Verify `list_api_keys` returns masked strings (e.g. `sk-...1234`) and never leaks the raw key.
    2. Test `EndpointSecurity`:
       - Approved domain (`https://api.openai.com/v1/chat/completions`) passes validation for `["https://api.openai.com"]`.
       - Unauthorized domain (`https://malicious-site.com/steal`) is rejected.
       - Localhost URL (`http://localhost:11434/api/tags`) is permitted.
       - Non-HTTPS external URL (`http://api.openai.com/...`) is rejected.
    3. Test `{{API_KEY}}` Placeholder Substitution:
       - Header `Authorization: Bearer {{API_KEY}}` becomes `Authorization: Bearer sk-test-key-1234`.
       - URL with `?key={{API_KEY}}` correctly substitutes key.
       - Request body with `{"key": "{{API_KEY}}"}` correctly substitutes key.
    4. Test `DataScope` filtering:
       - `DataScope::None` returns empty line and empty note.
       - `DataScope::Line` returns current line and empty note.
       - `DataScope::Full` returns both current line and full note text.
    5. Test `call_api` error handling:
       - Calling with unconfigured key returns descriptive error.
       - Calling unauthorized endpoint returns security error before any network request.

    AVOID: Relying on external network requests during tests; mock network or verify pre-flight validation.
    USE: Assertions on exact Antinote error strings and security constraints.
  </action>
  <verify>
    cargo test --test extensions_tests
  </verify>
  <done>
    `cargo test --test extensions_tests` passes all tests with zero failures.
  </done>
</task>

</tasks>

<verification>
After all tasks, verify:
- [ ] SQLite credential store correctly saves and masks API keys
- [ ] Endpoint security rejects unauthorized URLs and non-HTTPS external protocols
- [ ] Placeholder substitution injects actual secret into {{API_KEY}}
- [ ] TypeScript extensions runtime compiles and exposes base constructors and callAPI bridge
- [ ] Integration tests pass via `cargo test --test extensions_tests`
</verification>

<success_criteria>
- [ ] All tasks verified passing
- [ ] Must-haves confirmed
- [ ] No regressions in tests
</success_criteria>
