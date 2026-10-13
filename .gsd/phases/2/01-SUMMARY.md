# Plan 2.1 Summary: Slotted Notes Backend Store Logic, Initialization, and Slot Invariance Enforcement

## Overview

Plan 2.1 implemented the permanent 9-slot scratchpad architecture mapped to Greek deities (Hermes through Zeus) in `plainpad-core`, wired idempotent initialization into `SqliteStore`, enforced slot invariance rules (immunity to deletion into The Void, stack displacement immunity, and preserving empty slotted content), and exposed IPC commands in the Tauri backend with comprehensive integration tests.

## Key Changes

1. **Greek Deity Constants and Default Content** (`crates/plainpad-core/src/store/notes.rs`, `store/mod.rs`):
   - Defined `GREEK_DEITIES`: Hermes (0), Apollo (1), Ares (2), Cronus (3), Gaia (4), Hades (5), Ouranos (6), Poseidon (7), Zeus (8).
   - Set `HERMES_DEFAULT_CONTENT` to `"This is your first of nine slotted notes. It'll stay with you."`.
   - Added `default_content_for_slot(slot_index)` and `deity_for_slot(slot_index)`.

2. **Idempotent 9-Slot Initialization & Retrieval** (`crates/plainpad-core/src/store/notes.rs`):
   - Implemented `ensure_slotted_notes(&self) -> Result<Vec<Note>, StoreError>`: runs a transaction checking existence of `is_slotted = 1 AND slot_index = i`; inserts missing slots with default content and `slot_index = i, db_index = i`; preserves existing user content intact.
   - Wired `ensure_slotted_notes` into `SqliteStore::open` and `SqliteStore::open_in_memory` so all stores automatically have the 9 slots ready on startup.
   - Implemented `list_slotted_notes(&self) -> Result<Vec<Note>, StoreError>` ordering by `slot_index ASC`.
   - Implemented `get_slotted_note(&self, slot_index: i32) -> Result<Option<Note>, StoreError>` with strict bounds checking (`0..=8`), returning `StoreError::InvalidOperation` on out-of-bounds queries.

3. **Slot Invariance Enforcement** (`crates/plainpad-core/src/store/notes.rs`, `void.rs`):
   - Updated `list_active_notes()` to filter `is_slotted = 0`, keeping the scratch notes stack distinct from slotted notes.
   - Updated `create_note()` and `restore_from_void()` to compute `db_index` based on unslotted notes (`is_slotted = 0`).
   - Updated `reorder_notes()` to target only unslotted notes (`WHERE is_slotted = 0`), preserving slotted notes from position displacement.
   - Confirmed `soft_delete_note()` rejects slotted note deletion with `StoreError::SlottedNoteCannotBeDeleted`.
   - Preserved ability to save empty content `""` on slotted notes without soft-deleting them.

4. **Tauri IPC Commands & App Initialization** (`src-tauri/src/main.rs`):
   - Added `list_slotted_notes` and `get_slotted_note` IPC commands returning `NoteMeta` and `Note`.
   - Registered both commands in `tauri::generate_handler![]`.
   - Called `store.ensure_slotted_notes()` during startup in `main()` before webview creation.

5. **Integration Test Suite** (`crates/plainpad-core/tests/slotted_notes_tests.rs`):
   - Created 4 comprehensive tests covering count & deity mappings, idempotence, bounds checking, and invariance rules.

## Commits

1. `f178a32`: `feat(phase-2): 9-slot Greek deity initialization, query methods, and slot invariance protections in SqliteStore`
2. `3532ce5`: `feat(phase-2): expose slotted note IPC commands in Tauri and create integration test suite`

## Verification

- `cargo test --package plainpad-core`: 55 unit/lifecycle/relational/schema tests passing green.
- `cargo test --test slotted_notes_tests`: 4 integration tests passing green.
- `cargo check --manifest-path src-tauri/Cargo.toml`: Tauri crate compiles cleanly without errors or warnings.
- Total test suite: 59 passing tests.

## Deviations Handled

1. **Rule 1 (Bug Fix - Test Adjustment):** Updated `test_slotted_note_void_immunity` in `tests/note_lifecycle_tests.rs` to query the initialized slot 1 note directly rather than creating a duplicate slotted note with conflicting slot_index.
