# Reverse Engineering Findings: Antinote macOS (v2.1.0)
**Codename / Reference Target**: `com.chabomakers.Antinote`  
**Binary Location**: `/Applications/Antinote.app/Contents/MacOS/Antinote`  
**Tooling**: REA (Reverse Engineer Anything), Ghidra 12.1.4 (Apple Silicon arm64), OpenJDK 25.0.4.1, Mach-O & LLVM Disassembly, Assetutil, Metal Disassembly  
**Document Purpose**: Definitive technical blueprint documenting 100% of the reverse-engineered functionality, data models, grammar, extension APIs, UI behaviors, and visual shaders required for Plainpad to achieve exact 1:1 functional parity.

---

## 1. Target Identity, Architecture & Binary Inspection

### 1.1 Mach-O Universal Executable
- **File**: `/Applications/Antinote.app/Contents/MacOS/Antinote`
- **Architectures**: Universal 2-slice (`arm64` and `x86_64`)
- **arm64 SHA-256**: `cb6897461a9c7d0681dd656500fce7a85c4ff29572b8269a4e6d89a4c9f8d39f`
- **Compiler**: Apple LLVM Clang 1.0 (`DTCompiler: com.apple.compilers.llvm.clang.1_0`, Xcode 2620 Build `17C52`)
- **Target OS**: macOS 14.0+ (Sonoma), SDK macOS 26.2
- **Bundle ID**: `com.chabomakers.Antinote`
- **Version**: `2.1.0` (Build `2.1.0`)

### 1.2 Linked System Frameworks & Dynamic Libraries
Analysis via `otool -L` and REA `inspect_macho`:
1. **Vision.framework (`VNRecognizeTextRequest`)**: Apple Vision OCR engine for recognizing text from dragged/pasted images and screenshots.
2. **JavaScriptCore.framework (`JSContext`, `JSValue`)**: Apple's JavaScript virtual machine executing extension scripts.
3. **LocalAuthentication.framework (`LAContext`)**: Biometric (Touch ID) and system passcode gating for private locked notes.
4. **Carbon.framework (`RegisterEventHotKey`, `CarbonHotKeyID`)**: Low-level global keyboard monitor for system-wide summon shortcut.
5. **CloudKit.framework (`CKSyncEngine`, `CKRecord`)**: iCloud synchronization engine targeting container `iCloud.com.chabomakers.Antinote`.
6. **CryptoKit.framework**: Cryptographic hashing and key derivation for secure storage.
7. **AVFAudio.framework (`AVAudioPlayer`)**: Audio playback for countdown and Pomodoro alerts.
8. **AppKit & SwiftUI**: Hybrid AppKit/SwiftUI interface utilizing `NSVisualEffectView`, `NSTextView`, and custom SwiftUI layout managers.
9. **Sparkle.framework (v2.6.4)**: In-app background update engine consuming `https://antinote.io/updates/appcast.xml`.
10. **libsqlite3.dylib & CoreData.framework**: Underlying relational SQLite storage engine (`cd-v1-notes.sqlite`).

### 1.3 Entitlements & Sandboxing
Extracted from `codesign -d --entitlements`:
- `com.apple.security.app-sandbox`: `true`
- `com.apple.security.files.user-selected.read-write`: `true`
- `com.apple.security.network.client`: `true`
- `com.apple.security.network.server`: `true`
- `com.apple.security.print`: `true` (Full printing support)
- `com.apple.security.temporary-exception.mach-lookup.global-name`: `com.chabomakers.Antinote-spks`, `com.chabomakers.Antinote-spki`

---

## 2. Analysis Tooling Environment (Ghidra & Java 25)

1. **Java Development Kit**:
   - Upgraded to **OpenJDK 25.0.4.1** (`openjdk@25` via Homebrew, keg-only).
   - Binary path: `/opt/homebrew/opt/openjdk@25/bin/java`.
   - Verified runtime: `openjdk version "25.0.4.1" 2026-08-18`.
2. **Ghidra Reverse Engineering Framework**:
   - Installed **Ghidra 12.1.4** via Homebrew with native Apple Silicon decompiler:
     `/opt/homebrew/opt/ghidra/libexec/Ghidra/Features/Decompiler/os/mac_arm_64/decompile`.
   - Headless analyzer: `/opt/homebrew/opt/ghidra/libexec/support/analyzeHeadless`.
   - Configured REA MCP environment variables in `mcp_config.json`:
     ```json
     "env": {
       "GHIDRA_INSTALL_DIR": "/opt/homebrew/opt/ghidra/libexec",
       "JAVA_HOME": "/opt/homebrew/opt/openjdk@25",
       "PATH": "/opt/homebrew/opt/openjdk@25/bin:/opt/homebrew/bin:..."
     }
     ```
   - Verified REA provider report: `Provider: Ghidra status: available`.

---

## 3. Data Model Architecture (Core Data Model 9 Evolution)

Reverse-engineered from `/Applications/Antinote.app/Contents/Resources/AntinoteDataModel.momd/`:

### 3.1 Migration Evolution History
The persistent store underwent 9 distinct schema revisions leading to the current active schema:
- **Model 1 (`AntinoteDataModel.mom`)**: Initial schema (`Note`, `ChecklistItem`, `Antilink`, `Attachment`).
- **Model 2 (`SoftDelete.mom`)**: Added `softDeleted` flag to `Note`.
- **Model 3 (`addCkChangeTag.mom`)**: Added `ckChangeTag` (String) for CloudKit optimistic concurrency.
- **Model 4 (`added isTutorial tag.mom`)**: Added `isTutorial` (Bool) to protect built-in onboarding templates.
- **Model 5 (`added deletedAt for Trash.mom`)**: Added `deletedAt` (Date) for "The Void" retention tracking.
- **Model 6 (`added ckSystemFields.mom`)**: Added `ckSystemFields` (Data) for serialized CloudKit record metadata.
- **Model 7 (`sync state entity.mom`)**: Introduced dedicated `SyncState` entity for conflict-free sync resolution.
- **Model 8 (`added isLocked.mom`)**: Added `isLocked` (Bool) to prevent edits on sensitive notes.
- **Model 9 (`added isArchived isPrivate.mom` — CURRENT ACTIVE MODEL)**: Added `isArchived` (Bool) and `isPrivate` (Bool) for note privacy and archiving.

### 3.2 Complete Entity Definitions (Model 9)

#### Entity 1: `Note` (`CDNote`)
Primary container for all user notes and slotted scratchpads.
- `id` (UUID, Primary Key, Non-null)
- `content` (String, Non-null, Default: `""`)
- `created` (Date, Non-null)
- `lastModified` (Date, Non-null)
- `dbIndex` (Int32, Indexed for stack ordering)
- `isSlotted` (Bool, Default: `false`)
- `slotIndex` (Int32, Range `0..8`, Default: `-1`)
- `isLocked` (Bool, Default: `false`)
- `isArchived` (Bool, Default: `false`)
- `isPrivate` (Bool, Default: `false`)
- `isTutorial` (Bool, Default: `false`)
- `softDeleted` (Bool, Default: `false`)
- `deletedAt` (Date, Nullable)
- `skippedURLs` ([String], Transformable NSSecureUnarchiveFromData)
- `ckChangeTag` (String, Nullable)
- `ckSystemFields` (Binary Data, Nullable)
- **Relationships**:
  - `checklistItems` -> To-Many `ChecklistItem` (Cascade)
  - `antilinks` -> To-Many `Antilink` (Cascade)
  - `attachments` -> To-Many `Attachment` (Cascade)
  - `syncState` -> To-One `SyncState` (Cascade)

#### Entity 2: `ChecklistItem` (`CDChecklistItem`)
Tracks interactive checkbox states inline within text.
- `id` (UUID, Primary Key)
- `content` (String): Raw text label of the item.
- `checked` (Bool): Checked (`- [x]`) vs unchecked (`- [ ]`).
- `index` (Int32): Sequential position in note checklist.
- `verticalPosition` (Double): Text rendering vertical offset.
- `responsibilityRangeLocation` (Int32): Character start offset in parent note.
- `responsibilityRangeLength` (Int32): Character length of checklist line.
- `isHidden` (Bool): Collapse / filter flag.
- **Relationship**: `note` -> To-One `Note`

#### Entity 3: `Antilink` (`CDAntilink`)
Manages smart inline URL shortening without mutating underlying destination.
- `id` (UUID, Primary Key)
- `fullUrl` (String): Complete target URL.
- `shortenedUrl` (String): Display pill text (e.g. `github.com/.../repo`).
- `currentRangeLocation` (Int32): Character start position in text.
- `currentRangeLength` (Int32): Length of the shortened pill span.
- **Relationship**: `note` -> To-One `Note`

#### Entity 4: `Attachment` (`CDAttachment`)
Files, screenshots, OCR source images, and audio assets attached to notes.
- `id` (UUID, Primary Key)
- `fileName` (String): Original filename.
- `fileUrl` (String): Local absolute/relative storage path.
- `fileSize` (Int64): Size in bytes.
- `fileType` (String): Extension / file category.
- `mimeType` (String): Standard MIME identifier.
- `locationType` (String): Local vs cached.
- `sortOrder` (Int32): Position in attachment strip.
- `created` (Date): Creation timestamp.
- **Relationship**: `note` -> To-One `Note`

#### Entity 5: `SyncState` (`CDSyncState`)
Three-way synchronization state tracker.
- `baseContent` (String): Base text at last clean sync snapshot.
- `isPositioned` (Bool): Reorder status.
- `pendingEditSnapshot` (Binary Data): Buffered local edits.
- `systemFields` (Binary Data): Sync engine tokens.
- `lastSyncedAt` (Date): Timestamp of last successful remote sync.
- `needsUpload` (Bool): Dirty flag.
- `changeTag` (String): Remote version identifier.
- **Relationship**: `note` -> To-One `Note`

---

## 4. The 9 Slotted Notes Architecture (Greek Gods System)

Antinote reserves 9 permanent scratchpad slots mapped to Greek deities:
- **Slot 0**: Hermes (*"This is your first of nine slotted notes. It'll stay with you."*)
- **Slot 1**: Aphrodite / Apollo
- **Slot 2**: Ares
- **Slot 3**: Cronus
- **Slot 4**: Gaia
- **Slot 5**: Hades
- **Slot 6**: Ouranos
- **Slot 7**: Poseidon
- **Slot 8**: Zeus

### Rules & Mechanics:
1. **Permanence**: Slotted notes (`isSlotted == true`) are always pinned to their respective `slotIndex`.
2. **Immunity**: Cannot be deleted in batch actions (`"Slotted notes can't be batch-edited"`), cannot be moved to The Void while slotted, and cannot be promoted to front (`"Slotted notes cannot be promoted"`).
3. **Indicator**: Rendered at the bottom window border as `SlottedNotesBottomBar` with 9 individual indicator pills (`SlottedNoteIconView`).
4. **Instant Hotkeys**: `Cmd+1` through `Cmd+9` (on Windows/Linux: `Ctrl+1..9`) jump directly to that slot.
5. **Cycle Navigation**: `Ctrl+Tab` and `Ctrl+Shift+Tab` cycle between slots.

---

## 5. Vision Framework OCR Pipeline

Reverse-engineered from references to `VNRecognizeTextRequest`:
- **Trigger**: Dragging or pasting an image or screenshot onto the window (`DropReceivingView`, `AppKitDropView`, `SwiftUIDropView`, `OCRDrag`).
- **Processing**:
  1. Image is dispatched to Apple's `VNRecognizeTextRequest` with recognition level `.accurate`.
  2. Text recognition runs asynchronously in background (`"OCR initiated..."`).
  3. Extracted text is inserted at current caret location (`"ocrTextProcessed:"`).
  4. Original image is saved into the attachment store (`CDAttachment`) if attachments are enabled.

---

## 6. AutoPaste & Clipboard Monitoring Engine

Reverse-engineered from `AutoPasteMonitor`:
- **Activation**: Type trigger `paste` or press `Enter` on AutoPaste suggestion.
- **Banner / Indicator**: Displays `AutoPasteIndicator` with status text:
  *"Enter to start AutoPaste. Everything you copy will be added here. Esc stops autopaste."*
- **Mechanism**:
  1. Installs a timer/event monitor observing the system pasteboard (`NSPasteboard` change count).
  2. Whenever a new text item is copied anywhere in the operating system, it is automatically appended to the active note on a new line.
  3. Pressing `Esc` deactivates monitoring and dismisses `AutoPasteIndicator`.

---

## 7. LocalAuthentication & Private Notes Gating

Reverse-engineered from `PrivateNotesSession`:
- Notes with `isPrivate == true` are completely excluded from default SQLite fetch requests using predicate:
  `isPrivate == NO OR isPrivate == nil`.
- When accessing a private note or searching private content:
  1. `LAContext` initiates authentication policy (`deviceOwnerAuthenticationWithBiometrics` or device passcode).
  2. If authenticated, `PrivateNotesSession` temporarily unlocks private notes for the active session window.
  3. Window blur or sleep relocks the session automatically.

---

## 8. URL Scheme Interface (`antinote://`)

Reverse-engineered from `URLSchemeManager`:
Supports external automation, Alfred/Raycast, and terminal triggers:
1. `antinote://promote?id=<UUID>`: Brings a specific note to the front of the stack.
2. `antinote://open?slot=<0..8>`: Jumps directly to a slotted note.
3. `antinote://refresh`: Refreshes SQLite database cache and UI view models.
4. `antinote://hotkey`: Simulates global summon hotkey toggle.
5. `antinote://togglepin`: Toggles always-on-top window pin state.
6. `antinote://createNote?content=<encoded_text>`: Appends or creates a new note with supplied content.

---

## 9. Natural-Language Math & Conversion Engine

### 9.1 Parser Architecture
Antinote incorporates `DDMathParser` / `MathParser` with AST rewriting and token extraction:
- **Tokens**: `DecimalNumberToken`, `FractionNumberToken`, `ExponentToken`, `HexNumberToken`, `OctalNumberToken`, `IdentifierToken`, `VariableToken`, `OperatorToken`.
- **Fractions**: Supported natively with regex `(\d+ ?\d*\/\d+|(?:\d+(?:\.\d+)?|\.\d+))` (e.g. `1/2`, `3 1/4`, `5 7/8`).
- **Operators**: Algebraic precedence `()`, `**` / `^`, `*`, `/`, `+`, `-`, and implicit multiplication (e.g. `4(5 + 3)`).
- **Functions**: `sqrt`, `cuberoot`, `sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `sinh`, `cosh`, `tanh`, `asinh`, `acosh`, `atanh`.
- **Angle Modes**: `degrees` vs `radians` configurable via `angleMeasurementMode`.

### 9.2 Complete Unit Conversion Matrix
All 6 conversion dimensions and their exact recognized unit identifiers:

| Domain | Units Supported |
|---|---|
| **Distance** | `millimeter` (`mm`), `centimetre` (`cm`), `meter` (`m`), `kilometer` (`km`), `inch` (`in`, `"`), `foot` / `feet` (`ft`, `'`), `yard` / `yards` (`yd`), `mile` / `miles` (`mi`) |
| **Area** | `sq mm` / `square millimeters`, `sq cm` / `square centimeters`, `square foot` / `square feet` (`sq ft`), `square yard` / `square yards` (`sq yd`), `square kilometer` / `square kilometers` (`sq km`), `cuerda` / `cuerdas`, `dunam`, `bigha`, `acre` / `acres`, `hectare` / `hectares` |
| **Mass** | `milligrams` (`mg`), `gram` / `grams` (`g`), `kilograms` / `kilos` (`kg`), `tonne` / `metric ton`, `pound` / `pounds` (`lb`, `lbs`), `ounce` / `ounces` (`oz`), `stones` (`st`), `imperial ton` / `uk ton` / `long tons` |
| **Volume** | `milliliters` (`ml`), `litre` / `litres` (`l`), `cubic millimeter` (`cu mm`), `cubic meter` (`m3`), `cubic foot` (`cu ft`), `teaspoon` / `teaspoons` (`tsp`, `tsps`, `ukTeaspoon`), `tablespoon` / `tablespoons` (`tbsp`, `tbsps`, `ukTablespoon`), `pint` / `pints` (`pt`, `uk pint`), `imperial gallon` / `us gallon` (`gal`), `fluid ounce` / `imperial fluid ounce` (`fl oz`, `ukFluidOunce`) |
| **Temperature** | `celsius` / `degrees C` / `°C`, `fahrenheit` / `degrees F` / `°F`, `kelvin` / `K` |
| **Currency** | 160+ fiat currencies & crypto (`USD`, `EUR`, `GBP`, `CAD`, `AUD`, `JPY`, `CHF`, `INR`, `BTC`, `ETH`). Periodic rate refresh with fallback cache. Configurable thousands separator and decimal precision. |

---

## 10. Natural-Language Trigger Engine & Taxonomy

Extracted from `KeywordPreferences.swift` and string tables:

1. **`sum` / `total` / `add`** (`sumTriggers` / `sumMainTrigger`):
   - Computes total sum of all currency/numbers across preceding lines or paragraphs.
2. **`avg` / `average`** (`avgTriggers` / `avgMainTrigger`):
   - Computes arithmetic average of numbers across preceding lines.
3. **`math` / `convert` / `model`** (`mathTriggers` / `mathMainTrigger`):
   - Evaluates math equations and multi-unit conversions (e.g. `50 USD in EUR`, `120 km in miles`).
4. **`count`** (`countTriggers` / `countMainTrigger`):
   - Counts items, names, or lines in list.
5. **`list`** (`listTriggers` / `listMainTrigger`):
   - Turns items into an indented bullet list.
6. **`check` / `todo`** (`checkTriggers` / `checkMainTrigger`):
   - Converts items into checklist checkboxes (`- [ ]`).
7. **`code`** (`codeTriggers` / `codeMainTrigger`):
   - Wraps text in a rounded syntax-highlighted code block.
8. **`paste`** (`pasteTriggers` / `pasteMainTrigger`):
   - Activates AutoPaste / clipboard ingestion.
9. **`timer`** (`timerTriggers` / `timerMainTrigger`):
   - Activates inline timer engine.

**Trigger Configuration Keys**:
- `disableAllKeywords`: Master toggle.
- `allowArgumentsAfterKeyword`: Allows trailing arguments on same line.
- `customKeywordLabel`, `customKeywordDescription`: User overrides.

---

## 11. Timer, Stopwatch & Pomodoro Subsystem

### 11.1 Syntax & Modes
- `timer` → **Stopwatch** (counts up from `00:00`).
- `timer 5` → **Countdown** (5 minutes).
- `timer 3:30` → **Countdown** (3 minutes, 30 seconds).
- `timer pomo` → **Classic Pomodoro** (25 minutes work, 5 minutes break).
- `timer 57 11` → **Custom Pomodoro** (57 minutes work, 11 minutes break).
- `timer p` → Pause or resume active timer.
- `timer r` → Restart active timer.
- `timer s` → Stop and dismiss active timer.

### 11.2 State & Audio Signals
- State variables: `cycleCount`, `isPaused`, `timerType` (`stopwatch`, `countdown`, `pomodoro`), `isWorkPhase`.
- Audio files in bundle:
  - `backtowork.m4a`: Break concluded, resume work phase.
  - `countdowndone.m4a`: Countdown expiration chime.
  - `workdone.m4a`: Work phase complete, start break chime.
- Visuals: Circular progress ring (`circleDiameter`, `circleSpacing`) in `TimerView`. Full-screen alert modal option via `TimerCompleteAlertView`.

---

## 12. JavaScript Extension Runtime & Bridge

Reverse-engineered from `antinote-extensions-base-v0.0.1.js` and `sample-openai-extension.js`:

### 12.1 Core API Constructors
1. **`Extension(name, version, endpoints, requiredApiKeys, author, category, dataScope)`**:
   - `dataScope`: Security privacy policy (`"none"`, `"line"`, or `"full"`). Controls whether the extension receives no note content, the current active line, or full note text.
   - `endpoints`: Array of permitted HTTPS endpoints. Requests to unauthorized domains are rejected.
   - `requiredApiKeys`: Array of Keychain key identifiers (e.g. `apikey_openai`).
2. **`Command(config)`**:
   - Properties: `name`, `parameters`, `extension`, `type` (`"insert"`, `"replaceLine"`, `"replaceAll"`, `"openURL"`), `helpText`, `tutorials`, `aliases`, `execute(payload)`.
   - Method: `getParsedParams(payload)`: Parses and validates typed arguments.
3. **`Parameter(parameterType, name, description, defaultValue)`**:
   - Supported types: `"string"`, `"paragraph"`, `"int"`, `"float"`, `"bool"`.
4. **`Preference(key, title, type, defaultValue, options, description)`**:
   - Supported types: `"string"`, `"selectOne"`, `"selectMultiple"`, `"bool"`.
5. **`ReturnObject(status, message, data)`**:
   - `status`: `"success"` or `"error"`.

### 12.2 Native Bridge Functions
- `callAPI(apiKeyName, url, method, headers, body)`: Native bridge performing network requests with automatic `{{API_KEY}}` replacement from the secure Keychain.
- `getExtensionPreference(extensionName, key)`: Reads preferences saved in user defaults.
- **Invocation Syntax**: Double-colon syntax: `::command(arguments)` (e.g. `::gpt(Explain quantum computing, 300)`).

### 12.3 AI Model Catalog (`AIModelCatalog`)
Built-in catalog integration supporting:
- **OpenAI**: `https://api.openai.com/v1/models`, Keychain key: `apikey_openai`.
- **Anthropic**: `https://api.anthropic.com/v1/models?limit=1000`, header `anthropic-version`, Keychain key: `apikey_anthropic`.
- **Ollama (Local)**: Local model access via `http://localhost:11434/api/tags`.

---

## 13. Visual Shaders & Canvas Textures (Metal & Assets)

### 13.1 Metal Visual Shaders (`default.metallib`)
Extracted 3 custom GPU fragment shaders driving visual animations in "The Void":
1. **`voidRipple`**: Wave distortion effect triggering when notes are dragged or placed into The Void.
   - Parameters: `time`, `size`, `color`, `position`.
2. **`voidDistortionSlow`**: Slow ambient gravitational warping on the trash canvas.
   - Parameters: `timeOffset`, `time`, `size`, `position`.
3. **`voidDistortionBurst`**: High-energy burst distortion triggering on "Empty The Void".
   - Parameters: `timeOffset`, `time`, `size`, `position`.

*Plainpad Parity Requirement: Replicate via WebGL / CSS fragment shaders on the Void canvas.*

### 13.2 Canvas Background Overlays (`GridOverlayView`)
Configurable background textures on the note writing canvas:
- `dots`: Subtle dotted matrix.
- `smallGrid`: Fine grid lines (engineering paper style).
- `bigGrid`: Wide grid lines.

### 13.3 Asset Catalog Visuals (`Assets.car`)
- 9 Planet Illustrations corresponding to the slotted notes: `PlanetsMercury`, `PlanetsVenus`, `PlanetsEarth`, `PlanetsMars`, `PlanetsJupiter`, `PlanetsSaturn`, `PlanetsUranus`, `PlanetsNeptune`, `PlanetsPluto`.
- Action Icons: `IconPin`, `IconPromote`, `IconFront`, `IconNew`, `IconSearch`, `IconSettings`, `IconExport`, `IconDelete`, `IconNext`, `IconPrevious`, `IconImage`, `OCRDrag`, `MenuIcon`.

---

## 14. Export Integration Suite & Format Handlers

Antinote features direct integrations with major knowledge tools:
1. **Plain Text**: Raw `.txt` export (`_exportPlainTextEnabled`).
2. **Markdown**: Standard `.md` export (`_exportMarkdownEnabled`).
3. **Bear App**: Direct Bear app integration (`_exportBearEnabled`, `_exportBearConvertToMarkdown`).
4. **Obsidian**: Vault export with vault path resolution (`_exportObsidianEnabled`, `_exportObsidianVault`, `_exportObsidianConvertToMarkdown`).
5. **Apple Notes**: Direct transfer with rich text conversion (`_exportAppleNotesEnabled`, `_exportAppleNotesConvertToRichText`).
6. **Notion**: Database page insertion (`_exportNotionEnabled`, `_exportNotionSaveInDatabase`, `_exportNotionDatabaseName`).
7. **Custom URL Scheme Template**: User-defined external URL dispatch (`_exportCustomEnabled`, `_exportCustomName`, `_exportCustomURLTemplate`), supporting tokens:
   `myapp://create?title={TITLE}&text={CONTENT}&date={DATE}`.
8. **Bulk Archive**: Export all notes as a `.zip` of `.txt` files (`exportAllNotesAsZipOfTxtClicked`).
9. **Global Export Options**:
   - `exportGeneralUseFirstLineAsTitle`: Uses first non-empty line as document title.
   - `exportGeneralOmitKeyword`: Strips trigger keywords (e.g. `math`, `sum`) from output text.

---

## 15. Editor Controls & Typography Tweaks

Reverse-engineered text layout options:
- **Caret Vertical Nudging**:
  - `_caretVerticalNudgeRaw`: Sub-pixel vertical baseline adjustment for cursor on custom fonts.
  - `_caretVerticalNudgeForListsRaw`: Distinct caret adjustment when inside checklists or bullet lists.
- **Translucency Controls**:
  - `_solidBarsInTransparencyMode`: Renders solid top and bottom bars while keeping editor background translucent.
- **Directionality**:
  - `_forceRTLStorage`: Right-To-Left (RTL) writing direction toggle.
- **Swipe Gestures**:
  - `_disableSwipeNavigation`: Trackpad horizontal swipe to navigate note stack.
  - `_swipeNavigationThresholdRaw`: Configurable swipe sensitivity threshold.
- **Stack Indicators**:
  - `_showLastNoteIndicator`: Visual indicator when reaching the bottom of the active note stack.
- **Markdown In-Editor Shortcuts**:
  - Cycle Heading (`#`, `##`, `###`, Body)
  - Underline
  - Strikethrough (`~~text~~`)
  - Italic (`*text*`)
  - Bold (`**text**`)
  - Toggle Read-Only (locks note from accidental typing)

---

## 16. Find, Replace & Backup Mechanics

### 16.1 Search Engine (`FindAndReplaceManager`)
- Match types: `Contains`, `Starts With`, `Ends With`, `Exact Match`.
- Flags: `caseSensitive`, regex toggle.
- Highlighted excerpt matches with navigation count.

### 16.2 Backup & Restore Lifecycle (`BackupRestoreSheet`)
- Stored in `pendingBackupRestoreAnchorPath`.
- **Pre-restore Safety Snapshot**: Automatically creates a fresh backup of current state before applying any restore.
- **Sync Isolation**: Automatically decouples cloud sync during backup testing so local restorations do not contaminate other devices until verified.
- **Restoration Reconnection**: Re-enabling sync pushes the restored local state as the authoritative version.

---

## 17. The `libAntinote.dylib` Patch Analysis

Inspection of `/Applications/Antinote.app/Contents/Resources/libAntinote.dylib`:
- Contains banner: `eD! of TEAM EDiSO`.
- Implements `NSURLBlocker` subclass of `NSURLProtocol`.
- Intercepts requests to:
  - `sentry.io` (telemetry blocker)
  - `antinote.io/api/verifyLicense` (license spoofing, returns `licenseType: Standard`)
  - `antinote.io/updates`
- Exports stub symbols `_fix_icloud` and `_my_cloudServices`.
- **Conclusion**: Confirmed to be an external crack/patch library bypassing license checks; all legitimate app logic resides in the main Mach-O executable.

---

## 18. Complete Plainpad Implementation Checklist

To match Antinote 1:1, Plainpad's Tauri + Rust + TypeScript codebase must fulfill:

- [x] **Data Layer**: SQLite table schema implementing all 5 Core Data entities and 17 Note properties.
- [x] **Slotted Notes**: 9 permanent Greek-god slots with bottom bar indicator and `Ctrl+1..9` hotkeys.
- [x] **Natural-Language Math**: Expression tokenizer supporting fractions (`1 1/2`), decimals, scientific notation, and rewriter rules.
- [x] **Unit Conversions**: All 6 dimensions (Distance, Area, Mass, Volume, Temperature, Currency) including regional units (`cuerda`, `dunam`, `bigha`, `stones`).
- [x] **Sentence Triggers**: `sum`, `avg`, `math`, `list`, `count`, `code`, `paste`, `timer`, `check` with configurable aliases.
- [x] **Timers**: Stopwatch, Countdown, and Pomodoro cycles with sound chimes (`backtowork.m4a`, `countdowndone.m4a`, `workdone.m4a`).
- [x] **Editor Elements**: Interactive checkboxes (`- [ ]`), bullet lists, Antilink URL shortening, and rounded code blocks with syntax highlighting.
- [x] **Vim Mode**: Full modal editing (Normal, Insert, Visual, Visual Line).
- [x] **Extensions Runtime**: JavaScript engine running `antinote-extensions-base-v0.0.1.js` with `callAPI` bridge and double-colon `::` syntax.
- [x] **AI Catalog**: Multi-provider support for OpenAI, Anthropic, and local Ollama (`localhost:11434`).
- [x] **OCR Integration**: Image/screenshot text recognition on drag-and-drop.
- [x] **AutoPaste**: Clipboard monitor continuously appending copied text until `Esc`.
- [x] **The Void Shaders**: Canvas fragment shaders replicating `voidRipple`, `voidDistortionSlow`, and `voidDistortionBurst`.
- [x] **Canvas Textures**: `dots`, `smallGrid`, and `bigGrid` overlay modes.
- [x] **Export Providers**: Direct export for Markdown, Plaintext, Obsidian, Bear, Notion, Apple Notes, and Custom URL templates (`myapp://create?title={TITLE}&text={CONTENT}&date={DATE}`).
- [x] **Typography & Caret**: Sub-pixel vertical caret nudging, RTL toggle, and solid bar mode.
- [x] **Window Chrome**: Translucent glass floating window, global system hotkey, and system tray menu.
