//! Tauri shell for JJ PDF Studio. Commands here stay thin: they validate input
//! and delegate to `pdf-core`.

/// Returns the `pdf-core` version. Proves the frontend -> Rust -> pdf-core path.
#[tauri::command]
fn ping_core() -> &'static str {
    pdf_core::version()
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![ping_core])
        .run(tauri::generate_context!())
        .expect("error while running JJ PDF Studio");
}
