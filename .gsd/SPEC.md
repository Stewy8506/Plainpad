# SPEC.md — Plainpad 1:1 Parity Specification with Antinote

> **Status**: `FINALIZED`
>
> ⚠️ **Planning Lock**: This specification defines the complete, exhaustive feature set extracted via binary reverse engineering of Antinote v2.1.0 (`com.chabomakers.Antinote`). All implementation tasks in Plainpad must conform to this document without omission.

---

## 1. Vision & Executive Summary

**Plainpad** is an intelligent, distraction-free plaintext scratchpad and natural-language calculator for desktop platforms (Windows, Linux, macOS). It provides a floating, instant-access scratchpad that lives under a global hotkey and menu bar/system tray icon.

The reference benchmark is **Antinote (macOS v2.1.0)**. Plainpad must match Antinote in **every single user-facing capability, interaction model, data model attribute, calculation grammar, extension API, visual shader effect, and keyboard behavior** without compromising cross-platform performance.

---

## 2. Complete Reverse-Engineered Data Model Specification

Extracted directly from Antinote's Core Data Model (Version 9 — `AntinoteDataModel 9 - added isArchived isPrivate.mom` and `cd-v1-notes.sqlite`):

### 2.1 Entity: `Note`
Represents an individual scratchpad note.

| Field Name | Type | Constraints / Defaults | Description |
|---|---|---|---|
| `id` | UUID / String | Primary Key, Non-null | Unique note identifier. |
| `content` | String | Default: `""` | Complete raw plaintext content of the note. |
| `created` | DateTime | Non-null | Timestamp when note was created. |
| `lastModified` | DateTime | Non-null | Timestamp of last user edit or programmatic update. |
| `dbIndex` | Int32 | Indexed | Ordering index within active note stack. |
| `isSlotted` | Boolean | Default: `false` | Whether this note occupies one of the 9 permanent quick-access slots. |
| `slotIndex` | Int32 | Range: `0..8`, Default: `-1` | Slot position (0=Hermes, 1=Apollo/Aphrodite, 2=Ares, 3=Cronus, 4=Gaia, 5=Hades, 6=Ouranos, 7=Poseidon, 8=Zeus). |
| `isLocked` | Boolean | Default: `false` | Prevents accidental modification or deletion. |
| `isArchived` | Boolean | Default: `false` | Archived out of active stack without deleting. |
| `isPrivate` | Boolean | Default: `false` | Hidden note requiring biometric/password unlock. |
| `isTutorial` | Boolean | Default: `false` | Built-in interactive onboarding tutorial notes. |
| `softDeleted` | Boolean | Default: `false` | Marked for deletion into "The Void" (Trash). |
| `deletedAt` | DateTime (Nullable) | Default: `null` | Timestamp when moved to The Void; used for retention/pruning. |
| `skippedURLs` | Array<String> | JSON / Transformable | URLs explicitly opted out of Antilink shortening. |
| `ckChangeTag` | String (Nullable) | Sync tag | Cloud / remote sync change tag. |
| `ckSystemFields` | Binary / Base64 | Sync metadata | Serialized remote sync engine system state. |

**Relationships:**
- To-Many `ChecklistItem` (Cascade delete)
- To-Many `Antilink` (Cascade delete)
- To-Many `Attachment` (Cascade delete)
- To-One `SyncState` (Cascade delete)

### 2.2 Entity: `ChecklistItem`
Tracks interactive checkboxes inline within note text.

| Field Name | Type | Description |
|---|---|---|
| `id` | UUID | Unique item identifier. |
| `content` | String | Text label associated with this checklist row. |
| `checked` | Boolean | Checked state (`true` = completed / `[X]`, `false` = pending / `[ ]`). |
| `index` | Int32 | Sequential index within note's list items. |
| `verticalPosition` | Double | Cached layout offset for smooth visual rendering. |
| `responsibilityRangeLocation` | Int32 | Character start index of checklist item in `Note.content`. |
| `responsibilityRangeLength` | Int32 | Character length of the item span. |
| `isHidden` | Boolean | Hidden state (e.g., when collapsed or filtered). |

### 2.3 Entity: `Antilink`
Smart inline URL shortening and representation.

| Field Name | Type | Description |
|---|---|---|
| `id` | UUID | Unique antilink identifier. |
| `fullUrl` | String | Original long destination URL. |
| `shortenedUrl` | String | Compact display label (e.g., `github.com/…/repo`). |
| `currentRangeLocation` | Int32 | Text start offset within note text. |
| `currentRangeLength` | Int32 | Length of the shortened pill span. |

### 2.4 Entity: `Attachment`
Attached files and media assets (images, screenshots, audio, documents).

| Field Name | Type | Description |
|---|---|---|
| `id` | UUID | Unique attachment identifier. |
| `fileName` | String | Original filename. |
| `fileUrl` | String | Local file storage path or hash reference. |
| `fileSize` | Int64 | Size in bytes. |
| `fileType` | String | Extension / identifier (e.g., `png`, `pdf`, `m4a`). |
| `mimeType` | String | Standard MIME type. |
| `locationType` | String | Storage location strategy (`local`, `cached`, `remote`). |
| `sortOrder` | Int32 | Order in attachment gallery. |
| `created` | DateTime | Attachment creation timestamp. |

### 2.5 Entity: `SyncState`
State store for client synchronization and conflict resolution.

| Field Name | Type | Description |
|---|---|---|
| `baseContent` | String | Base text at last successful sync checkpoint. |
| `isPositioned` | Boolean | Ordering synchronization status. |
| `pendingEditSnapshot` | Binary | Pending local diff awaiting sync dispatch. |
| `systemFields` | Binary | Engine metadata. |
| `lastSyncedAt` | DateTime | Last synchronization timestamp. |
| `needsUpload` | Boolean | Dirty flag indicating local changes need sync. |
| `changeTag` | String | Version change tag. |

---

## 3. The 9 Slotted Notes Architecture (Greek Gods System)

Antinote maintains a dedicated permanent carousel of **9 Slotted Notes**, named after Greek deities:
1. **Slot 0: Hermes** (First slotted note — "It'll stay with you")
2. **Slot 1: Aphrodite / Apollo**
3. **Slot 2: Ares**
4. **Slot 3: Cronus**
5. **Slot 4: Gaia**
6. **Slot 5: Hades**
7. **Slot 6: Ouranos**
8. **Slot 7: Poseidon**
9. **Slot 8: Zeus**

### Behavioral Rules:
- **Persistent Bottom Bar UI:** An indicator bar (`SlottedNotesBottomBar`) with 9 dot/pill icons (`SlottedNoteIconView`) indicates the active slot.
- **Direct Navigation Hotkeys:** `Ctrl+1` through `Ctrl+9` (or `Cmd+1..9` on Mac) instantly jumps to the corresponding slotted note.
- **Cycle Navigation:** `Ctrl+Tab` / `Ctrl+Shift+Tab` cycles sequentially between active slots.
- **Slot Invariance:** Slotted notes **cannot be batch-deleted** and **cannot be promoted to front** because their position is fixed in the permanent registry.
- **Unslotted / Scratch Notes:** Notes created beyond the 9 slots form a fluid scratch stack. Users can promote any note to the front, pin it, or send it to The Void.

---

## 4. Natural-Language Math & Conversion Engine

Antinote embeds an intelligent mathematical parsing and conversion engine based on tokenizer/expression-rewriter rules.

### 4.1 Number Formatting & Grammar
- **Decimals:** `4.25`, `.5`, `1,250.50` (respects decimal separator settings: `.` or `,`).
- **Fractions:** Matches regex `(\d+ ?\d*\/\d+|(?:\d+(?:\.\d+)?|\.\d+))` — accepts `1/2`, `3 1/4`, `5 7/8`.
- **Scientific Notation & Exponents:** `1e6`, `2^8`, `2**8`.
- **Precedence & Rewriting:** Standard algebraic precedence `()`, `**`, `*`, `/`, `+`, `-`.
- **Implicit Multiplication:** `4(12 + 3)` or `5 kg * 2`.
- **Angle Modes:** Configurable via `angleMeasurementMode`: `degrees` vs `radians`. Built-in functions: `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `sinh`, `cosh`, `tanh`, `asinh`, `acosh`, `atanh`, `sqrt`, `cuberoot`.

### 4.2 Comprehensive Unit Conversion Matrix
Plainpad must support all 6 conversion domains with identical aliasing:

1. **Distance:**
   - Units: `millimeter` (`mm`), `centimetre` (`cm`), `meter` (`m`), `kilometer` (`km`), `inch` (`in`, `"`), `foot` / `feet` (`ft`, `'`), `yard` / `yards` (`yd`), `mile` / `miles` (`mi`).
2. **Area:**
   - Units: `sq mm` / `square millimeters`, `sq cm` / `square centimeters`, `square foot` / `square feet` (`sq ft`), `square yard` / `square yards` (`sq yd`), `square kilometer` / `square kilometers` (`sq km`), `cuerda` / `cuerdas`, `dunam`, `bigha`, `acre` / `acres`, `hectare` / `hectares`.
3. **Mass / Weight:**
   - Units: `milligrams` (`mg`), `gram` / `grams` (`g`), `kilograms` / `kilos` (`kg`), `tonne` / `metric ton`, `pound` / `pounds` (`lb`, `lbs`), `ounce` / `ounces` (`oz`), `stones` (`st`), `imperial ton` / `uk ton` / `long tons`.
4. **Volume:**
   - Units: `milliliters` (`ml`), `litre` / `litres` (`l`), `cubic millimeter` (`cu mm`), `cubic meter` (`m3`), `cubic foot` (`cu ft`), `teaspoon` / `teaspoons` (`tsp`, `tsps`, `ukTeaspoon`), `tablespoon` / `tablespoons` (`tbsp`, `tbsps`, `ukTablespoon`), `pint` / `pints` (`pt`, `uk pint`), `imperial gallon` / `us gallon` (`gal`), `fluid ounce` / `imperial fluid ounce` (`fl oz`, `ukFluidOunce`).
5. **Temperature:**
   - Units: `celsius` / `degrees C` / `°C`, `fahrenheit` / `degrees F` / `°F`, `kelvin` / `K`.
6. **Currency:**
   - Support for 160+ fiat currencies and top cryptocurrencies (`USD`, `EUR`, `GBP`, `CAD`, `AUD`, `JPY`, `CHF`, `INR`, `BTC`, `ETH`, etc.).
   - Background rate sync with fallback cache (`_manualCurrencyRates`, `_lastCurrencyUpdateInterval`, `lastCurrencyUpdateDate`).
   - Thousands separators (`thousandsSeparator`, `_useThousandSeparatorsStorage`, `numberOfDecimalPoints`).

---

## 5. Natural-Language Triggers & Keyword Taxonomy

Antinote processes plain English sentences with configurable trigger keywords:

| Category | Storage Key | Default Triggers | Behavior |
|---|---|---|---|
| **Sum** | `sumTriggers` / `sumMainTrigger` | `sum`, `total`, `add` | Aggregates all numbers and currencies in the preceding paragraph or list. |
| **Average** | `avgTriggers` / `avgMainTrigger` | `avg`, `average` | Computes the arithmetic mean of all numeric values. |
| **Math** | `mathTriggers` / `mathMainTrigger` | `math`, `convert`, `model` | Evaluates inline math, currency conversions ("50 USD in EUR"), and unit conversions. |
| **Count** | `countTriggers` / `countMainTrigger` | `count` | Counts items, lines, words, or people in a list. |
| **List** | `listTriggers` / `listMainTrigger` | `list` | Converts following lines into an indented formatted bullet list. |
| **Check** | `checkTriggers` / `checkMainTrigger` | `check`, `todo` | Converts following items into interactive checklist checkboxes (`- [ ]`). |
| **Code** | `codeTriggers` / `codeMainTrigger` | `code` | Formats current/selected text into a rounded code block with syntax highlighting. |
| **Paste** | `pasteTriggers` / `pasteMainTrigger` | `paste` | Triggers clipboard AutoPaste / smart OCR extraction. |
| **Timer** | `timerTriggers` / `timerMainTrigger` | `timer` | Parses and starts an inline timer, stopwatch, or Pomodoro. |

**Settings Controls:**
- `disableAllKeywords`: Master toggle to disable all trigger processing.
- `allowArgumentsAfterKeyword`: Allows arguments inline following the trigger word.
- `customKeywordLabel` & `customKeywordDescription`: Allows user-defined custom aliases.

---

## 6. Timer & Pomodoro Engine

Antinote features a complete inline timer and alarm subsystem:

### 6.1 Syntax Variations
1. `timer` → Starts a **Stopwatch** counting up from `00:00`.
2. `timer 5` → Starts a **Countdown** for 5 minutes (`05:00`).
3. `timer 3:30` → Starts a **Countdown** for 3 minutes and 30 seconds (`03:30`).
4. `timer pomo` → Starts a classic **Pomodoro session** (25 minutes work, 5 minutes break).
5. `timer 57 11` → Starts a custom Pomodoro (57 minutes work, 11 minutes break).
6. `timer p` → Pauses or resumes the active timer.
7. `timer r` → Restarts the active timer from its initial duration.
8. `timer s` → Stops and dismisses the active timer.

### 6.2 Visuals & Audio Signals
- **Interactive Timer Pill (`TimerView`):** Shows animated progress circle (`circleDiameter`, `circleSpacing`), time remaining, and state (`isPaused`, `isWorkPhase`, `cycleCount`).
- **Sound Effects:** Shipped audio alerts played on transitions:
  - `backtowork.m4a`: End of break, resume work phase.
  - `countdowndone.m4a`: Countdown expiration chime.
  - `workdone.m4a`: Work phase complete, start break.
- **Alert Options:**
  - `timerCountdownPlaySound`: Sound notification toggle.
  - `timerPomoShowFullScreenAlert`: Full-screen modal overlay option (`TimerCompleteAlertView`).
  - `pauseTheTimerIfAppClosed`: Pauses timers if app is hidden or closed.

### 6.3 Pomodoro Break Screen & Fullscreen Takeover (`BreakTimeView`)
When a Pomodoro work phase finishes and break begins (or when `timerPomoShowFullScreenAlert` is enabled):
1. **Fullscreen Desktop Takeover**:
   - Tauri window summons/expands to borderless fullscreen always-on-top, immersing the display with dark atmospheric frosted glass (`backdrop-filter: blur(48px)`), radial vignette, and subtle ambient magenta/violet lighting.
2. **Visual Layout & Iconography**:
   - Centered mint-green line-art orbit illustration (`#4ade80`).
   - "Break Time" headline with tracked uppercase category badge (`WILDLIFE FACT-FILE`, `SPACE TRIVIA`, `MINDFUL MOMENTS`).
   - Rotating local catalog of bite-sized wildlife facts and mindfulness thoughts (e.g. *"Ostriches can run faster than horses."*).
   - Live interactive countdown (`Break time remaining: [ || ] mm:ss`) synchronized in real-time with backend `TimerEngine` break phase.
3. **"Sneak a Peek" & "No-Peek Streak" Gamification**:
   - "Sneak a peek" button temporarily minimizes or un-blurs the overlay so the user can inspect their screen if urgent.
   - "End Pomodoro" button aborts the session.
   - Persistent SQLite streak tracker: `current_no_peek_streak` and `best_no_peek_streak`. Completing a break without peeking increments the streak; sneaking a peek resets current streak to 0 while preserving best record.
   - Displays `No-Peek Streak: X (Best: Y)` with golden accent numbers.

---

## 7. Editor Capabilities & Rich Inline Elements

Plainpad's editor must provide rich inline behaviors over raw plaintext:

1. **Interactive Checkboxes:**
   - Type `- [ ] ` or `- [x] ` or use `check` keyword.
   - Clicking checkbox toggles `- [ ]` ↔ `- [x]`.
   - Supported via `inlineChecklistCheckboxesEnabled`.
2. **List Autocompletion:**
   - Bullet lists (`- `, `* `) and numbered lists (`1. `, `2. `) automatically continue on `Enter`.
   - Pressing `Enter` on an empty list item terminates the list.
   - Supported via `inlineChecklistBulletsEnabled` and `inlineNumberedListsEnabled`.
3. **Smart Antilinks:**
   - Automatically detects URLs in text.
   - Shortens long URLs to clean domain pills (e.g., `github.com/...`) while preserving the underlying full URL.
   - Configurable via `disableLinkShortening` and `disableLinkHighlighting`.
4. **Rounded Code Blocks (`RoundedCodeLayoutManager`):**
   - Fenced code blocks (```` ``` ````) render with rounded corner backgrounds, syntax highlighting (Highlightr themes), and a quick "Copy Code" button.
5. **Vim Mode (`activateVimMode`):**
   - Full modal editing: Normal Mode, Insert Mode, Visual Mode, Visual Line Mode (`enterVisualLineMode`).
   - Motions: `h`, `j`, `k`, `l`, `w`, `b`, `e`, `0`, `$`, `gg`, `G`, `<Up>`, `<Down>`, `<Left>`, `<Right>`.
   - Actions: `d`, `y`, `p`, `u`, `x`, `c`, `r`, `i`, `a`, `o`, `O`.
   - Debounced motion processing (`debounceTimerForVimMotions`).
6. **In-Editor Typography & Markdown Shortcuts:**
   - Markdown Heading Cycle (`#`, `##`, `###`, Body)
   - Formatting: Strikethrough (`~~text~~`), Underline, Italic (`*text*`), Bold (`**text**`).
   - `Toggle Read-Only` mode.
   - Caret nudging: Sub-pixel vertical cursor alignment (`_caretVerticalNudgeRaw`, `_caretVerticalNudgeForListsRaw`).
   - RTL text orientation (`_forceRTLStorage`).
   - Canvas background texture: `dots`, `smallGrid`, `bigGrid` via `GridOverlayView`.

---

## 8. JavaScript Extension System & AI Providers

Antinote ships a complete JS runtime allowing user-authored extensions:

### 8.1 Base Architecture (`antinote-extensions-base-v0.0.1.js`)
- **Constructor `Extension(name, version, endpoints, requiredApiKeys, author, category, dataScope)`:**
  - `dataScope`: Security privacy scope (`"none"`, `"line"`, or `"full"`). Only the permitted scope of note text is sent to the extension.
  - `endpoints`: Array of approved HTTPS origins the extension may request via `callAPI`.
  - `requiredApiKeys`: Array of Keychain key identifiers required (e.g., `apikey_openai`).
- **Constructor `Command(config)`:**
  - Types: `"insert"`, `"replaceLine"`, `"replaceAll"`, `"openURL"`.
  - Parameters: typed `Parameter(type, name, description, defaultValue)` with types `"string"`, `"paragraph"`, `"int"`, `"float"`, `"bool"`.
  - Tutorials: `TutorialCommand(exampleString, description)`.
- **Constructor `Preference(key, title, type, defaultValue, options, description)`:**
  - Types: `"string"`, `"selectOne"`, `"selectMultiple"`, `"bool"`.
- **Return Type `ReturnObject(status, message, data)`:**
  - `status`: `"success"` or `"error"`.
- **Bridge Functions:**
  - `callAPI(apiKeyName, url, method, headers, body)`: Invokes native network client with automatic API key injection from secure storage.
  - `getExtensionPreference(extensionName, key)`: Reads user preference for that extension.
- **Invocation Syntax:**
  - Double-colon syntax: `::commandName(param1, param2)`.

### 8.2 AI Model Catalog (`AIModelCatalog`)
Direct integration with:
- **OpenAI**: `https://api.openai.com/v1/models`, key: `apikey_openai`.
- **Anthropic**: `https://api.anthropic.com/v1/models?limit=1000`, header `anthropic-version`, key: `apikey_anthropic`.
- **Ollama (Local)**: Direct local LLM bridge to `http://localhost:11434/api/tags`.

---

## 9. Window Management, UI Aesthetics & Visual Shaders

1. **Floating Window & System Tray:**
   - Instant summoning via global system hotkey (`HotKeyManager`).
   - System tray / status item icon with quick menu.
   - Always-on-top pin toggle (`restorePinOnOpen`, `_lastPinState`).
   - Window glass translucency with vibrant backdrop blur (`ButtonGlassBackgroundView`, `CustomTitleBarView`).
   - Solid bar mode toggle (`_solidBarsInTransparencyMode`).
2. **Search Engine & Match Types (`FindAndReplaceManager`):**
   - Instant search across active/archived/locked notes (`SearchResultView`).
   - Match criteria: `Contains`, `Starts With`, `Ends With`, `Exact Match`, case-sensitive.
3. **The Void & Metal Shaders:**
   - Deleted notes are soft-deleted into "The Void" with `deletedAt` timestamp.
   - Visual animation shaders:
     - `voidRipple`: Wave distortion when notes drop into The Void.
     - `voidDistortionSlow`: Ambient slow gravitational warping.
     - `voidDistortionBurst`: Energy burst on emptying The Void.
   - Permanent delete confirmation: *"This will permanently delete %lld notes. There are no turtles beyond The Void."*
4. **Safety Backup & Restore Lifecycle (`BackupRestoreSheet`):**
   - Pre-restore snapshotting: Current notes saved as a fresh backup prior to restoring.
   - Sync isolation: Cloud sync detached during backup evaluation.

---

## 10. Third-Party Export Suite

Supported direct export targets:
1. **Plain Text**: `.txt` export (`_exportPlainTextEnabled`).
2. **Markdown**: Standard `.md` format (`_exportMarkdownEnabled`).
3. **Obsidian**: Vault export with path routing (`_exportObsidianEnabled`, `_exportObsidianVault`, `_exportObsidianConvertToMarkdown`).
4. **Bear**: Direct Bear integration (`_exportBearEnabled`, `_exportBearConvertToMarkdown`).
5. **Apple Notes**: Rich text conversion transfer (`_exportAppleNotesEnabled`, `_exportAppleNotesConvertToRichText`).
6. **Notion**: Database insertion (`_exportNotionEnabled`, `_exportNotionSaveInDatabase`, `_exportNotionDatabaseName`).
7. **Custom URL Scheme Template**: Dispatches `myapp://create?title={TITLE}&text={CONTENT}&date={DATE}` (`_exportCustomEnabled`, `_exportCustomURLTemplate`).
8. **Bulk Archive**: Export all notes as a `.zip` archive of `.txt` files.
9. **Options**: `exportGeneralUseFirstLineAsTitle`, `exportGeneralOmitKeyword`.

---

## 11. Complete Preference Matrix

| Setting Key | Type | Default | Description |
|---|---|---|---|
| `globalHotkey` | String | `"Ctrl+Space"` | Global summon shortcut. |
| `restorePinOnOpen` | Boolean | `false` | Whether to persist pinned (always-on-top) state across app restarts. |
| `activateVimMode` | Boolean | `false` | Enable modal Vim keybindings in editor. |
| `inlineChecklistCheckboxesEnabled` | Boolean | `true` | Render interactive checkboxes for `- [ ]`. |
| `inlineChecklistBulletsEnabled` | Boolean | `true` | Auto-format bullet lists on Enter. |
| `inlineNumberedListsEnabled` | Boolean | `true` | Auto-increment numbered lists on Enter. |
| `disableLinkShortening` | Boolean | `false` | Disable compact Antilink shortening. |
| `disableLinkHighlighting` | Boolean | `false` | Disable link visual pill highlights. |
| `disableAllKeywords` | Boolean | `false` | Disable natural-language trigger processing. |
| `angleMeasurementMode` | String | `"degrees"` | Trig mode (`"degrees"` or `"radians"`). |
| `thousandsSeparator` | String | `","` | Number grouping separator. |
| `numberOfDecimalPoints` | Int | `2` | Currency/calculation decimal precision. |
| `timerCountdownPlaySound` | Boolean | `true` | Play audio chime when countdown completes. |
| `timerPomoShowFullScreenAlert` | Boolean | `false` | Display full-screen overlay when Pomodoro ends. |
| `pauseTheTimerIfAppClosed` | Boolean | `true` | Pause active timers when window closes. |
| `solidBarsInTransparencyMode` | Boolean | `false` | Keep bars solid when window is translucent. |
| `forceRTL` | Boolean | `false` | Enable Right-To-Left text layout. |
| `gridOverlayMode` | String | `"none"` | Canvas background overlay (`"none"`, `"dots"`, `"smallGrid"`, `"bigGrid"`). |

---

## 12. Cross-Platform Parity Execution in Plainpad

Plainpad's Tauri + Rust + TypeScript stack must implement each subsystem to achieve 100% equivalence:

1. **Data Layer (`crates/plainpad-core/src/store.rs`):**
   - SQLite schema reflecting the 5 Core Data entities (`notes`, `checklist_items`, `antilinks`, `attachments`, `sync_state`).
2. **Calculation & Triggers (`crates/plainpad-core/src/mathwrap.rs`, `intent.rs`, `currency.rs`):**
   - Full tokenization with fractions, unit conversions across all 6 dimensions, and sentence-level trigger parsing.
3. **Timers (`crates/plainpad-core/src/timers.rs`):**
   - Backend timer tick thread with audio triggers matching the 3 sound assets (`ui/sounds/`).
4. **Extension Engine:**
   - Embedded JavaScript runtime executing `antinote-extensions-base-v0.0.1.js` with secure HTTP fetching bridge and AI model catalog.
5. **UI & Window (`ui/src/`, `src-tauri/src/main.rs`):**
   - Glassmorphic styling, 9 slotted notes bottom bar, Vim mode engine, WebGL Void shaders, canvas grid overlays, and third-party export providers.

---

*Verified against Antinote v2.1.0 binary, Core Data Model 9, Metal shaders, Asset Catalog, and extensions runtime.*
