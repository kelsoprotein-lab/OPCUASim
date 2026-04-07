use crate::error::OpcUaSimError;
use crate::node::{BrowseResultItem, NodeAttributes};

/// Browse children of a node. Pass None for node_id to browse from root (Objects folder).
pub async fn browse_node(
    _endpoint_url: &str,
    _node_id: Option<&str>,
) -> Result<Vec<BrowseResultItem>, OpcUaSimError> {
    // TODO: Task 8 will implement actual async-opcua browsing.
    Ok(vec![])
}

/// Read detailed attributes of a specific node.
pub async fn read_node_attributes(
    _endpoint_url: &str,
    _node_id: &str,
) -> Result<NodeAttributes, OpcUaSimError> {
    // TODO: Task 8 will implement actual async-opcua attribute reading.
    Err(OpcUaSimError::ReadError("Not yet implemented".to_string()))
}
