#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod clipboard;
mod icons;
mod indexer;

use clipboard::SharedClips;
use indexer::Indexer;
use serde::Serialize;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager, WindowEvent};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

struct AppState {
    indexer: Indexer,
    clips: SharedClips,
    hotkey: Mutex<String>,
    icon_cache: Mutex<HashMap<String, Option<String>>>,
}

#[derive(Serialize)]
struct Status {
    hotkey: String,
    indexed: usize,
    indexing: bool,
}

/// Append a line to %LOCALAPPDATA%\Lukfor\lukfor.log — the release build has
/// no console, so this is the only way to see startup diagnostics.
fn log_line(msg: &str) {
    eprintln!("[lukfor] {msg}");
    if let Some(base) = dirs::data_local_dir() {
        let dir = base.join("Lukfor");
        let _ = std::fs::create_dir_all(&dir);
        use std::io::Write;
        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(dir.join("lukfor.log"))
        {
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let _ = writeln!(f, "{ts} {msg}");
        }
    }
}

#[tauri::command]
fn search(query: String, state: tauri::State<AppState>) -> Vec<indexer::SearchResult> {
    state.indexer.search(&query, 20)
}

#[tauri::command]
fn open_entry(path: String, state: tauri::State<AppState>) -> Result<(), String> {
    // Only open paths that came from our own index — never raw user input.
    if !state.indexer.holds(&path) {
        return Err("unknown entry".into());
    }
    // Store/UWP apps live in the shell AppsFolder namespace, not on disk —
    // they can't be opened as a file path, so launch via explorer.exe.
    #[cfg(windows)]
    if path.starts_with("shell:AppsFolder\\") {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        return std::process::Command::new("explorer.exe")
            .arg(&path)
            .creation_flags(CREATE_NO_WINDOW)
            .spawn()
            .map(|_| ())
            .map_err(|e| e.to_string());
    }
    tauri_plugin_opener::open_path(&path, None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    if !url.starts_with("https://www.google.com/search?q=") {
        return Err("blocked url".into());
    }
    tauri_plugin_opener::open_url(&url, None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_icon(path: String, state: tauri::State<AppState>) -> Option<String> {
    // Icons only for app entries from our own index — same rule as open_entry.
    if !state.indexer.holds_app(&path) {
        return None;
    }
    let mut cache = state.icon_cache.lock().unwrap();
    if let Some(hit) = cache.get(&path) {
        return hit.clone();
    }
    if cache.len() > 2000 {
        cache.clear();
    }
    let icon = icons::extract(&path);
    cache.insert(path, icon.clone());
    icon
}

#[tauri::command]
fn copy_text(text: String, state: tauri::State<AppState>) -> Result<(), String> {
    let mut cb = arboard::Clipboard::new().map_err(|e| e.to_string())?;
    cb.set_text(text.clone()).map_err(|e| e.to_string())?;
    clipboard::remember(&state.clips, &text);
    Ok(())
}

#[tauri::command]
fn clipboard_history(state: tauri::State<AppState>) -> Vec<clipboard::Clip> {
    state.clips.lock().unwrap().iter().cloned().collect()
}

#[tauri::command]
fn get_status(state: tauri::State<AppState>) -> Status {
    Status {
        hotkey: state.hotkey.lock().unwrap().clone(),
        indexed: state.indexer.len(),
        indexing: state.indexer.is_indexing(),
    }
}

/// Rebuild the index now, so apps and folders added since startup show up
/// without waiting for the 03:00 pass. False = a build was already running.
#[tauri::command]
fn reindex(state: tauri::State<AppState>) -> bool {
    state.indexer.request_rebuild()
}

#[tauri::command]
fn hide_window(window: tauri::WebviewWindow) {
    let _ = window.hide();
}

fn toggle_window(app: &tauri::AppHandle) {
    let Some(win) = app.get_webview_window("main") else {
        log_line("toggle: main window missing");
        return;
    };
    if win.is_visible().unwrap_or(false) {
        log_line("toggle: hide");
        let _ = win.hide();
    } else {
        log_line("toggle: show");
        // Windows can silently collapse a hidden non-resizable window (e.g.
        // after a DPI/resolution change or display sleep), leaving it 15×15.
        // Re-assert the intended size on every show so the panel always
        // appears full-sized.
        if let Err(e) = win.set_size(tauri::LogicalSize::new(660.0, 460.0)) {
            log_line(&format!("toggle: set_size failed: {e}"));
        }
        let _ = win.center();
        if let Err(e) = win.show() {
            log_line(&format!("toggle: show failed: {e}"));
        }
        let _ = win.set_focus();
        let _ = win.emit("lukfor://shown", ());
        // Opening the panel is the one moment we know the user is about to
        // search, so it's where a stale index gets refreshed — in the
        // background, with the current results still live meanwhile.
        app.state::<AppState>().indexer.refresh_if_stale();
    }
}

fn main() {
    let state = AppState {
        indexer: Indexer::new(),
        clips: Arc::new(Mutex::new(VecDeque::new())),
        hotkey: Mutex::new(String::new()),
        icon_cache: Mutex::new(HashMap::new()),
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    if event.state() == ShortcutState::Pressed {
                        toggle_window(app);
                    }
                })
                .build(),
        )
        .manage(state)
        .setup(|app| {
            let state = app.state::<AppState>();
            state.indexer.spawn_startup_build();
            state.indexer.spawn_scheduler();
            clipboard::spawn_watcher(state.clips.clone());

            // Alt+Space first (Spotlight-style); if another app owns it,
            // fall back to Ctrl+Alt+Space so we never silently fail.
            let mut chosen = String::new();
            for combo in ["Alt+Space", "Ctrl+Alt+Space", "Ctrl+Shift+Space"] {
                let sc: Shortcut = combo.parse().expect("valid shortcut");
                match app.global_shortcut().register(sc) {
                    Ok(()) => {
                        chosen = combo.to_string();
                        break;
                    }
                    Err(e) => {
                        log_line(&format!("could not register {combo}: {e}"));
                    }
                }
            }
            if chosen.is_empty() {
                log_line("WARNING: no global hotkey registered");
            } else {
                log_line(&format!("hotkey registered: {chosen}"));
            }
            *state.hotkey.lock().unwrap() = chosen;
            Ok(())
        })
        .on_window_event(|window, event| {
            match event {
                // Click outside / focus lost → hide, Spotlight-style.
                WindowEvent::Focused(false) => {
                    let _ = window.hide();
                }
                // Closing the window (e.g. Alt+F4) hides it instead; the app
                // keeps running in the background waiting for the hotkey.
                WindowEvent::CloseRequested { api, .. } => {
                    api.prevent_close();
                    let _ = window.hide();
                }
                _ => {}
            }
        })
        .invoke_handler(tauri::generate_handler![
            search,
            open_entry,
            open_url,
            get_icon,
            copy_text,
            clipboard_history,
            get_status,
            reindex,
            hide_window
        ])
        .run(tauri::generate_context!())
        .expect("error while running Lukfor");
}
