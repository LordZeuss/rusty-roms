mod query;
mod setup;
mod data;
use data::{scrape, setup, console_fill, remove_old_db};
mod download;
mod status;
mod settings;
mod start;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        // .setup(|_app| {
        //     setup::temp_dir();
        //     std::thread::spawn(|| {
        //         run_startup_tasks();
        //     });
        //     Ok(())
        // })
        .invoke_handler(tauri::generate_handler![
            query::search_games,
            download::download_file,
            status::network_check,
            settings::get_download_dir,
            settings::set_download_dir,
            settings::pick_download_dir,
            settings::clear_download_dir,
            start::run_startup_tasks
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

