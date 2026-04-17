use opcua_server::address_space::{AddressSpace, VariableBuilder};
use opcua_types::{
    LocalizedText, NodeId, QualifiedName, Variant, UAString,
};

use super::models::{DataType, ServerFolder, ServerNode, SimulationMode};
use crate::error::OpcUaSimError;

/// Convert our DataType enum to the OPC UA DataTypeId NodeId.
fn data_type_to_node_id(dt: &DataType) -> NodeId {
    NodeId::new(0, dt.type_id())
}

/// Convert a string value to a Variant for the given data type.
pub fn string_to_variant(value: &str, data_type: &DataType) -> Variant {
    match data_type {
        DataType::Boolean => Variant::Boolean(value.eq_ignore_ascii_case("true") || value == "1"),
        DataType::Int16 => value.parse::<i16>().map(Variant::Int16).unwrap_or(Variant::Int16(0)),
        DataType::Int32 => value.parse::<i32>().map(Variant::Int32).unwrap_or(Variant::Int32(0)),
        DataType::Int64 => value.parse::<i64>().map(Variant::Int64).unwrap_or(Variant::Int64(0)),
        DataType::UInt16 => value.parse::<u16>().map(Variant::UInt16).unwrap_or(Variant::UInt16(0)),
        DataType::UInt32 => value.parse::<u32>().map(Variant::UInt32).unwrap_or(Variant::UInt32(0)),
        DataType::UInt64 => value.parse::<u64>().map(Variant::UInt64).unwrap_or(Variant::UInt64(0)),
        DataType::Float => value.parse::<f32>().map(Variant::Float).unwrap_or(Variant::Float(0.0)),
        DataType::Double => value.parse::<f64>().map(Variant::Double).unwrap_or(Variant::Double(0.0)),
        DataType::String => Variant::String(UAString::from(value)),
        DataType::DateTime => Variant::String(UAString::from(value)),
        DataType::ByteString => Variant::String(UAString::from(value)),
    }
}

/// Convert an f64 value to a Variant for the given data type.
pub fn f64_to_variant(value: f64, data_type: &DataType) -> Variant {
    match data_type {
        DataType::Boolean => Variant::Boolean(value > 0.5),
        DataType::Int16 => Variant::Int16(value.clamp(i16::MIN as f64, i16::MAX as f64) as i16),
        DataType::Int32 => Variant::Int32(value.clamp(i32::MIN as f64, i32::MAX as f64) as i32),
        DataType::Int64 => Variant::Int64(value.clamp(i64::MIN as f64, i64::MAX as f64) as i64),
        DataType::UInt16 => Variant::UInt16(value.clamp(0.0, u16::MAX as f64) as u16),
        DataType::UInt32 => Variant::UInt32(value.clamp(0.0, u32::MAX as f64) as u32),
        DataType::UInt64 => Variant::UInt64(value.clamp(0.0, u64::MAX as f64) as u64),
        DataType::Float => Variant::Float(value as f32),
        DataType::Double => Variant::Double(value),
        DataType::String => Variant::String(UAString::from(format!("{:.2}", value))),
        DataType::DateTime => Variant::Double(value),
        DataType::ByteString => Variant::Double(value),
    }
}

/// Parse a node_id string to OPC UA NodeId.
pub fn parse_node_id(node_id_str: &str) -> Result<NodeId, OpcUaSimError> {
    node_id_str.parse::<NodeId>()
        .map_err(|e| OpcUaSimError::ServerError(format!("Invalid node id '{}': {}", node_id_str, e)))
}

/// Populate an address space with folders and variable nodes.
pub fn populate_address_space(
    address_space: &mut AddressSpace,
    namespace_index: u16,
    folders: &[ServerFolder],
    nodes: &[ServerNode],
) {
    // Add folders
    for folder in folders {
        let node_id = make_node_id(namespace_index, &folder.node_id);
        let parent_id = make_parent_id(namespace_index, &folder.parent_id);
        address_space.add_folder(
            &node_id,
            QualifiedName::new(namespace_index, &folder.display_name),
            LocalizedText::new("", &folder.display_name),
            &parent_id,
        );
    }

    // Add variable nodes
    for node in nodes {
        add_variable_node(address_space, namespace_index, node);
    }
}

/// Add a single variable node to the address space.
pub fn add_variable_node(
    address_space: &mut AddressSpace,
    namespace_index: u16,
    node: &ServerNode,
) -> bool {
    let node_id = make_node_id(namespace_index, &node.node_id);
    let parent_id = make_parent_id(namespace_index, &node.parent_id);
    let dt_node_id = data_type_to_node_id(&node.data_type);

    let initial_value = match &node.simulation {
        SimulationMode::Static { value } => string_to_variant(value, &node.data_type),
        _ => string_to_variant("0", &node.data_type),
    };

    let mut builder = VariableBuilder::new(&node_id, &node.display_name, &node.display_name)
        .data_type(dt_node_id)
        .value(initial_value)
        .organized_by(parent_id);

    if node.writable {
        builder = builder.writable();
    }

    builder.insert(address_space)
}

/// Remove a node from the address space.
pub fn remove_node(address_space: &mut AddressSpace, namespace_index: u16, node_id_str: &str) -> bool {
    let node_id = make_node_id(namespace_index, node_id_str);
    address_space.delete(&node_id, true).is_some()
}

/// Create an OPC UA NodeId from a string, handling namespace prefixed formats.
fn make_node_id(namespace_index: u16, id_str: &str) -> NodeId {
    // If it already has a namespace prefix (ns=X;), parse directly
    if id_str.starts_with("ns=") || id_str.starts_with("i=") || id_str.starts_with("s=") {
        id_str.parse::<NodeId>().unwrap_or_else(|_| NodeId::new(namespace_index, id_str))
    } else {
        NodeId::new(namespace_index, id_str)
    }
}

/// Resolve parent_id: "i=85" is the Objects folder (root).
fn make_parent_id(namespace_index: u16, parent_id: &str) -> NodeId {
    if parent_id == "i=85" || parent_id.is_empty() {
        NodeId::objects_folder_id()
    } else {
        make_node_id(namespace_index, parent_id)
    }
}
