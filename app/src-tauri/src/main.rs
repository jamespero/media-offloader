// Release builds on Windows shouldn't open a console window.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Tauri shell around `offloader-core`: sets up the app and registers its commands.

mod commands;
mod state;
mod thumbs;

use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(state::Engine::default())
        .setup(|app| {
            let cache = app.path().app_cache_dir()?.join("thumbs");
            app.manage(thumbs::Thumbs::start(app.handle().clone(), cache));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::sources::list_sources,
            commands::sources::preview_source,
            commands::offload::prepare,
            commands::offload::start,
            commands::offload::cancel,
            commands::offload::is_running,
            commands::reverify::reverify_folder,
            commands::history::history_list,
            commands::history::history_detail,
            commands::history::forget_history,
            commands::offload::prepare_resume,
            commands::reverify::cancel_reverify,
            commands::system::read_text,
            commands::system::log_dir,
            commands::sources::eject,
            commands::system::thumbs_request,
            commands::links::reveal,
            commands::links::open_folder,
            commands::links::open_repo,
            commands::links::open_release
        ])
        .run(tauri::generate_context!())
        .expect("error while running Media Offloader");
}
