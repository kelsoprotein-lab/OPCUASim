mod commands;
mod state;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            // Connection commands
            commands::create_connection,
            commands::connect,
            commands::disconnect,
            commands::delete_connection,
            commands::list_connections,
            commands::get_endpoints,
            // Log commands
            commands::get_communication_logs,
            commands::clear_communication_logs,
            commands::export_logs_csv,
            // Group commands
            commands::create_group,
            commands::delete_group,
            commands::list_groups,
            commands::add_nodes_to_group,
            commands::remove_nodes_from_group,
            // Project file commands
            commands::save_project,
            commands::load_project,
        ])
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
