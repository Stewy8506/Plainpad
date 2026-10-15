---
phase: 7
plan: 2
wave: 2
gap_closure: false
depends_on:
  - "7.1"
files_modified:
  - crates/plainpad-core/src/extensions.rs
  - src-tauri/src/main.rs
  - ui/src/extensions.ts
  - ui/src/main.ts
  - ui/src/settings.ts
  - ui/settings.html
  - ui/styles.css
  - crates/plainpad-core/tests/ai_catalog_tests.rs
autonomous: true
must_haves:
  truths:
    - "Multi-provider AI model catalog fetches and normalizes models from OpenAI (/v1/models), Anthropic (/v1/models), and local Ollama (localhost:11434/api/tags)"
    - "Double-colon invocation ::commandName(...) and aliases parse parameters with type validation (string, paragraph, int, float, bool, mathExpression)"
    - "Command return types (insert, replaceLine, replaceAll, openURL) apply results accurately to the active note content or system browser"
    - "Built-in OpenAI sample extension (sample-openai-extension.js) registers gpt command and executes chat completions via callAPI"
    - "Settings modal provides dedicated Extensions & AI Providers section with API key management, model selector, and extension preference controls"
  artifacts:
    - crates/plainpad-core/tests/ai_catalog_tests.rs
    - ui/src/extensions.ts
    - ui/src/main.ts
    - ui/src/settings.ts
    - ui/settings.html
  key_links:
    - "src-tauri/src/main.rs exposes fetch_ai_models command"
    - "ui/src/main.ts intercepts :: syntax and delegates execution to ui/src/extensions.ts"
    - "ui/src/settings.ts interacts with store_api_key, list_api_keys, fetch_ai_models, and set_extension_preference"
---

# Plan 7.2: AI Model Catalog Integration (OpenAI, Anthropic, Ollama) and ::command Editor Syntax

<objective>
Implement the multi-provider AI model catalog and double-colon editor command invocation:
1. Multi-provider AI Model Catalog in Rust and IPC commands for OpenAI (`/v1/models`), Anthropic (`/v1/models`), and local Ollama (`http://localhost:11434/api/tags`), normalizing model lists with offline and error fallbacks.
2. Double-colon invocation syntax `::commandName(param1, param2)` (and command aliases like `gpt(...)`) in the editor, parsing typed parameters and dispatching execution with output handling (`insert`, `replaceLine`, `replaceAll`, `openURL`).
3. Settings UI for managing API keys, viewing installed extensions, configuring extension preferences (e.g. model selection, system prompt), and registering the built-in `sample-openai-extension.js`.

Purpose: Enable users to execute AI actions directly within their scratchpad notes using intuitive syntax `::command(...)`, with seamless multi-model support across cloud providers and local Ollama.
Output: `crates/plainpad-core/tests/ai_catalog_tests.rs`, updated `crates/plainpad-core/src/extensions.rs`, updated `src-tauri/src/main.rs`, updated `ui/src/extensions.ts`, `ui/src/main.ts`, `ui/src/settings.ts`, `ui/settings.html`, and `ui/styles.css`.
</objective>

<context>
Load these files for context:
- .gsd/SPEC.md (§8 JavaScript Extension System & AI Providers)
- .gsd/findings.md (§12 JavaScript Extension Runtime & Bridge)
- .gsd/REQUIREMENTS.md (REQ-25)
- crates/plainpad-core/resources/antinote-extensions-base-v0.0.1.js
- crates/plainpad-core/resources/sample-openai-extension.js
- crates/plainpad-core/src/extensions.rs
- ui/src/extensions.ts
- ui/src/main.ts
- ui/src/settings.ts
- ui/settings.html
</context>

<tasks>

<task type="auto">
  <name>Implement Multi-Provider AI Model Catalog (OpenAI, Anthropic, Ollama) and Sample Extension Registration</name>
  <files>
    crates/plainpad-core/src/extensions.rs
    src-tauri/src/main.rs
    ui/src/extensions.ts
    crates/plainpad-core/tests/ai_catalog_tests.rs
  </files>
  <action>
    Extend `crates/plainpad-core/src/extensions.rs` and `src-tauri/src/main.rs` to implement the AI Model Catalog.

    Steps:
    1. Define `AiModel` struct:
       - `pub id: String`, `pub name: String`, `pub provider: String`, `pub context_length: Option<u32>`.
    2. Implement provider fetchers and JSON response parsers:
       - `fetch_openai_models(api_key: &str) -> Result<Vec<AiModel>, String>`:
         - GET `https://api.openai.com/v1/models`
         - Header: `Authorization: Bearer {api_key}`
         - Parse `data[].id`, filter relevant chat/reasoning models (`gpt-4*`, `gpt-3.5*`, `o1*`, `o3*`), sort alphabetically.
       - `fetch_anthropic_models(api_key: &str) -> Result<Vec<AiModel>, String>`:
         - GET `https://api.anthropic.com/v1/models?limit=1000`
         - Headers: `x-api-key: {api_key}`, `anthropic-version: 2023-06-01`
         - Parse `data[].id` and `data[].display_name`.
       - `fetch_ollama_models(base_url: Option<&str>) -> Result<Vec<AiModel>, String>`:
         - GET `{base_url}/api/tags` (default `http://localhost:11434/api/tags`)
         - Parse `models[].name`. If connection refused or timeout, return graceful error `Ollama is not running on {base_url}`.
    3. Expose Tauri IPC command in `src-tauri/src/main.rs`:
       - `fetch_ai_models(provider: String) -> Result<Vec<AiModel>, String>`
    4. In `ui/src/extensions.ts`, embed the built-in `sample-openai-extension.js`:
       - On runtime initialization, register the OpenAI extension root, its preferences (`model`, `systemPrompt`), and the `gpt` command.
       - Set default model preference to `gpt-4o`.
    5. Write integration tests in `crates/plainpad-core/tests/ai_catalog_tests.rs`:
       - Test JSON response parsing for OpenAI `/v1/models` response format.
       - Test JSON response parsing for Anthropic `/v1/models` response format.
       - Test JSON response parsing for Ollama `/api/tags` response format.
       - Verify normalized `AiModel` outputs and error handling when credentials are missing.

    AVOID: Crashing or hanging when Ollama is not running locally. Return descriptive offline error states.
    USE: Standard reqwest timeout (5s) for model discovery network calls.
  </action>
  <verify>
    cargo test --test ai_catalog_tests && npm run typecheck
  </verify>
  <done>
    `cargo test --test ai_catalog_tests` passes and TypeScript typecheck succeeds with zero errors.
  </done>
</task>

<task type="auto">
  <name>Implement Double-Colon Command Syntax (::command) and Execution Handlers in Editor</name>
  <files>
    ui/src/main.ts
    ui/src/extensions.ts
  </files>
  <action>
    Implement double-colon command invocation syntax in `ui/src/extensions.ts` and `ui/src/main.ts`.

    Steps:
    1. In `ui/src/extensions.ts`, implement command parser:
       - `parseCommandInvocation(text: string): { commandName: string, rawParams: any[] } | null`:
         - Matches `::([a-zA-Z0-9_-]+)(?:\((.*)\))?` (e.g. `::gpt(Explain quantum computing, 300)`).
         - Also matches direct command alias `gpt(...)` if configured or explicitly prefixed.
         - Parses comma-separated arguments, properly handling double/single-quoted strings with commas inside, numbers, and booleans.
       - `executeCommand(name: string, rawParams: any[], currentLine: string, fullNote: string): Promise<ReturnObject>`:
         - Resolves command from `commandRegistry`.
         - Calls `command.getParsedParams(payload)`.
         - Applies `dataScope` to payload.
         - Awaits `command.execute(payload)`.
         - Returns structured `ReturnObject`.
    2. In `ui/src/main.ts`:
       - Intercept Enter / Tab key events when current line ends with `::commandName(...)` or user presses execution shortcut (Ctrl+Enter / Cmd+Enter).
       - Show execution status indicator (e.g. `#extension-status-indicator` displaying "Running ::gpt...").
       - Execute command asynchronously.
       - Process `ReturnObject`:
         - If `status === "success"`:
           - According to `command.type`:
             - `"insert"`: Inserts `payload` at cursor position (replacing the invocation expression).
             - `"replaceLine"`: Replaces entire current line with `payload`.
             - `"replaceAll"`: Replaces entire note content with `payload`.
             - `"openURL"`: Opens URL via browser (`window.open` or Tauri shell `open`).
           - Trigger note auto-save to SQLite.
         - If `status === "error"`:
           - Display toast / inline notification with `message` (e.g. "OpenAI API error: Rate limit exceeded").
       - Hide execution status indicator.

    AVOID: Blocking the UI thread while awaiting network LLM completions.
    USE: Async execution with a visual loading/pulsing state so the user knows an AI completion is generating.
  </action>
  <verify>
    npm run build:ui
  </verify>
  <done>
    ui/src/main.ts and ui/src/extensions.ts build cleanly, parsing ::command(...) expressions and handling insert, replaceLine, replaceAll, and openURL return types.
  </done>
</task>

<task type="auto">
  <name>Implement Extensions & API Keys Management UI in Settings</name>
  <files>
    ui/src/settings.ts
    ui/settings.html
    ui/styles.css
  </files>
  <action>
    Add a dedicated "Extensions & AI Providers" configuration view to `ui/settings.html`, `ui/src/settings.ts`, and `ui/styles.css`.

    Steps:
    1. In `ui/settings.html`, add section `#settings-extensions`:
       - Section header: "Extensions & AI Providers"
       - **API Keys Card**:
         - OpenAI Key (`apikey_openai`): Password input with toggle visibility eye icon, "Save" button, "Clear" button, and status badge (`Configured` in green / `Not Set` in muted grey).
         - Anthropic Key (`apikey_anthropic`): Password input with toggle, "Save" button, "Clear" button, and status badge.
         - Ollama Host: Text input (default `http://localhost:11434`), "Test Connection" button, and status badge (`Online` / `Offline`).
       - **Model Catalog Picker**:
         - Dropdown to select active default model populated via `fetch_ai_models` (or fallback defaults: `gpt-4o`, `gpt-4o-mini`, `claude-3-5-sonnet`, `llama3`).
       - **Installed Extensions & Preferences**:
         - Card listing loaded extensions (e.g. OpenAI extension).
         - Displays extension metadata: version, author, category, `dataScope` pill (`line`), approved endpoints list.
         - Preference inputs for registered preferences (`model` select dropdown, `systemPrompt` textarea).
    2. In `ui/src/settings.ts`:
       - On settings load, query `list_api_keys` to populate key status badges and masked values.
       - Wire "Save" and "Clear" buttons to `store_api_key` and `delete_api_key`.
       - Wire "Test Connection" and "Refresh Models" buttons to `fetch_ai_models`.
       - Wire extension preference changes to `set_extension_preference`.
    3. In `ui/styles.css`:
       - Add clean dark glassmorphic styling matching Plainpad's aesthetic for key input rows, status badges (`.status-badge.configured`, `.status-badge.missing`), model selectors, and extension cards.

    AVOID: Plaintext exposure of saved API keys in the DOM; use masked placeholders (`sk-...xxxx`).
    USE: Consistent glassmorphic card design and clear validation badges.
  </action>
  <verify>
    npm run build:ui && npm run typecheck
  </verify>
  <done>
    Settings modal compiles and renders the Extensions & AI Providers configuration UI with zero TypeScript errors.
  </done>
</task>

</tasks>

<verification>
After all tasks, verify:
- [ ] AI Model Catalog correctly fetches and normalizes models from OpenAI, Anthropic, and local Ollama
- [ ] Unit and integration tests pass via `cargo test --test ai_catalog_tests`
- [ ] Editor detects `::commandName(...)` syntax, parses arguments, and applies insert/replaceLine/replaceAll/openURL actions
- [ ] Sample OpenAI extension is registered and operational
- [ ] Settings modal allows saving API keys, viewing configured status, selecting models, and customizing extension preferences
- [ ] Full UI build and TypeScript validation pass via `npm run build:ui && npm run typecheck`
</verification>

<success_criteria>
- [ ] All tasks verified passing
- [ ] Must-haves confirmed
- [ ] No regressions in tests
</success_criteria>
