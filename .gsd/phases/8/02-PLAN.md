---
phase: 8
plan: 2
wave: 2
gap_closure: false
depends_on:
  - "8.1"
files_modified:
  - ui/src/void_shaders.ts
  - ui/src/main.ts
  - ui/index.html
  - ui/styles.css
  - ui/style.css
  - src-tauri/src/main.rs
autonomous: true
must_haves:
  truths:
    - "WebGL fragment shaders reproduce the 3 Metal animations from Antinote's default.metallib: voidRipple, voidDistortionSlow, and voidDistortionBurst"
    - "voidDistortionSlow provides ambient gravitational cosmic warping on The Void / Trash canvas at steady 60fps"
    - "voidRipple renders an expanding concentric wave distortion whenever a note is soft-deleted into The Void"
    - "voidDistortionBurst renders a high-energy outward shockwave followed by collapse on Empty The Void"
    - "The Void view lists all soft-deleted notes with deleted_at timestamps, restoration buttons, and permanent delete controls"
    - "Empty The Void dialog confirms with exact Antinote quote: 'This will permanently delete %lld notes. There are no turtles beyond The Void.'"
    - "Procedural Canvas 2D fallback guarantees rendering stability if WebGL is unavailable or disabled"
  artifacts:
    - ui/src/void_shaders.ts
    - ui/src/main.ts
    - ui/index.html
    - ui/styles.css
  key_links:
    - "ui/src/void_shaders.ts compiles fragment shaders and binds to #void-canvas in ui/index.html"
    - "ui/src/main.ts triggers voidRipple on note deletion and voidDistortionBurst on void purge"
    - "Empty The Void confirmation dialog calls purge_void command in src-tauri/src/main.rs"
---

# Plan 8.2: The Void WebGL Distortion Shaders and Permanent Delete Confirmation Dialog

<objective>
Implement the visual shader animations and trash management lifecycle for "The Void":
1. WebGL fragment shader engine replicating the 3 Metal shaders extracted from `default.metallib`: `voidRipple` (deletion wave), `voidDistortionSlow` (ambient gravitational vortex), and `voidDistortionBurst` (purge energy shockwave), with procedural Canvas 2D fallback.
2. The Void modal/overlay view listing soft-deleted notes with restore actions and the permanent deletion confirmation dialog containing the canonical Antinote quote: *"This will permanently delete %lld notes. There are no turtles beyond The Void."*.

Purpose: In Antinote, deleted notes are not dropped into a generic trash bin—they descend into "The Void", accompanied by visual gravitational wave distortions and high-energy burst shaders. Delivering these shaders and the permanent delete quote achieves 1:1 aesthetic parity.
Output: `ui/src/void_shaders.ts`, Void UI overlay in `ui/index.html`, and deletion event wiring in `ui/src/main.ts`.
</objective>

<context>
Load these files for context:
- .gsd/SPEC.md (§9.3 The Void & Metal Shaders)
- .gsd/findings.md (§13.1 Metal Visual Shaders; §16.1 Search Engine & Trash)
- .gsd/REQUIREMENTS.md (REQ-27)
- crates/plainpad-core/src/store/void.rs
- src-tauri/src/main.rs
- ui/src/main.ts
- ui/index.html
- ui/styles.css
</context>

<tasks>

<task type="auto">
  <name>WebGL Fragment Shader Engine for The Void (voidRipple, voidDistortionSlow, voidDistortionBurst)</name>
  <files>
    ui/src/void_shaders.ts
    ui/index.html
    ui/styles.css
  </files>
  <action>
    Create `ui/src/void_shaders.ts` implementing the GPU fragment shaders matching Antinote's `default.metallib` specifications:
    1. WebGL Context & Pipeline Setup:
       - Mount full-bleed `<canvas id="void-canvas" class="void-canvas hidden"></canvas>` in `ui/index.html`.
       - Initialize WebGL context (`webgl2` or `webgl`) with alpha transparency support.
       - Implement screen-aligned full quad vertex shader (`attribute vec2 a_position; void main() { gl_Position = vec4(a_position, 0.0, 1.0); }`).
       - Provide procedural Canvas 2D fallback in case WebGL context initialization fails.
    2. Shader Implementations:
       - `voidRipple`:
         - Concentric wave distortion propagating outward from note drop/delete coordinates.
         - Uniforms: `u_time` (animation progress 0.0 -> 1.2s), `u_resolution` (vec2), `u_center` (vec2, default center or click coordinates), `u_color` (vec4 deep cosmic violet/blue).
         - Fragment logic: compute distance from `u_center`, modulate sinusoidal wave `sin(dist * freq - u_time * speed) * exp(-dist * decay) * (1.0 - u_time / duration)`.
       - `voidDistortionSlow`:
         - Ambient slow gravitational warping on the void canvas.
         - Uniforms: `u_time` (continuous elapsed time), `u_time_offset` (float), `u_resolution` (vec2), `u_center` (vec2).
         - Fragment logic: polar coordinate transformation `(r, theta)` around center with angular twist `theta += (1.0 / (r + 0.2)) * sin(u_time * 0.5)` creating an organic swirling black hole vortex with subtle luminous filaments.
       - `voidDistortionBurst`:
         - High-energy explosion triggering on "Empty The Void".
         - Uniforms: `u_time` (burst progress 0.0 -> 1.5s), `u_time_offset` (float), `u_resolution` (vec2), `u_center` (vec2).
         - Fragment logic: rapid expanding shockwave ring followed by chaotic noise/energy collapse inward, fading to clear alpha at completion.
    3. Exported Controller API:
       - `class VoidShaderController`:
         - `attach(canvas: HTMLCanvasElement): void`
         - `startAmbient(): void` (starts requestAnimationFrame loop for `voidDistortionSlow`)
         - `stopAmbient(): void` (pauses animation loop to conserve GPU/CPU when Void view is hidden)
         - `triggerRipple(originX?: number, originY?: number, onComplete?: () => void): void` (runs temporary `voidRipple` overlay, ~1.2s duration)
         - `triggerBurst(onComplete?: () => void): void` (runs high-energy `voidDistortionBurst` overlay, ~1.5s duration)
         - `resize(): void` (syncs canvas pixel buffer with client rect)

    AVOID: keeping the WebGL requestAnimationFrame loop running when Void view is closed; USE: pause rendering when inactive to avoid wasting battery and GPU cycles.
    AVOID: crashing on systems without WebGL support; USE: try/catch initialization with Canvas 2D procedural fallback.
  </action>
  <verify>
    npm run build:ui
  </verify>
  <done>
    `ui/src/void_shaders.ts` compiles cleanly, mounts `#void-canvas`, runs `voidDistortionSlow` at 60fps when active, and triggers `voidRipple` and `voidDistortionBurst` with proper cleanup.
  </done>
</task>

<task type="auto">
  <name>The Void UI View, Note Soft-Delete Ripple Trigger, and Permanent Purge Confirmation Dialog</name>
  <files>
    ui/src/main.ts
    ui/index.html
    ui/styles.css
    ui/style.css
    src-tauri/src/main.rs
  </files>
  <action>
    Wire The Void lifecycle, note deletion animations, and permanent purge dialog across UI and Rust backend:
    1. Note Deletion with Ripple Animation:
       - In `ui/src/main.ts`, update `deleteCurrentNote`:
         - Before removing the note from the active stack, call `triggerRipple(clickX, clickY)`.
         - Invoke `delete_note(currentNoteId)` (which calls `soft_delete_note` in SQLite store, setting `soft_deleted = 1` and `deleted_at = now_iso()`).
         - Advance active editor to adjacent note.
    2. The Void View Modal (`#void-modal` / `#trash-modal`):
       - Provide dedicated Void interface in `ui/index.html` accessible via Trash/Void button or hotkey:
         - Displays header "The Void" with subtitle "Soft-deleted notes resting in gravitational stasis".
         - Displays list of deleted notes fetched via `list_trash`: title, relative date/timestamp, "Restore" button (`btn-restore-note`), and "Purge" button.
         - Clicking "Restore" invokes `restore_note(id)` (which sets `soft_deleted = 0`, `deleted_at = NULL`), re-inserts note at top of active stack, and refreshes list.
         - Starts `startAmbient()` on open and `stopAmbient()` on close.
    3. Permanent Purge Confirmation Dialog:
       - Button "Empty The Void" (`#btn-empty-void`):
         - Fetches count of deleted notes via `list_trash`.
         - If count == 0: show subtle "The Void is already empty" toast/feedback.
         - If count > 0: open confirmation dialog (`#void-confirm-dialog`):
           - Headline: "Empty The Void?"
           - Message text: *"This will permanently delete %lld notes. There are no turtles beyond The Void."* where `%lld` is substituted with the exact note count.
           - Cancel button (`#btn-cancel-empty-void`): dismisses modal without deleting.
           - Confirm button (`#btn-confirm-empty-void`):
             - Triggers `triggerBurst()` high-energy shader animation.
             - Calls `purge_void` IPC command in `src-tauri/src/main.rs`.
             - Refreshes Void list to empty state.
    4. Individual Note Permanent Deletion:
       - Add IPC command `permanently_delete_note(id: String)` in `src-tauri/src/main.rs` calling `store.permanently_delete_note(&id)`.
       - Wire per-note delete button in Void list with brief confirmation prompt.

    AVOID: generic confirmation text like "Are you sure you want to delete these notes?"; USE: exact canonical Antinote quote *"This will permanently delete %lld notes. There are no turtles beyond The Void."*.
    AVOID: blocking UI during shader burst animation; USE: execute backend purge concurrently while shader burst plays.
  </action>
  <verify>
    npm run build:ui && cargo check -p plainpad
  </verify>
  <done>
    Deleting a note triggers the `voidRipple` wave distortion; The Void view displays soft-deleted notes with ambient cosmic warping; Empty The Void prompts with the canonical confirmation quote; confirming triggers `voidDistortionBurst` and permanently purges notes from SQLite store.
  </done>
</task>

</tasks>

<verification>
After all tasks complete, verify:
- [ ] `npm run typecheck` passes with zero errors
- [ ] `npm run build:ui` bundles UI assets with `void_shaders.ts`
- [ ] `cargo check -p plainpad` compiles without errors
- [ ] `#void-canvas` initializes WebGL context or falls back gracefully to Canvas 2D
- [ ] `voidDistortionSlow` renders ambient cosmic warping when Void view is open and stops when closed
- [ ] `voidRipple` animates concentric wave expansion on note deletion
- [ ] `voidDistortionBurst` animates shockwave burst on Empty The Void
- [ ] Empty The Void modal displays exact text: `"This will permanently delete %lld notes. There are no turtles beyond The Void."`
- [ ] `purge_void` successfully deletes all soft-deleted notes from SQLite database
</verification>

<success_criteria>
- [ ] All tasks verified passing
- [ ] Must-haves confirmed
- [ ] No regressions in test suites
</success_criteria>
