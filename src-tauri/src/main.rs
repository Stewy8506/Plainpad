// Plainpad — desktop shell (Tauri 2)
// Window management, tray icon, global hotkey, IPC commands, timer watcher.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use plainpad_core::{intent, Note, NoteMeta};
use std::path::PathBuf;
use std::sync::Mutex;
use tauri::{
    image::Image,
    menu::{MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
    Emitter, Manager, WebviewWindowBuilder,
};

struct AppState {
    store: plainpad_core::store::FileStore,
    timers: plainpad_core::timers::TimerEngine,
}

fn data_dir() -> PathBuf {
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
    state.lock().unwrap().store.list_notes()
}

#[tauri::command]
fn load_note(id: String, state: tauri::State<'_, Mutex<AppState>>) -> Option<Note> {
    state.lock().unwrap().store.load_note(&id)
}

#[tauri::command]
fn save_note(
    id: String,
    text: String,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<NoteMeta, String> {
    state.lock().unwrap().store.save_note(&id, &text).map_err(|e| e.to_string())
}

#[tauri::command]
fn create_note(state: tauri::State<'_, Mutex<AppState>>) -> String {
    state.lock().unwrap().store.create_note()
}

#[tauri::command]
fn delete_note(id: String, state: tauri::State<'_, Mutex<AppState>>) -> Result<(), String> {
    state.lock().unwrap().store.delete_note(&id).map_err(|e| e.to_string())
}

#[tauri::command]
fn restore_note(
    id: String,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<Option<String>, String> {
    state.lock().unwrap().store.restore_note(&id).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_trash(
    state: tauri::State<'_, Mutex<AppState>>,
) -> Vec<plainpad_core::TrashEntry> {
    state.lock().unwrap().store.list_trash()
}

#[tauri::command]
fn set_pinned(
    id: String,
    pinned: bool,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    state.lock().unwrap().store.set_pinned(&id, pinned).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_plain(
    id: String,
    plain: bool,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    state.lock().unwrap().store.set_plain(&id, plain).map_err(|e| e.to_string())
}

#[tauri::command]
fn analyze_note(text: String, plain: bool) -> intent::Analysis {
    intent::analyze(&text, plain)
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
fn start_timer(
    name: String,
    duration_ms: u64,
    state: tauri::State<'_, Mutex<AppState>>,
) -> u64 {
    state.lock().unwrap().timers.start(&name, duration_ms)
}

#[tauri::command]
fn cancel_timer(id: u64, state: tauri::State<'_, Mutex<AppState>>) -> bool {
    state.lock().unwrap().timers.cancel(id)
}

#[tauri::command]
fn list_timers(
    state: tauri::State<'_, Mutex<AppState>>,
) -> Vec<plainpad_core::timers::Timer> {
    state.lock().unwrap().timers.list()
}

#[tauri::command]
fn get_settings(state: tauri::State<'_, Mutex<AppState>>) -> serde_json::Value {
    state.lock().unwrap().store.get_settings()
}

#[tauri::command]
fn set_setting(
    key: String,
    value: serde_json::Value,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    state.lock().unwrap().store.set_setting(&key, value).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_geometry(state: tauri::State<'_, Mutex<AppState>>) -> serde_json::Value {
    state.lock().unwrap().store.get_geometry()
}

#[tauri::command]
fn set_geometry(
    geo: serde_json::Value,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    state.lock().unwrap().store.set_geometry(geo).map_err(|e| e.to_string())
}

#[tauri::command]
fn eval_math(expr: String) -> Option<String> {
    plainpad_core::mathwrap::evaluate(&expr).ok().map(|m| m.result)
}

#[tauri::command]
fn block_stats(lines: Vec<String>) -> plainpad_core::mathwrap::BlockStats {
    let refs: Vec<&str> = lines.iter().map(|s| s.as_str()).collect();
    plainpad_core::mathwrap::block_stats(&refs)
}

// ---------- main ----------

fn main() {
    let dir = data_dir();
    let store = plainpad_core::store::FileStore::open(&dir)
        .expect("failed to open plainpad data directory");
    let timer_path = dir.join("state/timers.json");
    let (timers, overdue) = plainpad_core::timers::TimerEngine::load(&timer_path);

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(Mutex::new(AppState { store, timers }))
        .setup(move |app| {
            let handle = app.handle().clone();

            let state = app.state::<Mutex<AppState>>();
            let geo = state.lock().unwrap().store.get_geometry();
            let w = geo.get("width").and_then(|v| v.as_f64()).unwrap_or(520.0);
            let h = geo.get("height").and_then(|v| v.as_f64()).unwrap_or(600.0);
            let x = geo.get("x").and_then(|v| v.as_f64());
            let y = geo.get("y").and_then(|v| v.as_f64());

            let mut builder = WebviewWindowBuilder::new(&handle, "main", tauri::WebviewUrl::App("index.html".into()))
                .title("Plainpad")
                .inner_size(w, h)
                .decorations(false)
                .transparent(true)
                .always_on_top(true)
                .skip_taskbar(true)
                .visible(true)
                .resizable(true);

            if let (Some(px), Some(py)) = (x, y) {
                builder = builder.position(px, py);
            }

            let _win = builder.build()?;

            // Tray icon
            let icon_bytes = include_bytes!("../icons/icon.png");
            let icon = Image::from_bytes(icon_bytes).expect("icon.png");

            let show_item = MenuItemBuilder::with_id("show", "Show").build(app)?;
            let hide_item = MenuItemBuilder::with_id("hide", "Hide").build(app)?;
            let quit_item = MenuItemBuilder::with_id("quit", "Quit").build(app)?;
            let menu = MenuBuilder::new(app)
                .item(&show_item)
                .item(&hide_item)
                .separator()
                .item(&quit_item)
                .build()?;

            let tray_handle = handle.clone();
            TrayIconBuilder::new()
                .icon(icon)
                .menu(&menu)
                .tooltip("Plainpad")
                .on_menu_event(move |_app, event| {
                    match event.id().as_ref() {
                        "show" => {
                            if let Some(w) = tray_handle.get_webview_window("main") {
                                w.show().ok();
                                w.set_focus().ok();
                            }
                        }
                        "hide" => {
                            if let Some(w) = tray_handle.get_webview_window("main") {
                                w.hide().ok();
                            }
                        }
                        "quit" => {
                            std::process::exit(0);
                        }
                        _ => {}
                    }
                })
                .build(app)?;

            // Global hotkey (Alt+A)
            use tauri_plugin_global_shortcut::{Code, GlobalShortcutExt, Modifiers, Shortcut};
            let shortcut = Shortcut::new(Some(Modifiers::ALT), Code::KeyA);
            let hotkey_handle = handle.clone();
            app.global_shortcut().on_shortcut(shortcut, move |_app, _shortcut, _event| {
                if let Some(w) = hotkey_handle.get_webview_window("main") {
                    if w.is_visible().unwrap_or(false) {
                        w.hide().ok();
                    } else {
                        w.show().ok();
                        w.set_focus().ok();
                    }
                }
            })?;

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
                timers.spawn_watcher(move |fired| {
                    let _ = fire_handle.emit("timer-fired", serde_json::json!({
                        "id": fired.id, "name": fired.name, "overdue": fired.overdue
                    }));
                });
            }

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
            start_timer,
            cancel_timer,
            list_timers,
            get_settings,
            set_setting,
            get_geometry,
            set_geometry,
            eval_math,
            block_stats,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Plainpad");
}
