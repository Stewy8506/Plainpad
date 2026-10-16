---
phase: 8
plan: 1
wave: 1
gap_closure: false
depends_on: []
files_modified:
  - src-tauri/src/main.rs
  - ui/src/main.ts
  - ui/src/settings.ts
  - ui/settings.html
  - ui/styles.css
  - ui/style.css
autonomous: true
must_haves:
  truths:
    - "Main window features translucent glass backdrop blur and configurable solid chrome bars in transparency mode"
    - "Global summon hotkey (default Ctrl+Space) toggles window visibility from anywhere in the OS without losing focus"
    - "App respects restorePinOnOpen setting: if false, window starts unpinned; if true, restores lastPinState"
    - "System tray icon provides quick access menu (Show, Hide, New Note, Slotted notes 1-9 quick jump, Settings, Quit)"
    - "Background canvas grid textures render none, dots, smallGrid, and bigGrid overlays on the note editor"
    - "RTL directionality toggle (forceRTL) mirrors editor text direction and alignment for right-to-left scripts"
  artifacts:
    - src-tauri/src/main.rs
    - ui/src/main.ts
    - ui/src/settings.ts
    - ui/settings.html
    - ui/styles.css
  key_links:
    - "src-tauri/src/main.rs registers globalHotkey from store settings and binds tray quick menu actions"
    - "ui/src/main.ts applies gridOverlayMode, solidBarsInTransparencyMode, and forceRTL CSS classes dynamically"
    - "ui/src/settings.ts exposes controls for globalHotkey, restorePinOnOpen, gridOverlayMode, solidBarsInTransparencyMode, and forceRTL"
---

# Plan 8.1: Window Translucency, Global Summon Hotkey, System Tray Quick Menu, and Canvas Grid Overlays

<objective>
Deliver the core window chrome, desktop integration, and canvas presentation layer matching Antinote specifications:
1. Floating glass window translucency with backdrop blur, pin persistence (`restorePinOnOpen`, `_lastPinState`), dynamic global summon hotkey (`globalHotkey`, default `"Ctrl+Space"`), and expanded system tray quick menu.
2. Background canvas grid textures (`gridOverlayMode`: `"none"`, `"dots"`, `"smallGrid"`, `"bigGrid"`), solid bar mode (`solidBarsInTransparencyMode`), and RTL text layout (`forceRTL`).

Purpose: Plainpad is designed as an instant-access floating scratchpad. Summoning via `Ctrl+Space`, always-on-top pin management, desktop tray integration, and customizable writing textures give users tactile control over their scratchpad environment.
Output: Desktop window management in `src-tauri/src/main.rs`, canvas layout and theme classes in `ui/styles.css` / `ui/src/main.ts`, and preferences controls in `ui/settings.html` / `ui/src/settings.ts`.
</objective>

<context>
Load these files for context:
- .gsd/SPEC.md (§9 Window Management, UI Aesthetics & Visual Shaders; §11 Complete Preference Matrix)
- .gsd/findings.md (§13.2 Canvas Background Overlays; §15 Editor Controls & Typography Tweaks)
- .gsd/REQUIREMENTS.md (REQ-26, REQ-28)
- src-tauri/src/main.rs
- ui/src/main.ts
- ui/src/settings.ts
- ui/settings.html
- ui/styles.css
</context>

<tasks>

<task type="auto">
  <name>Translucent Floating Glass Window, Global Summon Hotkey (Ctrl+Space), Pin Persistence, and Tray Quick Menu</name>
  <files>
    src-tauri/src/main.rs
    ui/src/main.ts
    ui/styles.css
    ui/style.css
  </files>
  <action>
    Implement desktop window management, global hotkey handling, and tray quick menu in `src-tauri/src/main.rs`:
    1. Global Hotkey Registration:
       - Read `globalHotkey` setting from SQLite store (default to `"Ctrl+Space"` or `"CommandOrControl+Space"`).
       - Parse hotkey string into Tauri `Shortcut` (supporting modifiers: `Ctrl`, `Command`, `Cmd`, `Alt`, `Shift`, `Super` + Key).
       - Register shortcut handler: if window is visible and focused, hide window; if window is hidden or unfocused, show, unminimize, and set focus.
       - Expose IPC command `register_global_hotkey(hotkey: String) -> Result<(), String>`: unregisters previous hotkey, registers new hotkey dynamically, and saves setting to store.
    2. Pin Persistence Lifecycle:
       - On window creation in `src-tauri/src/main.rs`: check `restorePinOnOpen` setting (default `false`).
       - If `restorePinOnOpen` is true: restore `_lastPinState` (boolean, fallback to `always_on_top` setting).
       - If `restorePinOnOpen` is false: initialize window unpinned (`always_on_top(false)`).
       - Update `toggle_always_on_top` command: saves both `always_on_top` and `_lastPinState` to store so last pin state is remembered when `restorePinOnOpen` is later enabled.
       - Ensure `btn-always-on-top` in `ui/src/main.ts` syncs active CSS class on startup and click.
    3. System Tray Quick Menu:
       - Expand tray menu with full Antinote quick actions:
         - "Show Plainpad" (shows and focuses main window)
         - "Hide Plainpad" (hides window, pauses timer if configured)
         - Separator
         - "New Note" (emits `menu-new-note` event to main window)
         - Submenu or items for Slotted Notes 1-9: "1: Hermes", "2: Apollo", "3: Ares", "4: Athena", "5: Poseidon", "6: Demeter", "7: Hephaestus", "8: Artemis", "9: Zeus" (emits `menu-select-slot` with index 0..8)
         - Separator
         - "Settings..." (opens settings window)
         - Separator
         - "Quit Plainpad" (exits process)
       - In `ui/src/main.ts`: listen for `menu-new-note` and `menu-select-slot` events to switch notes/slots accordingly.
    4. Glass Translucency Styling:
       - Ensure `backdrop-filter: blur(20px) saturate(180%)` and transparent background tokens apply to `#top-bar`, `#bottom-bar`, and `#editor-container`.
       - Add `.button-glass` styling matching `ButtonGlassBackgroundView` from macOS asset hierarchy.

    AVOID: hardcoding `Alt+A` shortcut in `main.rs`; USE: dynamic shortcut registration initialized from `globalHotkey` setting with `"Ctrl+Space"` default.
    AVOID: ignoring `restorePinOnOpen` preference on launch; USE: check `restorePinOnOpen` before applying pinned state.
  </action>
  <verify>
    npm run typecheck && cargo check -p plainpad
  </verify>
  <done>
    Window starts with correct pin state based on `restorePinOnOpen`, global hotkey toggles visibility, tray menu provides full quick menu with slotted notes, and glass backdrop styling applies cleanly.
  </done>
</task>

<task type="auto">
  <name>Background Canvas Grid Textures (dots, smallGrid, bigGrid), Solid Bar Mode, and RTL Layout Engine</name>
  <files>
    ui/src/main.ts
    ui/src/settings.ts
    ui/settings.html
    ui/styles.css
    ui/style.css
  </files>
  <action>
    Implement canvas grid textures, solid bar mode, and RTL layout across CSS, main editor, and settings:
    1. Canvas Grid Overlays (`GridOverlayView`):
       - Define CSS classes on `#editor-container`:
         - `.grid-none`: no background grid pattern (default).
         - `.grid-dots`: subtle dotted matrix (`radial-gradient(circle, var(--grid-dot-color, rgba(140, 140, 140, 0.22)) 1.2px, transparent 1.2px)`, background-size: `20px 20px`).
         - `.grid-small-grid`: fine engineering paper lines (two linear gradients at 0deg and 90deg, 1px lines at 12px grid pitch, subtle opacity).
         - `.grid-big-grid`: wide lined/quad grid (linear gradients at 32px pitch).
       - Ensure grid pattern is positioned behind text and does not obstruct readability in both light and dark themes.
    2. Solid Bars in Transparency Mode (`solidBarsInTransparencyMode` / `_solidBarsInTransparencyMode`):
       - Setting key: `solidBarsInTransparencyMode` (Boolean, default `false`).
       - Define `.solid-bars` class on `document.body`:
         - When `.solid-bars` is active, `#top-bar` and `#bottom-bar` render with solid opaque background colors (`var(--bg-solid-bar, #1e1e1e)` dark / `#f5f5f7` light) and border separators, while `#editor-container` remains translucent with backdrop blur.
    3. RTL Text Directionality (`forceRTL` / `_forceRTLStorage`):
       - Setting key: `forceRTL` (Boolean, default `false`).
       - When enabled, sets `dir="rtl"` on `#editor` and `#editor-backdrop`, aligning text to the right and reversing cursor/selection layout for Hebrew/Arabic/Persian scripts.
    4. Settings UI Controls:
       - In `ui/settings.html`:
         - In Appearance tab (`panel-appearance`):
           - Grid Overlay Mode select dropdown (`#setting-grid-overlay`) with options: "None", "Dots", "Small Grid", "Big Grid".
           - Solid Bars toggle checkbox (`#setting-solid-bars`) with description: "Keep top and bottom chrome bars solid when window is translucent".
         - In Editor tab (`panel-editor`):
           - RTL Layout toggle checkbox (`#setting-force-rtl`) with description: "Force Right-To-Left (RTL) writing direction".
         - In Shortcuts tab (`panel-shortcuts`):
           - Global Summon Hotkey input field (`#setting-global-hotkey`, default `"Ctrl+Space"`).
         - In General tab (`panel-general`):
           - Restore Pin on Open toggle checkbox (`#setting-restore-pin`).
       - In `ui/src/settings.ts`: wire change listeners to save settings and notify main window via IPC or storage updates.
       - In `ui/src/main.ts`: load settings on startup and apply `.grid-*`, `.solid-bars`, and `dir="rtl"` classes immediately.

    AVOID: applying grid textures across the entire window frame; USE: apply grid pattern specifically to `#editor-container`.
    AVOID: hardcoding dark mode colors for grid dots; USE: CSS variables `--grid-dot-color` and `--grid-line-color` adapting to theme.
  </action>
  <verify>
    npm run build:ui
  </verify>
  <done>
    Grid overlay modes (none, dots, smallGrid, bigGrid) toggle cleanly; solid bars mode keeps chrome bars opaque; forceRTL flips writing direction; all settings persist to SQLite store and sync between windows.
  </done>
</task>

</tasks>

<verification>
After all tasks complete, verify:
- [ ] `cargo check -p plainpad` compiles without warnings or errors
- [ ] `npm run typecheck` passes with zero errors
- [ ] `npm run build:ui` bundles UI assets and copies to `ui/dist`
- [ ] Global hotkey `globalHotkey` defaults to `"Ctrl+Space"` and can be re-registered dynamically
- [ ] Tray quick menu includes Show, Hide, New Note, Slotted notes 1-9, Settings, and Quit
- [ ] Grid overlay mode supports `"none"`, `"dots"`, `"smallGrid"`, and `"bigGrid"`
- [ ] Solid bars mode renders opaque chrome bars while preserving translucent canvas
- [ ] RTL mode sets `dir="rtl"` on editor and backdrop
</verification>

<success_criteria>
- [ ] All tasks verified passing
- [ ] Must-haves confirmed
- [ ] No regressions in test suites
</success_criteria>
