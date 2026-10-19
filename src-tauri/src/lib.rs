//! Tauri shell for JJ PDF Studio. Commands here stay thin: they validate input
//! and delegate to `pdf-core`.

mod commands;
mod settings;

use std::sync::{Arc, Mutex};

use tauri::{Manager, PhysicalPosition, PhysicalSize, State, WebviewWindow, WindowEvent};

use settings::{Settings, Store, WindowState};

/// Returns the `pdf-core` version. Proves the frontend -> Rust -> pdf-core path.
#[tauri::command]
fn ping_core() -> &'static str {
    pdf_core::version()
}

/// The saved settings, read once by the frontend at startup.
#[tauri::command]
fn get_settings(store: State<'_, Store>) -> Settings {
    store.get()
}

/// Remembers the theme; `None` goes back to following Windows.
#[tauri::command]
fn set_theme(theme: Option<String>, store: State<'_, Store>) -> Result<(), String> {
    match theme.as_deref() {
        None | Some("light" | "dark") => {
            store.update(|s| s.theme = theme);
            Ok(())
        }
        Some(_) => Err("Unknown theme.".into()),
    }
}

/// True when the top-left area of `state` is on one of the connected screens,
/// so a window saved on a monitor that is gone is not restored off screen.
fn on_a_screen(window: &WebviewWindow, state: &WindowState) -> bool {
    let probe = (state.x + 48, state.y + 16);
    window
        .available_monitors()
        .unwrap_or_default()
        .iter()
        .any(|m| {
            let (p, s) = (m.position(), m.size());
            probe.0 >= p.x
                && probe.0 < p.x + s.width as i32
                && probe.1 >= p.y
                && probe.1 < p.y + s.height as i32
        })
}

/// Applies the saved geometry, then shows the window (it starts hidden so the
/// user never sees it jump from the default place).
fn restore_window(window: &WebviewWindow, saved: Option<WindowState>) {
    if let Some(state) = saved {
        if state.width >= 320 && state.height >= 240 && on_a_screen(window, &state) {
            let _ = window.set_size(PhysicalSize::new(state.width, state.height));
            let _ = window.set_position(PhysicalPosition::new(state.x, state.y));
            if state.maximized {
                let _ = window.maximize();
            }
        }
    }
    let _ = window.show();
}

/// Keeps the last normal (not maximized, not minimized) geometry and writes it
/// to the settings file when the window closes.
fn remember_window(window: &WebviewWindow, saved: Option<WindowState>) {
    let normal = Arc::new(Mutex::new(saved));
    let handle = window.clone();
    window.on_window_event(move |event| match event {
        WindowEvent::Resized(_) | WindowEvent::Moved(_) => {
            if handle.is_minimized().unwrap_or(false) || handle.is_maximized().unwrap_or(false) {
                return;
            }
            if let (Ok(pos), Ok(size)) = (handle.outer_position(), handle.outer_size()) {
                *normal.lock().unwrap() = Some(WindowState {
                    x: pos.x,
                    y: pos.y,
                    width: size.width,
                    height: size.height,
                    maximized: false,
                });
            }
        }
        WindowEvent::CloseRequested { .. } => {
            let maximized = handle.is_maximized().unwrap_or(false);
            if let Some(mut state) = *normal.lock().unwrap() {
                state.maximized = maximized;
                handle.state::<Store>().update(|s| s.window = Some(state));
            }
        }
        _ => {}
    });
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(commands::Allowed::default())
        .manage(Store::open())
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                let saved = app.state::<Store>().get().window;
                restore_window(&window, saved);
                remember_window(&window, saved);
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            ping_core,
            get_settings,
            set_theme,
            commands::pages::open_pdf,
            commands::pages::read_pdf,
            commands::pages::save_pages,
            commands::pages::open_saved,
            commands::pages::reveal_saved,
            commands::merge::merge_pdfs,
            commands::images::open_image,
            commands::images::image_thumbnail,
            commands::images::images_to_pdf,
            commands::images::images_to_folder,
            commands::resize::plan_resize,
            commands::resize::resize_pdf,
            commands::split::parse_page_ranges,
            commands::split::split_pdf,
            commands::rotate::plan_rotation,
        ])
        .run(tauri::generate_context!())
        .expect("error while running JJ PDF Studio");
}
