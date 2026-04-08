use std::sync::Arc;

use opcua_client::Session;
use opcua_types::{
    AttributeId, BrowseDescription, BrowseDirection, NodeClass, NodeId,
    ReadValueId, ReferenceTypeId, TimestampsToReturn,
};

use crate::error::OpcUaSimError;
use crate::node::{BrowseResultItem, NodeAttributes};

/// Browse children of a node. Pass None for node_id to browse from root (Objects folder).
pub async fn browse_node(
    session: &Arc<Session>,
    node_id: Option<&str>,
) -> Result<Vec<BrowseResultItem>, OpcUaSimError> {
    let target_node = match node_id {
        Some(id) => id.parse::<NodeId>()
            .map_err(|e| OpcUaSimError::BrowseError(format!("Invalid node id '{}': {}", id, e)))?,
        None => NodeId::objects_folder_id(),
    };

    let browse_desc = vec![BrowseDescription {
        node_id: target_node,
        browse_direction: BrowseDirection::Forward,
        reference_type_id: ReferenceTypeId::HierarchicalReferences.into(),
        include_subtypes: true,
        node_class_mask: 0,
        result_mask: 0x3F, // All fields
    }];

    let results = session
        .browse(&browse_desc, 0, None)
        .await
        .map_err(|e| OpcUaSimError::BrowseError(format!("Browse failed: {}", e)))?;

    let mut items = Vec::new();
    for result in results {
        if let Some(refs) = result.references {
            for r in refs {
                let node_class_str = match r.node_class {
                    NodeClass::Object => "Object",
                    NodeClass::Variable => "Variable",
                    NodeClass::Method => "Method",
                    NodeClass::ObjectType => "ObjectType",
                    NodeClass::VariableType => "VariableType",
                    NodeClass::ReferenceType => "ReferenceType",
                    NodeClass::DataType => "DataType",
                    NodeClass::View => "View",
                    _ => "Unspecified",
                };

                // An Object or ObjectType likely has children; Variable typically doesn't
                let has_children = matches!(r.node_class, NodeClass::Object | NodeClass::ObjectType | NodeClass::View);

                items.push(BrowseResultItem {
                    node_id: r.node_id.node_id.to_string(),
                    display_name: r.display_name.text.value().clone().unwrap_or_default(),
                    node_class: node_class_str.to_string(),
                    data_type: None, // Would require additional read to determine
                    has_children,
                });
            }
        }
    }

    Ok(items)
}

/// Read detailed attributes of a specific node.
pub async fn read_node_attributes(
    session: &Arc<Session>,
    node_id: &str,
) -> Result<NodeAttributes, OpcUaSimError> {
    let target_node = node_id.parse::<NodeId>()
        .map_err(|e| OpcUaSimError::ReadError(format!("Invalid node id '{}': {}", node_id, e)))?;

    // Read multiple attributes: DisplayName, Description, DataType, Value, AccessLevel
    let nodes_to_read = vec![
        ReadValueId::new(target_node.clone(), AttributeId::DisplayName),
        ReadValueId::new(target_node.clone(), AttributeId::Description),
        ReadValueId::new(target_node.clone(), AttributeId::DataType),
        ReadValueId::new(target_node.clone(), AttributeId::Value),
        ReadValueId::new(target_node.clone(), AttributeId::AccessLevel),
    ];

    let values = session
        .read(&nodes_to_read, TimestampsToReturn::Both, 0.0)
        .await
        .map_err(|e| OpcUaSimError::ReadError(format!("Read failed: {}", e)))?;

    let display_name = values.first()
        .and_then(|dv| dv.value.as_ref())
        .map(|v| format!("{}", v))
        .unwrap_or_else(|| node_id.to_string());

    let description = values.get(1)
        .and_then(|dv| dv.value.as_ref())
        .map(|v| format!("{}", v))
        .unwrap_or_default();

    let data_type = values.get(2)
        .and_then(|dv| dv.value.as_ref())
        .map(|v| format!("{}", v))
        .unwrap_or_else(|| "Unknown".to_string());

    let value_dv = values.get(3);
    let value = value_dv
        .and_then(|dv| dv.value.as_ref())
        .map(|v| format!("{}", v));

    let quality = value_dv
        .and_then(|dv| dv.status.as_ref())
        .map(|s| format!("{}", s));

    let timestamp = value_dv
        .and_then(|dv| dv.source_timestamp.as_ref())
        .map(|t| t.to_string());

    let access_level = values.get(4)
        .and_then(|dv| dv.value.as_ref())
        .map(|v| format!("{}", v))
        .unwrap_or_else(|| "0".to_string());

    Ok(NodeAttributes {
        node_id: node_id.to_string(),
        display_name,
        description,
        data_type,
        access_level,
        value,
        quality,
        timestamp,
    })
}
