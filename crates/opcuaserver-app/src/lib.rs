mod commands;
mod state;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            // Server lifecycle
            commands::start_server,
            commands::stop_server,
            commands::get_server_status,
            // Simulation data
            commands::get_simulation_data,
            // Server config
            commands::update_server_config,
            commands::get_server_config,
            // Address space management
            commands::add_folder,
            commands::add_node,
            commands::batch_add_nodes,
            commands::remove_node,
            commands::update_node,
            commands::get_address_space,
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
