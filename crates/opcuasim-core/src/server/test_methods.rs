//! Test-only address-space helpers: register a callable Demo.Echo method
//! that echoes a single String input. Not used in production startup.

use opcua_nodes::MethodBuilder;
use opcua_types::{
    Argument, DataTypeId, LocalizedText, NodeId, ObjectId, StatusCode, UAString, Variant,
};

use crate::error::OpcUaSimError;
use crate::server::server::OpcUaServer;

pub async fn register_demo_echo_method(server: &OpcUaServer) -> Result<NodeId, OpcUaSimError> {
    let nm = server
        .node_manager()
        .await
        .ok_or_else(|| OpcUaSimError::ServerError("Server not started".into()))?;
    let ns = server.namespace_index().await;

    let parent_node_id: NodeId = ObjectId::ObjectsFolder.into();
    let method_id = NodeId::new(ns, "Demo.Echo");
    let in_args_id = NodeId::new(ns, "Demo.Echo.InputArguments");
    let out_args_id = NodeId::new(ns, "Demo.Echo.OutputArguments");

    {
        let mut addr = nm.address_space().write();
        let _ = MethodBuilder::new(&method_id, "Echo", "Echo")
            .component_of(parent_node_id)
            .executable(true)
            .user_executable(true)
            .input_args(
                &mut *addr,
                &in_args_id,
                &[Argument {
                    name: UAString::from("input"),
                    data_type: DataTypeId::String.into(),
                    value_rank: -1,
                    array_dimensions: None,
                    description: LocalizedText::from("Echoed back as output"),
                }],
            )
            .output_args(
                &mut *addr,
                &out_args_id,
                &[Argument {
                    name: UAString::from("output"),
                    data_type: DataTypeId::String.into(),
                    value_rank: -1,
                    array_dimensions: None,
                    description: LocalizedText::from("Same string as input"),
                }],
            )
            .insert(&mut *addr);
    }

    nm.inner()
        .add_method_callback(method_id.clone(), |inputs: &[Variant]| {
            match inputs.first() {
                Some(Variant::String(s)) => Ok(vec![Variant::String(s.clone())]),
                _ => Err(StatusCode::BadInvalidArgument),
            }
        });

    Ok(method_id)
}
