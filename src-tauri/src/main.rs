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
    AppHandle, Emitter, Manager, WebviewWindowBuilder,
};
use tauri_plugin_autostart::ManagerExt;

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
    duration_ms: Option<u64>,
    #[allow(non_snake_case)]
    durationMs: Option<u64>,
    state: tauri::State<'_, Mutex<AppState>>,
) -> u64 {
    let d = duration_ms.or(durationMs).unwrap_or(300_000);
    state.lock().unwrap().timers.start(&name, d)
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
    app: AppHandle,
    state: tauri::State<'_, Mutex<AppState>>,
) -> Result<(), String> {
    state.lock().unwrap().store.set_setting(&key, value.clone()).map_err(|e| e.to_string())?;
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
    let s = state.lock().unwrap();
    let trash_list = s.store.list_trash();
    for item in trash_list {
        let trash_file = s.store.root().join("trash").join(format!("{}.txt", item.id));
        let _ = std::fs::remove_file(trash_file);
    }
    let trash_meta = s.store.root().join("state/trash.json");
    let _ = std::fs::write(trash_meta, "[]");
    Ok(())
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

#[tauri::command]
fn minimize_window(app: AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let _ = w.minimize();
    }
}

#[tauri::command]
fn hide_window(app: AppHandle) {
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
fn toggle_always_on_top(app: AppHandle, state: tauri::State<'_, Mutex<AppState>>) -> Result<bool, String> {
    if let Some(w) = app.get_webview_window("main") {
        let mut s = state.lock().unwrap();
        let current = s.store.get_settings().get("always_on_top").and_then(|v| v.as_bool()).unwrap_or(false);
        let new_val = !current;
        let _ = w.set_always_on_top(new_val);
        let _ = s.store.set_setting("always_on_top", serde_json::Value::Bool(new_val));
        Ok(new_val)
    } else {
        Err("window not found".into())
    }
}

#[tauri::command]
fn get_always_on_top(state: tauri::State<'_, Mutex<AppState>>) -> bool {
    state.lock().unwrap().store.get_settings().get("always_on_top").and_then(|v| v.as_bool()).unwrap_or(false)
}

#[tauri::command]
fn get_rates_info() -> plainpad_core::currency::RatesInfo {
    plainpad_core::currency::get_rates_info()
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

// ---------- main ----------

fn main() {
    let dir = data_dir();
    let store = plainpad_core::store::FileStore::open(&dir)
        .expect("failed to open plainpad data directory");
    let timer_path = dir.join("state/timers.json");
    let (timers, overdue) = plainpad_core::timers::TimerEngine::load(&timer_path);

    tauri::Builder::default()
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "settings" {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
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

            let is_pinned = state.lock().unwrap().store.get_settings().get("always_on_top").and_then(|v| v.as_bool()).unwrap_or(false);

            let mut builder = WebviewWindowBuilder::new(&handle, "main", tauri::WebviewUrl::App("index.html".into()))
                .title("Plainpad")
                .inner_size(w, h)
                .decorations(false)
                .transparent(true)
                .always_on_top(is_pinned)
                .skip_taskbar(false)
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
            minimize_window,
            hide_window,
            toggle_maximize,
            toggle_always_on_top,
            get_always_on_top,
            get_rates_info,
            refresh_rates_now,
            open_settings_window,
            get_data_dir_path,
            open_data_dir,
            empty_trash_now,
            is_autostart_enabled,
            set_autostart,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Plainpad");
}
