// Plainpad — desktop shell (Tauri 2)
// Window management, tray icon, global hotkey, IPC commands, timer watcher.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use plainpad_core::{intent, Note, NoteMeta};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use tauri::{
    image::Image,
    menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder},
    tray::TrayIconBuilder,
    AppHandle, Emitter, Manager, WebviewWindowBuilder,
};
use tauri_plugin_autostart::ManagerExt;

#[derive(Debug, Clone)]
pub struct WindowStateBackup {
    pub x: i32,
    pub y: i32,
    pub width: f64,
    pub height: f64,
    pub is_always_on_top: bool,
}

struct AppState {
    store: Arc<plainpad_core::store::SqliteStore>,
    timers: plainpad_core::timers::TimerEngine,
    autopaste: Arc<plainpad_core::clipboard::AutoPasteEngine>,
    window_backup: Option<WindowStateBackup>,
}

type AppStateLock = Mutex<AppState>;

fn data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("PLAINPAD_DATA_DIR") {
        return PathBuf::from(dir);
    }
    dirs_next().unwrap_or_else(|| std::env::current_dir().unwrap().join("plainpad-data"))
}

fn dirs_next() -> Option<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        std::env::var("APPDATA").ok().map(|d| PathBuf::from(d).join("plainpad"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        std::env::var("XDG_DATA_HOME")
            .ok()
            .map(PathBuf::from)
            .or_else(|| std::env::var("HOME").ok().map(|h| PathBuf::from(h).join(".local/share")))
            .map(|d| d.join("plainpad"))
    }
}

// ---------- IPC commands ----------

#[tauri::command]
fn list_notes(state: tauri::State<'_, Mutex<AppState>>) -> Vec<NoteMeta> {
    state
        .lock()
        .unwrap()
        .store
        .list_active_notes()
        .unwrap_or_default()
        .iter()
        .map(NoteMeta::from)
        .collect()
}

#[tauri::command]
fn list_slotted_notes(state: tauri::State<'_, Mutex<AppState>>) -> Vec<NoteMeta> {
    state
        .lock()
        .unwrap()
        .store
        .list_slotted_notes()
        .unwrap_or_default()
        .iter()
        .map(NoteMeta::from)
        .collect()
}

#[tauri::command]
fn get_slotted_note(
    slot: i32,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<Option<Note>, String> {
    state
        .lock()
        .unwrap()
        .store
        .get_slotted_note(slot)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn load_note(id: String, state: tauri::State<'_, Mutex<AppState>>) -> Option<Note> {
    state.lock().unwrap().store.load_note(&id).ok().flatten()
}

#[tauri::command]
fn save_note(
    id: String,
    text: String,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<NoteMeta, String> {
    state
        .lock()
        .unwrap()
        .store
        .save_note(&id, &text)
        .map(|n| NoteMeta::from(&n))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn create_note(state: tauri::State<'_, Mutex<AppState>>) -> String {
    state
        .lock()
        .unwrap()
        .store
        .create_note("")
        .map(|n| n.id)
        .expect("Failed to create note")
}

#[tauri::command]
fn delete_note(id: String, state: tauri::State<'_, Mutex<AppState>>) -> Result<(), String> {
    state
        .lock()
        .unwrap()
        .store
        .soft_delete_note(&id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn restore_note(
    id: String,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<Option<String>, String> {
    state
        .lock()
        .unwrap()
        .store
        .restore_from_void(&id)
        .map(|n| Some(n.id))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn list_trash(
    state: tauri::State<'_, Mutex<AppState>>,
) -> Vec<plainpad_core::TrashEntry> {
    state
        .lock()
        .unwrap()
        .store
        .list_void()
        .unwrap_or_default()
        .into_iter()
        .map(|n| {
            let deleted_at_ms = n
                .deleted_at
                .as_deref()
                .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.timestamp_millis() as u64)
                .unwrap_or(0);
            plainpad_core::TrashEntry {
                id: n.id.clone(),
                title: n.title(),
                deleted_at_ms,
            }
        })
        .collect()
}

#[tauri::command]
fn purge_void(state: tauri::State<'_, Mutex<AppState>>) -> Result<usize, String> {
    state
        .lock()
        .unwrap()
        .store
        .purge_void()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn permanently_delete_note(
    id: String,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    state
        .lock()
        .unwrap()
        .store
        .permanently_delete_note(&id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn set_locked(
    id: String,
    locked: bool,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    state
        .lock()
        .unwrap()
        .store
        .set_locked(&id, locked)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn set_archived(
    id: String,
    archived: bool,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    state
        .lock()
        .unwrap()
        .store
        .set_archived(&id, archived)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn set_private(
    id: String,
    private: bool,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    state
        .lock()
        .unwrap()
        .store
        .set_private(&id, private)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn reorder_notes(
    ids: Vec<String>,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    state
        .lock()
        .unwrap()
        .store
        .reorder_notes(&ids)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn create_pre_restore_snapshot(
    reason: String,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<String, String> {
    state
        .lock()
        .unwrap()
        .store
        .create_pre_restore_snapshot(&reason)
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn restore_from_snapshot(
    path: String,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    state
        .lock()
        .unwrap()
        .store
        .restore_from_snapshot(Path::new(&path))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn set_pinned(
    id: String,
    pinned: bool,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    state
        .lock()
        .unwrap()
        .store
        .set_setting(&format!("pinned_{}", id), serde_json::Value::Bool(pinned))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn set_plain(
    id: String,
    plain: bool,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    state
        .lock()
        .unwrap()
        .store
        .set_setting(&format!("plain_{}", id), serde_json::Value::Bool(plain))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn analyze_note(
    text: String,
    plain: bool,
    state: tauri::State<'_, Mutex<AppState>>,
) -> intent::Analysis {
    let settings = state
        .lock()
        .unwrap()
        .store
        .get_settings()
        .unwrap_or_default();
    let config = plainpad_core::trigger_config::TriggerConfig::from_settings(&settings);
    intent::analyze_with_config(&text, plain, &config)
}

#[tauri::command]
fn get_trigger_config(
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<plainpad_core::trigger_config::TriggerConfig, String> {
    let settings = state
        .lock()
        .unwrap()
        .store
        .get_settings()
        .map_err(|e| e.to_string())?;
    Ok(plainpad_core::trigger_config::TriggerConfig::from_settings(&settings))
}

#[tauri::command]
fn set_trigger_config(
    config: plainpad_core::trigger_config::TriggerConfig,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    let map = config.to_settings();
    let lock = state.lock().unwrap();
    for (k, v) in map {
        lock.store.set_setting(&k, v).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn export_note(
    text: String,
    kind: String,
) -> Result<String, String> {
    let k = match kind.as_str() {
        "markdown" | "md" => plainpad_core::export::ExportKind::Markdown,
        _ => plainpad_core::export::ExportKind::Txt,
    };
    Ok(plainpad_core::export::export(&text, k))
}

#[tauri::command]
fn export_file_name(text: String, kind: String) -> String {
    let k = match kind.as_str() {
        "markdown" | "md" => plainpad_core::export::ExportKind::Markdown,
        _ => plainpad_core::export::ExportKind::Txt,
    };
    plainpad_core::export::export_file_name(&text, k)
}

#[tauri::command]
fn export_note_to_target(
    id: String,
    target: String,
    options: plainpad_core::export::ExportOptions,
    vault: Option<String>,
    custom_template: Option<String>,
    database_name: Option<String>,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<String, String> {
    let s = state.lock().unwrap();
    let note = s
        .store
        .load_note(&id)
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("Note {id} not found"))?;

    let (title, body) = plainpad_core::export::extract_title_and_body(
        &note.content,
        options.use_first_line_as_title,
    );
    let processed_body = if options.omit_keywords {
        plainpad_core::export::omit_keywords_from_text(&body)
    } else {
        body
    };

    match target.to_lowercase().as_str() {
        "txt" | "plaintext" => Ok(processed_body),
        "md" | "markdown" => Ok(plainpad_core::export::to_markdown_with_options(
            &note.content,
            &options,
        )),
        "obsidian" => {
            let v = vault.unwrap_or_else(|| "Notes".to_string());
            let content = if options.convert_to_markdown {
                plainpad_core::export::to_markdown_with_options(&processed_body, &options)
            } else {
                processed_body
            };
            Ok(plainpad_core::export::format_obsidian_uri(&v, &title, &content))
        }
        "bear" => {
            let content = if options.convert_to_markdown {
                plainpad_core::export::to_markdown_with_options(&processed_body, &options)
            } else {
                processed_body
            };
            Ok(plainpad_core::export::format_bear_uri(&title, &content))
        }
        "apple_notes" | "applenotes" => {
            Ok(plainpad_core::export::format_apple_notes(&title, &processed_body, &options))
        }
        "notion" => {
            Ok(plainpad_core::export::format_notion(&title, &processed_body, database_name.as_deref()))
        }
        "custom" => {
            let template = custom_template.unwrap_or_else(|| {
                "myapp://create?title={TITLE}&text={CONTENT}&date={DATE}".to_string()
            });
            Ok(plainpad_core::export::format_custom_url(
                &template,
                &title,
                &processed_body,
                chrono::Utc::now(),
            ))
        }
        other => Err(format!("Unknown export target: {other}")),
    }
}

#[tauri::command]
fn export_all_notes_zip(
    options: plainpad_core::export::ExportOptions,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<Vec<u8>, String> {
    let s = state.lock().unwrap();
    let mut notes = s.store.list_active_notes().map_err(|e| e.to_string())?;
    if let Ok(slotted) = s.store.list_slotted_notes() {
        notes.extend(slotted);
    }
    plainpad_core::export::export_all_to_zip(&notes, &options)
}

#[tauri::command]
fn resolve_custom_url(text: String, template: String) -> String {
    let title = plainpad_core::title_from_text(&text);
    plainpad_core::export::format_custom_url(&template, &title, &text, chrono::Utc::now())
}

#[tauri::command]
fn get_all_notes(state: tauri::State<'_, Mutex<AppState>>) -> Vec<plainpad_core::Note> {
    let s = state.lock().unwrap();
    let mut notes = s.store.list_active_notes().unwrap_or_default();
    if let Ok(slotted) = s.store.list_slotted_notes() {
        notes.extend(slotted);
    }
    notes
}

fn pause_timer_if_configured(app: &AppHandle) {
    let state = app.state::<Mutex<AppState>>();
    let s = state.lock().unwrap();
    let should_pause = s
        .store
        .get_setting("pauseTheTimerIfAppClosed")
        .ok()
        .flatten()
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    if should_pause {
        if let Some(paused) = s.timers.pause_active() {
            let _ = app.emit("timer-tick", paused);
        }
    }
}

#[tauri::command]
fn start_timer(
    name: String,
    duration_ms: Option<u64>,
    #[allow(non_snake_case)]
    durationMs: Option<u64>,
    timer_type: Option<String>,
    break_ms: Option<u64>,
    #[allow(non_snake_case)]
    breakMs: Option<u64>,
    state: tauri::State<'_, Mutex<AppState>>,
) -> plainpad_core::timers::Timer {
    let d = duration_ms.or(durationMs).unwrap_or(300_000);
    let s = state.lock().unwrap();
    match timer_type.as_deref() {
        Some("stopwatch") => s.timers.start_stopwatch(&name),
        Some("pomodoro") => {
            let b = break_ms.or(breakMs).unwrap_or(5 * 60_000);
            s.timers.start_pomodoro(&name, d, b)
        }
        _ => s.timers.start_countdown(&name, d),
    }
}

#[tauri::command]
fn start_timer_phrase(
    phrase: String,
    app: AppHandle,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<plainpad_core::timers::Timer, String> {
    let timer = state
        .lock()
        .unwrap()
        .timers
        .start_with_phrase(&phrase)
        .ok_or_else(|| "Failed to parse or start timer phrase".to_string())?;
    let _ = app.emit("timer-tick", &timer);
    Ok(timer)
}

#[tauri::command]
fn cancel_timer(
    id: Option<u64>,
    app: AppHandle,
    state: tauri::State<'_, Mutex<AppState>>,
) -> bool {
    let stopped = state.lock().unwrap().timers.stop(id);
    let _ = app.emit("timer-stopped", serde_json::json!({ "id": id }));
    stopped
}

#[tauri::command]
fn pause_timer(
    id: Option<u64>,
    app: AppHandle,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Option<plainpad_core::timers::Timer> {
    let timer = state.lock().unwrap().timers.pause_resume(id)?;
    let _ = app.emit("timer-tick", &timer);
    Some(timer)
}

#[tauri::command]
fn pause_resume_timer(
    id: Option<u64>,
    app: AppHandle,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Option<plainpad_core::timers::Timer> {
    let timer = state.lock().unwrap().timers.pause_resume(id)?;
    let _ = app.emit("timer-tick", &timer);
    Some(timer)
}

#[tauri::command]
fn resume_timer(
    _id: Option<u64>,
    app: AppHandle,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Option<plainpad_core::timers::Timer> {
    let timer = state.lock().unwrap().timers.resume_active()?;
    let _ = app.emit("timer-tick", &timer);
    Some(timer)
}

#[tauri::command]
fn restart_timer(
    id: Option<u64>,
    app: AppHandle,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Option<plainpad_core::timers::Timer> {
    let timer = state.lock().unwrap().timers.restart(id)?;
    let _ = app.emit("timer-tick", &timer);
    Some(timer)
}

#[tauri::command]
fn get_active_timer(
    state: tauri::State<'_, Mutex<AppState>>,
) -> Option<plainpad_core::timers::Timer> {
    state.lock().unwrap().timers.get_active()
}

#[tauri::command]
fn list_timers(
    state: tauri::State<'_, Mutex<AppState>>,
) -> Vec<plainpad_core::timers::Timer> {
    state.lock().unwrap().timers.list()
}

#[tauri::command]
fn get_no_peek_streak(
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<(u32, u32), String> {
    state
        .lock()
        .unwrap()
        .store
        .get_no_peek_streak()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn increment_no_peek_streak(
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<(u32, u32), String> {
    state
        .lock()
        .unwrap()
        .store
        .increment_no_peek_streak()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn reset_no_peek_streak(
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<(u32, u32), String> {
    state
        .lock()
        .unwrap()
        .store
        .reset_no_peek_streak()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_break_facts() -> Vec<plainpad_core::BreakFact> {
    plainpad_core::get_break_facts()
}

#[tauri::command]
fn record_peek_penalty(
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<(u32, u32), String> {
    reset_no_peek_streak(state)
}

#[tauri::command]
fn complete_break_streak(
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<(u32, u32), String> {
    increment_no_peek_streak(state)
}

#[tauri::command]
fn enter_break_fullscreen(
    window: tauri::Window,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    let (stored_w, stored_h, stored_x, stored_y, stored_pinned) = {
        let s = state.lock().unwrap();
        let geo = s
            .store
            .get_setting("geometry")
            .ok()
            .flatten()
            .unwrap_or(serde_json::json!({}));
        let w = geo.get("width").and_then(|v| v.as_f64()).unwrap_or(420.0);
        let h = geo.get("height").and_then(|v| v.as_f64()).unwrap_or(560.0);
        let x = geo.get("x").and_then(|v| v.as_i64()).unwrap_or(100) as i32;
        let y = geo.get("y").and_then(|v| v.as_i64()).unwrap_or(100) as i32;
        let pinned = s
            .store
            .get_setting("always_on_top")
            .ok()
            .flatten()
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        (w, h, x, y, pinned)
    };

    let scale = window.scale_factor().unwrap_or(1.0);
    let (width, height) = if let Ok(sz) = window.inner_size() {
        let log = sz.to_logical::<f64>(scale);
        if log.width > 100.0 && log.height > 100.0 {
            (log.width, log.height)
        } else {
            (stored_w, stored_h)
        }
    } else {
        (stored_w, stored_h)
    };

    let (x, y) = if let Ok(pos) = window.outer_position() {
        let log = pos.to_logical::<f64>(scale);
        (log.x as i32, log.y as i32)
    } else {
        (stored_x, stored_y)
    };

    {
        let mut s = state.lock().unwrap();
        if s.window_backup.is_none() {
            s.window_backup = Some(WindowStateBackup {
                x,
                y,
                width,
                height,
                is_always_on_top: stored_pinned,
            });
        }
    }

    let _ = window.set_always_on_top(true);
    window.set_fullscreen(true).map_err(|e| e.to_string())?;
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
    Ok(())
}

#[tauri::command]
fn exit_break_fullscreen(
    window: tauri::Window,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    window.set_fullscreen(false).map_err(|e| e.to_string())?;

    let backup_opt = {
        let mut s = state.lock().unwrap();
        s.window_backup.take()
    };

    if let Some(backup) = backup_opt {
        let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize::new(backup.width, backup.height)));
        let _ = window.set_position(tauri::Position::Logical(tauri::LogicalPosition::new(backup.x as f64, backup.y as f64)));
        let _ = window.set_always_on_top(backup.is_always_on_top);
    } else {
        let s = state.lock().unwrap();
        let geo = s
            .store
            .get_setting("geometry")
            .ok()
            .flatten()
            .unwrap_or(serde_json::json!({}));
        let w = geo.get("width").and_then(|v| v.as_f64()).unwrap_or(420.0);
        let h = geo.get("height").and_then(|v| v.as_f64()).unwrap_or(560.0);
        let pinned = s
            .store
            .get_setting("always_on_top")
            .ok()
            .flatten()
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let _ = window.set_size(tauri::Size::Logical(tauri::LogicalSize::new(w, h)));
        let _ = window.set_always_on_top(pinned);
        if let (Some(x), Some(y)) = (geo.get("x").and_then(|v| v.as_f64()), geo.get("y").and_then(|v| v.as_f64())) {
            let _ = window.set_position(tauri::Position::Logical(tauri::LogicalPosition::new(x, y)));
        }
    }
    let _ = window.show();
    let _ = window.unminimize();
    let _ = window.set_focus();
    Ok(())
}

#[tauri::command]
fn get_settings(state: tauri::State<'_, Mutex<AppState>>) -> serde_json::Value {
    let map = state
        .lock()
        .unwrap()
        .store
        .get_settings()
        .unwrap_or_default();
    serde_json::Value::Object(map)
}

#[tauri::command]
fn set_setting(
    key: String,
    value: serde_json::Value,
    app: AppHandle,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    state
        .lock()
        .unwrap()
        .store
        .set_setting(&key, value.clone())
        .map_err(|e| e.to_string())?;
    let _ = app.emit("setting-changed", serde_json::json!({ "key": key, "value": value }));
    Ok(())
}

#[tauri::command]
async fn open_settings_window(app: AppHandle) -> Result<(), String> {
    if let Some(win) = app.get_webview_window("settings") {
        let _ = win.unminimize();
        let _ = win.show();
        let _ = win.set_focus();
        return Ok(());
    }

    let win = WebviewWindowBuilder::new(&app, "settings", tauri::WebviewUrl::App("settings.html".into()))
        .title("Plainpad Preferences")
        .inner_size(680.0, 560.0)
        .min_inner_size(620.0, 480.0)
        .resizable(true)
        .decorations(true)
        .center()
        .build()
        .map_err(|e| e.to_string())?;

    let _ = win.show();
    let _ = win.set_focus();
    Ok(())
}

#[tauri::command]
fn get_data_dir_path() -> String {
    data_dir().to_string_lossy().to_string()
}

#[tauri::command]
fn open_data_dir() -> Result<String, String> {
    let p = data_dir();
    #[cfg(target_os = "windows")]
    {
        let _ = std::process::Command::new("explorer").arg(&p).spawn();
    }
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(&p).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(&p).spawn();
    }
    Ok(p.to_string_lossy().to_string())
}

#[tauri::command]
fn empty_trash_now(state: tauri::State<'_, Mutex<AppState>>) -> Result<(), String> {
    state
        .lock()
        .unwrap()
        .store
        .purge_void()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn is_autostart_enabled(app: AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
fn set_autostart(enabled: bool, app: AppHandle) -> Result<(), String> {
    if enabled {
        app.autolaunch().enable().map_err(|e| e.to_string())
    } else {
        app.autolaunch().disable().map_err(|e| e.to_string())
    }
}

#[tauri::command]
fn get_geometry(state: tauri::State<'_, Mutex<AppState>>) -> serde_json::Value {
    state
        .lock()
        .unwrap()
        .store
        .get_setting("geometry")
        .ok()
        .flatten()
        .unwrap_or(serde_json::json!({}))
}

#[tauri::command]
fn set_geometry(
    geo: serde_json::Value,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    state
        .lock()
        .unwrap()
        .store
        .set_setting("geometry", geo)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn eval_math(expr: String, state: tauri::State<'_, Mutex<AppState>>) -> Option<String> {
    let mode_str = state
        .lock()
        .unwrap()
        .store
        .get_setting("angleMeasurementMode")
        .ok()
        .flatten()
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "degrees".to_string());
    let mode = mode_str
        .parse::<plainpad_core::mathwrap::AngleMode>()
        .unwrap_or(plainpad_core::mathwrap::AngleMode::Degrees);
    plainpad_core::mathwrap::evaluate_with_mode(&expr, mode)
        .ok()
        .map(|m| m.result)
}

#[tauri::command]
fn set_angle_mode(
    mode: String,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    let normalized = mode.trim().to_lowercase();
    if normalized != "degrees" && normalized != "radians" {
        return Err("Angle mode must be 'degrees' or 'radians'".into());
    }
    state
        .lock()
        .unwrap()
        .store
        .set_setting("angleMeasurementMode", serde_json::Value::String(normalized))
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_angle_mode(state: tauri::State<'_, Mutex<AppState>>) -> Result<String, String> {
    let mode = state
        .lock()
        .unwrap()
        .store
        .get_setting("angleMeasurementMode")
        .map_err(|e| e.to_string())?
        .and_then(|v| v.as_str().map(|s| s.to_string()))
        .unwrap_or_else(|| "degrees".to_string());
    Ok(mode)
}

#[tauri::command]
fn block_stats(lines: Vec<String>) -> plainpad_core::mathwrap::BlockStats {
    let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
    plainpad_core::mathwrap::block_stats(&refs)
}

#[tauri::command]
fn minimize_window(app: AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.minimize();
    }
}

#[tauri::command]
fn hide_window(app: AppHandle) {
    pause_timer_if_configured(&app);
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.hide();
    }
}

#[tauri::command]
fn toggle_maximize(app: AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        if w.is_maximized().unwrap_or(false) {
            let _ = w.unmaximize();
        } else {
            let _ = w.maximize();
        }
    }
}

#[tauri::command]
fn start_dragging(app: AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.start_dragging();
    }
}

#[tauri::command]
fn toggle_always_on_top(app: AppHandle, state: tauri::State<'_, Mutex<AppState>>) -> Result<bool, String> {
    if let Some(w) = app.get_webview_window("main") {
        let s = state.lock().unwrap();
        let current = s
            .store
            .get_setting("always_on_top")
            .ok()
            .flatten()
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let new_val = !current;
        let _ = w.set_always_on_top(new_val);
        let _ = s
            .store
            .set_setting("always_on_top", serde_json::Value::Bool(new_val));
        let _ = s
            .store
            .set_setting("_lastPinState", serde_json::Value::Bool(new_val));
        let _ = app.emit("setting-changed", serde_json::json!({ "key": "always_on_top", "value": new_val }));
        Ok(new_val)
    } else {
        Err("window not found".into())
    }
}

#[tauri::command]
fn get_always_on_top(app: AppHandle, state: tauri::State<'_, Mutex<AppState>>) -> bool {
    if let Some(w) = app.get_webview_window("main") {
        if let Ok(pinned) = w.is_always_on_top() {
            return pinned;
        }
    }
    state
        .lock()
        .unwrap()
        .store
        .get_setting("always_on_top")
        .ok()
        .flatten()
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

fn register_hotkey_internal(app: &AppHandle, hotkey_str: &str) -> Result<tauri_plugin_global_shortcut::Shortcut, String> {
    use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};
    let parsed: Shortcut = hotkey_str
        .parse()
        .map_err(|e| format!("Failed to parse hotkey '{hotkey_str}': {e}"))?;

    let _ = app.global_shortcut().unregister_all();

    let hotkey_handle = app.clone();
    app.global_shortcut()
        .on_shortcut(parsed.clone(), move |_app, _shortcut, event| {
            if event.state != ShortcutState::Pressed {
                return;
            }
            if let Some(w) = hotkey_handle.get_webview_window("main") {
                let is_vis = w.is_visible().unwrap_or(false);
                let is_foc = w.is_focused().unwrap_or(false);
                if is_vis && is_foc {
                    pause_timer_if_configured(&hotkey_handle);
                    w.hide().ok();
                } else {
                    w.show().ok();
                    w.unminimize().ok();
                    w.set_focus().ok();
                }
            }
        })
        .map_err(|e| format!("Failed to register shortcut '{hotkey_str}': {e}"))?;
    Ok(parsed)
}

#[tauri::command]
fn register_global_hotkey(
    app: AppHandle,
    state: tauri::State<'_, Mutex<AppState>>,
    hotkey: String,
) -> Result<(), String> {
    let clean_hotkey = hotkey.trim();
    if clean_hotkey.is_empty() {
        return Err("Hotkey cannot be empty".into());
    }
    register_hotkey_internal(&app, clean_hotkey)?;
    let s = state.lock().unwrap();
    let _ = s.store.set_setting("globalHotkey", serde_json::Value::String(clean_hotkey.to_string()));
    let _ = s.store.set_setting("global_hotkey", serde_json::Value::String(clean_hotkey.to_string()));
    let _ = app.emit("setting-changed", serde_json::json!({ "key": "globalHotkey", "value": clean_hotkey }));
    Ok(())
}

#[tauri::command]
fn open_external_url(url: String) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let _ = std::process::Command::new("open").arg(&url).spawn();
    #[cfg(target_os = "linux")]
    let _ = std::process::Command::new("xdg-open").arg(&url).spawn();
    #[cfg(target_os = "windows")]
    let _ = std::process::Command::new("cmd").args(["/C", "start", &url]).spawn();
    Ok(())
}

#[tauri::command]
fn get_rates_info() -> plainpad_core::currency::RatesInfo {
    plainpad_core::currency::get_rates_info()
}

#[tauri::command]
fn get_currency_rates_info() -> plainpad_core::currency::RatesInfo {
    plainpad_core::currency::get_rates_info()
}

#[tauri::command]
fn sync_currency_rates(
    app: AppHandle,
    state: tauri::State<'_, AppStateLock>,
) -> Result<plainpad_core::currency::RatesInfo, String> {
    let dir = data_dir();
    let rates_path = dir.join("state/exchange_rates.json");
    let store = state.lock().unwrap().store.clone();
    let app_clone = app.clone();

    // Detached thread for non-blocking fetch (NFR-02)
    std::thread::spawn(move || {
        match fetch_and_save_rates(&rates_path) {
            Ok(info) => {
                let now_str = chrono::Utc::now().to_rfc3339();
                let _ = store.set_setting(
                    "lastCurrencyUpdateDate",
                    serde_json::Value::String(now_str),
                );
                let _ = app_clone.emit("rates-updated", &info);
            }
            Err(e) => {
                eprintln!("[plainpad] Background rate sync failed: {}", e);
            }
        }
    });

    Ok(plainpad_core::currency::get_rates_info())
}

#[tauri::command]
fn set_currency_format_settings(
    state: tauri::State<'_, AppStateLock>,
    thousands_sep: String,
    decimal_sep: String,
    decimal_points: usize,
) -> Result<(), String> {
    let store = state.lock().unwrap().store.clone();
    store
        .set_setting("thousandsSeparator", serde_json::Value::String(thousands_sep.clone()))
        .map_err(|e| e.to_string())?;
    store
        .set_setting("decimalSeparator", serde_json::Value::String(decimal_sep.clone()))
        .map_err(|e| e.to_string())?;
    store
        .set_setting("numberOfDecimalPoints", serde_json::json!(decimal_points))
        .map_err(|e| e.to_string())?;

    let cfg = plainpad_core::currency::CurrencyFormatConfig {
        thousands_separator: thousands_sep,
        decimal_separator: decimal_sep,
        decimal_points,
        use_thousands_separator: true,
    };
    plainpad_core::currency::set_format_config(cfg);
    Ok(())
}

#[tauri::command]
fn set_manual_currency_rate(
    state: tauri::State<'_, AppStateLock>,
    code: String,
    rate: f64,
) -> Result<(), String> {
    let store = state.lock().unwrap().store.clone();
    let mut current_map = match store.get_setting("_manualCurrencyRates") {
        Ok(Some(serde_json::Value::Object(map))) => map,
        _ => serde_json::Map::new(),
    };
    let upper = code.to_uppercase();
    if rate <= 0.0 {
        current_map.remove(&upper);
        plainpad_core::currency::remove_manual_rate(&upper);
    } else {
        current_map.insert(upper.clone(), serde_json::json!(rate));
        plainpad_core::currency::set_manual_rate(&upper, rate);
    }
    store
        .set_setting(
            "_manualCurrencyRates",
            serde_json::Value::Object(current_map),
        )
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn convert_currency_ipc(amount: f64, from: String, to: String) -> Result<String, String> {
    let engine = plainpad_core::currency::global_engine();
    let lock = engine.read().map_err(|e| e.to_string())?;
    let canonical_from = plainpad_core::currency::canonicalize_currency(&from)
        .unwrap_or(&from);
    let canonical_to = plainpad_core::currency::canonicalize_currency(&to)
        .unwrap_or(&to);
    let converted = lock
        .convert(amount, canonical_from, canonical_to)
        .ok_or_else(|| format!("Cannot convert from {} to {}", from, to))?;
    Ok(plainpad_core::currency::format_converted(converted, canonical_to))
}

#[tauri::command]
fn start_autopaste(
    app: AppHandle,
    state: tauri::State<'_, AppStateLock>,
) -> Result<(), String> {
    let autopaste = {
        let s = state.lock().unwrap();
        s.autopaste.start();
        s.autopaste.clone()
    };

    let app_handle = app.clone();
    std::thread::spawn(move || {
        let mut reader = plainpad_core::clipboard::SystemClipboardReader::new_lenient();
        while autopaste.is_active() {
            if let Ok(Some(text)) = reader.read_text() {
                if let Some(new_clip) = autopaste.process_clip(&text) {
                    let _ = app_handle.emit("autopaste-clip", serde_json::json!({ "text": new_clip }));
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(250));
        }
    });

    Ok(())
}

#[tauri::command]
fn stop_autopaste(state: tauri::State<'_, AppStateLock>) -> Result<(), String> {
    state.lock().unwrap().autopaste.stop();
    Ok(())
}

#[tauri::command]
fn get_autopaste_status(state: tauri::State<'_, AppStateLock>) -> bool {
    state.lock().unwrap().autopaste.is_active()
}

#[tauri::command]
fn submit_clipboard_clip(
    text: String,
    app: AppHandle,
    state: tauri::State<'_, AppStateLock>,
) -> Result<Option<String>, String> {
    let autopaste = state.lock().unwrap().autopaste.clone();
    if !autopaste.is_active() {
        return Ok(None);
    }
    if let Some(new_clip) = autopaste.process_clip(&text) {
        let _ = app.emit("autopaste-clip", serde_json::json!({ "text": new_clip }));
        Ok(Some(new_clip))
    } else {
        Ok(None)
    }
}

#[tauri::command]
fn refresh_rates_now(app: AppHandle) -> Result<plainpad_core::currency::RatesInfo, String> {
    let dir = data_dir();
    let rates_path = dir.join("state/exchange_rates.json");
    let info = fetch_and_save_rates(&rates_path)?;
    let _ = app.emit("rates-updated", &info);
    Ok(info)
}

fn fetch_and_save_rates(rates_path: &std::path::Path) -> Result<plainpad_core::currency::RatesInfo, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .user_agent("Plainpad/0.1")
        .build()
        .map_err(|e| e.to_string())?;

    let fiat_res = client
        .get("https://open.er-api.com/v6/latest/USD")
        .send()
        .map_err(|e| format!("Fiat fetch failed: {}", e))?;

    let mut fiat_json: serde_json::Value = fiat_res
        .json()
        .map_err(|e| format!("Fiat json parse failed: {}", e))?;

    // Fetch crypto rates from CoinGecko
    if let Ok(crypto_res) = client
        .get("https://api.coingecko.com/api/v3/simple/price?ids=bitcoin,ethereum,solana&vs_currencies=usd")
        .send()
    {
        if let Ok(crypto_json) = crypto_res.json::<serde_json::Value>() {
            if let Some(rates_obj) = fiat_json.get_mut("rates").and_then(|r| r.as_object_mut()) {
                if let Some(btc_usd) = crypto_json.get("bitcoin").and_then(|b| b.get("usd")).and_then(|u| u.as_f64()) {
                    if btc_usd > 0.0 {
                        rates_obj.insert("BTC".into(), serde_json::json!(1.0 / btc_usd));
                    }
                }
                if let Some(eth_usd) = crypto_json.get("ethereum").and_then(|b| b.get("usd")).and_then(|u| u.as_f64()) {
                    if eth_usd > 0.0 {
                        rates_obj.insert("ETH".into(), serde_json::json!(1.0 / eth_usd));
                    }
                }
                if let Some(sol_usd) = crypto_json.get("solana").and_then(|b| b.get("usd")).and_then(|u| u.as_f64()) {
                    if sol_usd > 0.0 {
                        rates_obj.insert("SOL".into(), serde_json::json!(1.0 / sol_usd));
                    }
                }
            }
        }
    }

    let json_str = serde_json::to_string_pretty(&fiat_json).map_err(|e| e.to_string())?;

    // Update in-memory engine
    plainpad_core::currency::update_rates(&json_str);

    // Save to disk
    if let Some(parent) = rates_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(rates_path, &json_str);

    Ok(plainpad_core::currency::get_rates_info())
}

// ---------- Extension Runtime IPC Commands ----------

#[tauri::command]
fn store_api_key(
    key_name: String,
    key_value: String,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    state
        .lock()
        .unwrap()
        .store
        .store_api_key(&key_name, &key_value)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn list_api_keys(
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<Vec<plainpad_core::ApiKeyMeta>, String> {
    state
        .lock()
        .unwrap()
        .store
        .list_api_keys()
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_api_key(
    key_name: String,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<bool, String> {
    state
        .lock()
        .unwrap()
        .store
        .delete_api_key(&key_name)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn call_api(
    api_key_name: String,
    url: String,
    method: String,
    headers: String,
    body: Option<String>,
    approved_endpoints: Vec<String>,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<plainpad_core::CallApiResponse, String> {
    let parsed_headers = plainpad_core::parse_headers(&headers);
    let req = plainpad_core::CallApiRequest {
        api_key_name,
        url,
        method,
        headers: parsed_headers,
        body,
        approved_endpoints,
    };
    state
        .lock()
        .unwrap()
        .store
        .call_api(req)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_extension_preference(
    extension_name: String,
    key: String,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<Option<String>, String> {
    state
        .lock()
        .unwrap()
        .store
        .get_extension_preference(&extension_name, &key)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn set_extension_preference(
    extension_name: String,
    key: String,
    value: String,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    state
        .lock()
        .unwrap()
        .store
        .set_extension_preference(&extension_name, &key, &value)
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn fetch_ai_models(
    provider: String,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<Vec<plainpad_core::AiModel>, String> {
    state
        .lock()
        .unwrap()
        .store
        .fetch_ai_models(&provider)
}

fn play_native_sound(sound_file: &str) {
    let clean = sound_file
        .trim_end_matches(".m4a")
        .trim_end_matches(".wav")
        .trim_end_matches(".mp3")
        .trim_end_matches(".ogg");

    let clean = clean.to_string();
    let original = sound_file.to_string();

    std::thread::spawn(move || {
        let candidates = [
            format!("ui/dist/sounds/{clean}.wav"),
            format!("ui/sounds/{clean}.wav"),
            format!("ui/dist/sounds/{original}"),
            format!("ui/sounds/{original}"),
        ];

        let found_path = candidates.into_iter().find(|p| std::path::Path::new(p).exists());

        #[cfg(target_os = "linux")]
        {
            if let Some(ref path) = found_path {
                if let Ok(mut child) = std::process::Command::new("pw-play").arg(path).spawn() {
                    let _ = child.wait();
                    return;
                }
                if let Ok(mut child) = std::process::Command::new("paplay").arg(path).spawn() {
                    let _ = child.wait();
                    return;
                }
                if let Ok(mut child) = std::process::Command::new("aplay").arg("-q").arg(path).spawn() {
                    let _ = child.wait();
                    return;
                }
            }
            let _ = std::process::Command::new("canberra-gtk-play")
                .arg("-i")
                .arg("complete")
                .spawn();
        }

        #[cfg(target_os = "macos")]
        {
            if let Some(ref path) = found_path {
                let _ = std::process::Command::new("afplay").arg(path).spawn();
            }
        }

        #[cfg(target_os = "windows")]
        {
            if let Some(ref path) = found_path {
                let script = format!("(New-Object Media.SoundPlayer '{}').PlaySync()", path.replace('/', "\\"));
                let _ = std::process::Command::new("powershell")
                    .args(["-NoProfile", "-Command", &script])
                    .spawn();
            }
        }
    });
}

#[tauri::command]
fn play_sound(sound: String) {
    play_native_sound(&sound);
}

// ---------- main ----------

fn main() {
    eprintln!("[Plainpad] 1. Process started.");
    #[cfg(target_os = "linux")]
    {
        // Avoid WebKitGTK 2.40+ hardware dmabuf compositing crash / blank screen on Linux Wayland/X11
        if std::env::var("WEBKIT_DISABLE_DMABUF_RENDERER").is_err() {
            std::env::set_var("WEBKIT_DISABLE_DMABUF_RENDERER", "1");
        }
        eprintln!("[Plainpad] 2. Linux environment configured.");
    }

    let dir = data_dir();
    eprintln!("[Plainpad] 3. Data directory: {:?}", dir);
    let db_path = dir.join("plainpad.db");
    eprintln!("[Plainpad] 4. Opening SQLite store at: {:?}", db_path);
    let store = Arc::new(
        plainpad_core::store::SqliteStore::open(&db_path)
            .expect("Failed to initialize SQLite store"),
    );
    eprintln!("[Plainpad] 5. SQLite store opened.");
    let timer_path = dir.join("state/timers.json");
    let (timers, overdue) = plainpad_core::timers::TimerEngine::load(&timer_path);
    eprintln!("[Plainpad] 6. Timers loaded.");

    tauri::Builder::default()
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "settings" {
                    api.prevent_close();
                    let _ = window.hide();
                } else if window.label() == "main" {
                    pause_timer_if_configured(window.app_handle());
                }
            }
        })
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(Mutex::new(AppState {
            store,
            timers,
            autopaste: Arc::new(plainpad_core::clipboard::AutoPasteEngine::new()),
            window_backup: None,
        }))
        .setup(move |app| {
            let handle = app.handle().clone();

            let state = app.state::<Mutex<AppState>>();
            let (w, h, x, y, is_pinned) = {
                let s = state.lock().unwrap();
                let geo = s
                    .store
                    .get_setting("geometry")
                    .ok()
                    .flatten()
                    .unwrap_or(serde_json::json!({}));
                let w = geo.get("width").and_then(|v| v.as_f64()).unwrap_or(520.0);
                let h = geo.get("height").and_then(|v| v.as_f64()).unwrap_or(600.0);
                let x = geo.get("x").and_then(|v| v.as_f64());
                let y = geo.get("y").and_then(|v| v.as_f64());

                let restore_pin = s
                    .store
                    .get_setting("restorePinOnOpen")
                    .ok()
                    .flatten()
                    .or_else(|| s.store.get_setting("restore_pin").ok().flatten())
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                let is_pinned = if restore_pin {
                    s.store
                        .get_setting("_lastPinState")
                        .ok()
                        .flatten()
                        .or_else(|| s.store.get_setting("always_on_top").ok().flatten())
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
                } else {
                    let _ = s.store.set_setting("always_on_top", serde_json::Value::Bool(false));
                    false
                };

                (w, h, x, y, is_pinned)
            };

            let is_transparent = if cfg!(target_os = "linux") {
                std::env::var("PLAINPAD_TRANSPARENT").map(|v| v == "1" || v == "true").unwrap_or(false)
            } else {
                true
            };

            eprintln!("[Plainpad] 7. Obtaining main window...");
            let win = if let Some(existing) = app.get_webview_window("main") {
                eprintln!("[Plainpad] 8. Found pre-configured window from tauri.conf.json.");
                existing
            } else {
                eprintln!("[Plainpad] 8. Building main window dynamically...");
                let mut builder = WebviewWindowBuilder::new(&handle, "main", tauri::WebviewUrl::App("index.html".into()))
                    .title("Plainpad")
                    .inner_size(w, h)
                    .decorations(false)
                    .transparent(is_transparent)
                    .always_on_top(is_pinned)
                    .skip_taskbar(false)
                    .visible(true)
                    .resizable(true);

                if let (Some(px), Some(py)) = (x, y) {
                    builder = builder.position(px, py);
                } else {
                    builder = builder.center();
                }
                builder.build()?
            };

            if let (Some(px), Some(py)) = (x, y) {
                let _ = win.set_position(tauri::Position::Logical(tauri::LogicalPosition::new(px, py)));
            } else {
                let _ = win.center();
            }
            let _ = win.set_size(tauri::Size::Logical(tauri::LogicalSize::new(w, h)));
            let _ = win.set_always_on_top(is_pinned);
            let _ = win.show();
            let _ = win.unminimize();
            let _ = win.set_focus();
            #[cfg(debug_assertions)]
            {
                win.open_devtools();
            }
            eprintln!("[Plainpad] 9. Main window presented ({}x{}, pinned={}, transparent={}).", w, h, is_pinned, is_transparent);

            // Tray icon
            let icon_bytes = include_bytes!("../icons/icon.png");
            let icon = Image::from_bytes(icon_bytes).expect("icon.png");

            let show_item = MenuItemBuilder::with_id("show", "Show Plainpad").build(app)?;
            let hide_item = MenuItemBuilder::with_id("hide", "Hide Plainpad").build(app)?;
            let new_note_item = MenuItemBuilder::with_id("new_note", "New Note").build(app)?;

            let mut slots_sub = SubmenuBuilder::new(app, "Slotted Notes");
            let slot_names = [
                ("slot_0", "1: Hermes"),
                ("slot_1", "2: Apollo"),
                ("slot_2", "3: Ares"),
                ("slot_3", "4: Athena"),
                ("slot_4", "5: Poseidon"),
                ("slot_5", "6: Demeter"),
                ("slot_6", "7: Hephaestus"),
                ("slot_7", "8: Artemis"),
                ("slot_8", "9: Zeus"),
            ];
            for (id, label) in &slot_names {
                let item = MenuItemBuilder::with_id(*id, *label).build(app)?;
                slots_sub = slots_sub.item(&item);
            }
            let slots_submenu = slots_sub.build()?;

            let settings_item = MenuItemBuilder::with_id("settings", "Settings...").build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "Quit Plainpad").build(app)?;
            let menu = MenuBuilder::new(app)
                .item(&show_item)
                .item(&hide_item)
                .separator()
                .item(&new_note_item)
                .item(&slots_submenu)
                .separator()
                .item(&settings_item)
                .separator()
                .item(&quit_item)
                .build()?;

            let tray_handle = handle.clone();
            if let Err(e) = TrayIconBuilder::new()
                .icon(icon)
                .menu(&menu)
                .tooltip("Plainpad")
                .on_menu_event(move |_app, event| {
                    match event.id().as_ref() {
                        "show" => {
                            if let Some(w) = tray_handle.get_webview_window("main") {
                                w.show().ok();
                                w.unminimize().ok();
                                w.set_focus().ok();
                            }
                        }
                        "hide" => {
                            if let Some(w) = tray_handle.get_webview_window("main") {
                                pause_timer_if_configured(&tray_handle);
                                w.hide().ok();
                            }
                        }
                        "new_note" => {
                            if let Some(w) = tray_handle.get_webview_window("main") {
                                w.show().ok();
                                w.unminimize().ok();
                                w.set_focus().ok();
                                let _ = w.emit("menu-new-note", ());
                            }
                        }
                        "settings" => {
                            let h = tray_handle.clone();
                            tauri::async_runtime::spawn(async move {
                                let _ = open_settings_window(h).await;
                            });
                        }
                        "quit" => {
                            std::process::exit(0);
                        }
                        id if id.starts_with("slot_") => {
                            if let Ok(slot_idx) = id.strip_prefix("slot_").unwrap_or("0").parse::<usize>() {
                                if let Some(w) = tray_handle.get_webview_window("main") {
                                    w.show().ok();
                                    w.unminimize().ok();
                                    w.set_focus().ok();
                                    let _ = w.emit("menu-select-slot", slot_idx);
                                }
                            }
                        }
                        _ => {}
                    }
                })
                .build(app)
            {
                eprintln!("[Tray] Warning: System tray could not be initialized: {e}");
            }

            // Global hotkey initialized from settings (default "Ctrl+Space")
            let hotkey_str = {
                let s = state.lock().unwrap();
                s.store
                    .get_setting("globalHotkey")
                    .ok()
                    .flatten()
                    .or_else(|| s.store.get_setting("global_hotkey").ok().flatten())
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
                    .unwrap_or_else(|| "Ctrl+Space".to_string())
            };
            let hotkey_handle = handle.clone();
            tauri::async_runtime::spawn(async move {
                // Yield briefly to ensure setup finishes and the event loop starts
                std::thread::sleep(std::time::Duration::from_millis(50));
                if let Err(e) = register_hotkey_internal(&hotkey_handle, &hotkey_str) {
                    eprintln!("[Hotkey] Warning: Could not register global shortcut '{hotkey_str}': {e}");
                } else {
                    println!("[Plainpad] Registered global summon hotkey: {hotkey_str}");
                }
            });

            // Fire overdue timers (expired while app was closed)
            for t in &overdue {
                let _ = handle.emit("timer-fired", serde_json::json!({
                    "id": t.id, "name": t.name, "overdue": t.overdue
                }));
            }

            // Timer watcher thread → emit events to the UI
            {
                let s = app.state::<Mutex<AppState>>();
                let timers = &s.lock().unwrap().timers;
                let fire_handle = handle.clone();
                timers.spawn_watcher(move |event| {
                    match event {
                        plainpad_core::timers::TimerEvent::Tick(timer) => {
                            let _ = fire_handle.emit("timer-tick", timer);
                        }
                        plainpad_core::timers::TimerEvent::Fired { id, name, sound, overdue } => {
                            let _ = fire_handle.emit("timer-fired", serde_json::json!({
                                "id": id, "name": name, "sound": sound, "overdue": overdue
                            }));
                            play_native_sound(&sound);
                            use tauri_plugin_notification::NotificationExt;
                            let title = if name.is_empty() { "Plainpad Timer" } else { &name };
                            let body = if overdue {
                                format!("{title} (missed while away)")
                            } else {
                                format!("{title} finished!")
                            };
                            let _ = fire_handle.notification().builder().title(title).body(&body).show();
                        }
                        plainpad_core::timers::TimerEvent::PhaseTransition { id, is_work_phase, cycle_count, sound } => {
                            let _ = fire_handle.emit("timer-phase-transition", serde_json::json!({
                                "id": id, "is_work_phase": is_work_phase, "cycle_count": cycle_count, "sound": sound
                            }));
                            play_native_sound(&sound);
                            use tauri_plugin_notification::NotificationExt;
                            let title = if is_work_phase { "Break Finished" } else { "Work Complete" };
                            let body = if is_work_phase {
                                format!("Starting work cycle {cycle_count}")
                            } else {
                                format!("Completed work cycle {cycle_count}. Time for a break!")
                            };
                            let _ = fire_handle.notification().builder().title(title).body(&body).show();
                        }
                    }
                });
            }

            // Restore currency preferences and manual rates from SQLite app_state
            {
                let s = app.state::<Mutex<AppState>>();
                let store = s.lock().unwrap().store.clone();
                plainpad_core::currency::init_from_store_settings(&store);
            }

            // Currency rates: initial load from disk cache if present
            let rates_path = dir.join("state/exchange_rates.json");
            let mut needs_refresh = true;
            if rates_path.exists() {
                if let Ok(content) = std::fs::read_to_string(&rates_path) {
                    plainpad_core::currency::update_rates(&content);
                    let info = plainpad_core::currency::get_rates_info();
                    let now_sec = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    // If rates were fetched within the last 24 hours, no need to re-fetch on startup
                    if !info.is_seed && info.last_updated_unix > 0 && now_sec.saturating_sub(info.last_updated_unix) < 86_400 {
                        needs_refresh = false;
                    }
                }
            }

            // Background worker: automatically fetches once daily on startup if outdated (>24h), then refreshes daily
            let bg_rates_path = rates_path.clone();
            let bg_handle = handle.clone();
            std::thread::spawn(move || {
                if needs_refresh {
                    match fetch_and_save_rates(&bg_rates_path) {
                        Ok(info) => {
                            let _ = bg_handle.emit("rates-updated", &info);
                        }
                        Err(err) => {
                            eprintln!("[plainpad] Background rate refresh failed (using cached/seed rates): {}", err);
                        }
                    }
                }
                loop {
                    // Check every 6 hours if >24h has elapsed since last update
                    std::thread::sleep(std::time::Duration::from_secs(6 * 3600));
                    let info = plainpad_core::currency::get_rates_info();
                    let now_sec = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    if info.is_seed || now_sec.saturating_sub(info.last_updated_unix) >= 86_400 {
                        if let Ok(new_info) = fetch_and_save_rates(&bg_rates_path) {
                            let _ = bg_handle.emit("rates-updated", &new_info);
                        }
                    }
                }
            });

            eprintln!("[Plainpad] 11. Setup hook finished successfully.");
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            list_notes,
            load_note,
            save_note,
            create_note,
            delete_note,
            restore_note,
            list_trash,
            set_pinned,
            set_plain,
            analyze_note,
            export_note,
            export_file_name,
            export_note_to_target,
            export_all_notes_zip,
            resolve_custom_url,
            get_all_notes,
            start_timer,
            start_timer_phrase,
            pause_timer,
            pause_resume_timer,
            resume_timer,
            restart_timer,
            get_active_timer,
            cancel_timer,
            list_timers,
            get_settings,
            set_setting,
            get_geometry,
            set_geometry,
            eval_math,
            set_angle_mode,
            get_angle_mode,
            block_stats,
            minimize_window,
            hide_window,
            toggle_maximize,
            start_dragging,
            toggle_always_on_top,
            get_always_on_top,
            get_rates_info,
            get_currency_rates_info,
            sync_currency_rates,
            set_currency_format_settings,
            set_manual_currency_rate,
            convert_currency_ipc,
            refresh_rates_now,
            open_settings_window,
            get_data_dir_path,
            open_data_dir,
            empty_trash_now,
            is_autostart_enabled,
            set_autostart,
            purge_void,
            permanently_delete_note,
            set_locked,
            set_archived,
            set_private,
            reorder_notes,
            create_pre_restore_snapshot,
            restore_from_snapshot,
            list_slotted_notes,
            get_slotted_note,
            get_trigger_config,
            set_trigger_config,
            start_autopaste,
            stop_autopaste,
            get_autopaste_status,
            submit_clipboard_clip,
            open_external_url,
            store_api_key,
            list_api_keys,
            delete_api_key,
            call_api,
            get_extension_preference,
            set_extension_preference,
            fetch_ai_models,
            register_global_hotkey,
            play_sound,
            get_no_peek_streak,
            increment_no_peek_streak,
            reset_no_peek_streak,
            get_break_facts,
            record_peek_penalty,
            complete_break_streak,
            enter_break_fullscreen,
            exit_break_fullscreen,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Plainpad");
}

#[cfg(test)]
mod tests {
    use tauri_plugin_global_shortcut::Shortcut;

    #[test]
    fn test_parse_shortcuts() {
        assert!("Ctrl+Space".parse::<Shortcut>().is_ok());
        println!("Cmd+Space: {:?}", "Cmd+Space".parse::<Shortcut>());
        println!("Command+Space: {:?}", "Command+Space".parse::<Shortcut>());
        println!("Super+Space: {:?}", "Super+Space".parse::<Shortcut>());
        println!("Shift+Ctrl+Space: {:?}", "Shift+Ctrl+Space".parse::<Shortcut>());
    }
}
