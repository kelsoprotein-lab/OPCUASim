use tauri::State;
use crate::state::AppState;

#[tauri::command]
pub fn list_connections(_state: State<'_, AppState>) -> Vec<String> {
    vec![]
}
