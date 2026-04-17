use serde::{Deserialize, Serialize};
use tauri::State;

use opcuasim_core::server::models::{
    DataType, ServerConfig, ServerFolder, ServerNode, SimulationMode,
};

use crate::state::{AddressSpaceDto, AppState, ServerFolderDto, ServerNodeDto, ServerStatusDto};

// --- Server lifecycle ---

#[tauri::command]
pub async fn start_server(state: State<'_, AppState>) -> Result<(), String> {
    let config = state.config.read().unwrap().clone();
    let folders = state.folders.read().unwrap().clone();
    let nodes = state.nodes.read().unwrap().clone();
    let server = state.server.clone();

    server.start(&config, &folders, &nodes).await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn stop_server(state: State<'_, AppState>) -> Result<(), String> {
    let server = state.server.clone();
    server.stop().await.map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_server_status(state: State<'_, AppState>) -> Result<ServerStatusDto, String> {
    let server = state.server.clone();
    let server_state = server.state().await;
    let node_count = state.nodes.read().unwrap().len();
    let folder_count = state.folders.read().unwrap().len();

    Ok(ServerStatusDto {
        state: format!("{:?}", server_state),
        node_count,
        folder_count,
    })
}

// --- Simulation data polling ---

#[derive(Serialize)]
pub struct SimulationDataDto {
    pub values: Vec<SimValueDto>,
    pub seq: u64,
}

#[derive(Serialize)]
pub struct SimValueDto {
    pub node_id: String,
    pub value: String,
}

#[tauri::command]
pub async fn get_simulation_data(
    state: State<'_, AppState>,
    since_seq: u64,
) -> Result<SimulationDataDto, String> {
    let server = state.server.clone();
    if let Some(engine) = server.simulation_engine().await {
        let (changed, seq) = engine.get_values_since(since_seq).await;
        Ok(SimulationDataDto {
            values: changed.into_iter().map(|(nid, val)| SimValueDto {
                node_id: nid,
                value: val,
            }).collect(),
            seq,
        })
    } else {
        Ok(SimulationDataDto { values: vec![], seq: 0 })
    }
}

// --- Server config ---

#[tauri::command]
pub async fn update_server_config(
    state: State<'_, AppState>,
    config: ServerConfig,
) -> Result<(), String> {
    *state.config.write().unwrap() = config;
    Ok(())
}

#[tauri::command]
pub async fn get_server_config(state: State<'_, AppState>) -> Result<ServerConfig, String> {
    Ok(state.config.read().unwrap().clone())
}

// --- Address space management ---

#[tauri::command]
pub async fn add_folder(
    state: State<'_, AppState>,
    node_id: String,
    display_name: String,
    parent_id: String,
) -> Result<(), String> {
    let folder = ServerFolder {
        node_id: node_id.clone(),
        display_name: display_name.clone(),
        parent_id: parent_id.clone(),
    };

    // Add to live address space if server is running
    let server = state.server.clone();
    if let Some(nm) = server.node_manager().await {
        let ns_index = server.namespace_index().await;
        let mut address_space = nm.address_space().write();
        opcuasim_core::server::address_space::populate_address_space(
            &mut address_space,
            ns_index,
            &[folder.clone()],
            &[],
        );
    }

    state.folders.write().unwrap().push(folder);
    Ok(())
}

#[derive(Deserialize)]
pub struct AddNodeParams {
    pub node_id: String,
    pub display_name: String,
    pub parent_id: String,
    pub data_type: DataType,
    pub writable: bool,
    pub simulation: SimulationMode,
}

#[tauri::command]
pub async fn add_node(
    state: State<'_, AppState>,
    params: AddNodeParams,
) -> Result<(), String> {
    let node = ServerNode {
        node_id: params.node_id,
        display_name: params.display_name,
        parent_id: params.parent_id,
        data_type: params.data_type,
        writable: params.writable,
        simulation: params.simulation,
        update_seq: 0,
        current_value: None,
    };

    // Add to live address space if server is running
    let server = state.server.clone();
    if let Some(nm) = server.node_manager().await {
        let ns_index = server.namespace_index().await;
        let mut address_space = nm.address_space().write();
        opcuasim_core::server::address_space::add_variable_node(
            &mut address_space,
            ns_index,
            &node,
        );
    }

    state.nodes.write().unwrap().push(node);
    Ok(())
}

#[tauri::command]
pub async fn batch_add_nodes(
    state: State<'_, AppState>,
    nodes: Vec<AddNodeParams>,
) -> Result<usize, String> {
    let count = nodes.len();
    let server_nodes: Vec<ServerNode> = nodes.into_iter().map(|params| ServerNode {
        node_id: params.node_id,
        display_name: params.display_name,
        parent_id: params.parent_id,
        data_type: params.data_type,
        writable: params.writable,
        simulation: params.simulation,
        update_seq: 0,
        current_value: None,
    }).collect();

    // Add to live address space if server is running
    let server = state.server.clone();
    if let Some(nm) = server.node_manager().await {
        let ns_index = server.namespace_index().await;
        let mut address_space = nm.address_space().write();
        for node in &server_nodes {
            opcuasim_core::server::address_space::add_variable_node(
                &mut address_space,
                ns_index,
                node,
            );
        }
    }

    state.nodes.write().unwrap().extend(server_nodes);
    Ok(count)
}

#[tauri::command]
pub async fn remove_node(
    state: State<'_, AppState>,
    node_id: String,
) -> Result<(), String> {
    // Remove from live address space if server is running
    let server = state.server.clone();
    if let Some(nm) = server.node_manager().await {
        let ns_index = server.namespace_index().await;
        let mut address_space = nm.address_space().write();
        opcuasim_core::server::address_space::remove_node(
            &mut address_space,
            ns_index,
            &node_id,
        );
    }

    // Remove from folders and nodes (including children)
    {
        let mut folders = state.folders.write().unwrap();
        folders.retain(|f| f.node_id != node_id && f.parent_id != node_id);
    }
    {
        let mut nodes = state.nodes.write().unwrap();
        nodes.retain(|n| n.node_id != node_id && n.parent_id != node_id);
    }
    Ok(())
}

#[tauri::command]
pub async fn update_node(
    state: State<'_, AppState>,
    node_id: String,
    display_name: Option<String>,
    data_type: Option<DataType>,
    writable: Option<bool>,
    simulation: Option<SimulationMode>,
) -> Result<(), String> {
    let mut nodes = state.nodes.write().unwrap();
    if let Some(node) = nodes.iter_mut().find(|n| n.node_id == node_id) {
        if let Some(name) = display_name { node.display_name = name; }
        if let Some(dt) = data_type { node.data_type = dt; }
        if let Some(w) = writable { node.writable = w; }
        if let Some(sim) = simulation { node.simulation = sim; }
        Ok(())
    } else {
        Err(format!("Node '{}' not found", node_id))
    }
}

#[tauri::command]
pub async fn get_address_space(state: State<'_, AppState>) -> Result<AddressSpaceDto, String> {
    let folders = state.folders.read().unwrap();
    let nodes = state.nodes.read().unwrap();

    Ok(AddressSpaceDto {
        folders: folders.iter().map(|f| ServerFolderDto {
            node_id: f.node_id.clone(),
            display_name: f.display_name.clone(),
            parent_id: f.parent_id.clone(),
        }).collect(),
        nodes: nodes.iter().map(|n| ServerNodeDto {
            node_id: n.node_id.clone(),
            display_name: n.display_name.clone(),
            parent_id: n.parent_id.clone(),
            data_type: n.data_type.to_string(),
            writable: n.writable,
            simulation: serde_json::to_value(&n.simulation).unwrap_or_default(),
            current_value: n.current_value.clone(),
        }).collect(),
    })
}
