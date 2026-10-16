# Phase 8 Plan 2: The Void WebGL Distortion Shaders & Permanent Delete Dialog Summary

## Overview

Plan 8.2 implemented visual shader animations matching Antinote's `default.metallib` specifications (`voidRipple`, `voidDistortionSlow`, `voidDistortionBurst`), built The Void modal overlay view listing soft-deleted notes with live restoration and per-note purge actions, and implemented the permanent deletion confirmation dialog bearing Antinote's canonical quote: *"This will permanently delete %lld notes. There are no turtles beyond The Void."*.

## Deliverables & Key Changes

1. **WebGL / Canvas 2D Void Fragment Shader Engine (`ui/src/void_shaders.ts`)**:
   - Implemented `VoidShaderController` managing screen-aligned quad rendering over `<canvas id="void-canvas">`.
   - `voidDistortionSlow`: Ambient slow gravitational vortex with polar coordinate twist `theta += (1.0 / (r + 0.2)) * sin(t)` and cosmic violet/blue accretion disk filaments operating at steady 60fps. Automatically paused when Void view is closed to conserve GPU/CPU cycles.
   - `voidRipple`: Concentric wave distortion propagating outward from click/delete coordinates with sinusoidal ripple decay and wavefront highlight, triggered upon soft-deleting notes into The Void.
   - `voidDistortionBurst`: High-energy shockwave explosion followed by turbulent energy filaments and inward gravitational collapse, triggered on Empty The Void.
   - Full procedural Canvas 2D fallback ensuring visual continuity and rendering stability on systems without WebGL support.

2. **The Void Modal & Note Lifecycle Management (`ui/index.html`, `ui/src/main.ts`, `ui/styles.css`, `ui/style.css`)**:
   - Dedicated Void modal `#void-modal` accessible via `#btn-void` in the bottom bar and `Ctrl+Shift+T` / `Cmd+Shift+T` global keyboard shortcuts.
   - Displays soft-deleted notes fetched via `list_trash`: title, human-friendly relative deletion timestamps (e.g., "Deleted 5m ago"), "Restore" action button, and "Purge" button.
   - Restoring a note invokes `restore_note(id)`, assigning `db_index = MAX(active db_index) + 1` to bring it back to the top of the active stack, refreshes The Void list, and reloads active notes.
   - Per-note permanent deletion (`btn-purge-note`) prompts for confirmation and calls `permanently_delete_note(id)`.

3. **Permanent Purge Confirmation Dialog (`ui/index.html`, `ui/src/main.ts`)**:
   - "Empty The Void" (`#btn-empty-void`) checks trash count; shows subtle toast if already empty.
   - Displays confirmation dialog (`#void-confirm-dialog`) with the exact canonical Antinote quote:
     *"This will permanently delete %lld notes. There are no turtles beyond The Void."* with `%lld` replaced with the count of deleted notes.
   - Confirming triggers the `voidDistortionBurst` shader animation while concurrently executing `purge_void` on the SQLite database, resetting The Void to empty state.

4. **Tauri Backend Integration (`src-tauri/src/main.rs`, `crates/plainpad-core`)**:
   - Added `permanently_delete_note` IPC command to `src-tauri/src/main.rs` invoking `store.permanently_delete_note(&id)`.
   - Added integration test `test_permanently_delete_single_note` in `crates/plainpad-core/tests/note_lifecycle_tests.rs` verifying invariant enforcement (rejects deleting active notes, deletes soft-deleted notes cleanly, returns `NotFound` thereafter).

## Verification Results

- `npm run typecheck`: Passed with 0 errors.
- `npm run build:ui`: Cleanly compiled TypeScript and copied UI assets.
- `cargo check -p plainpad`: Cleanly compiled with 0 errors.
- `cargo test`: All 147 unit, integration, and doc tests passed.
