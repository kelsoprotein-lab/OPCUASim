//! Method service helpers: read InputArguments / OutputArguments Property
//! nodes of a Method, then call the method.

use std::sync::Arc;

use opcua_client::Session;
use opcua_types::{
    Argument, AttributeId, BrowseDescription, BrowseDirection, BrowseResultMask,
    CallMethodRequest, DataValue, ExtensionObject, NodeClassMask, NodeId, NumericRange,
    QualifiedName, ReadValueId, ReferenceTypeId, StatusCode, TimestampsToReturn, Variant,
};

use crate::error::OpcUaSimError;

#[derive(Debug, Clone, Default)]
pub struct ArgumentInfo {
    pub name: String,
    pub data_type: String,
    pub data_type_id: NodeId,
    pub description: String,
}

#[derive(Debug, Clone, Default)]
pub struct MethodArgumentsInfo {
    pub inputs: Vec<ArgumentInfo>,
    pub outputs: Vec<ArgumentInfo>,
}

pub async fn read_method_arguments(
    session: &Arc<Session>,
    method_id: &NodeId,
) -> Result<MethodArgumentsInfo, OpcUaSimError> {
    let browse = vec![BrowseDescription {
        node_id: method_id.clone(),
        browse_direction: BrowseDirection::Forward,
        reference_type_id: ReferenceTypeId::HasProperty.into(),
        include_subtypes: true,
        node_class_mask: NodeClassMask::VARIABLE.bits(),
        result_mask: BrowseResultMask::All as u32,
    }];

    let results = session
        .browse(&browse, 0, None)
        .await
        .map_err(|e| OpcUaSimError::SubscriptionError(format!("Browse method props failed: {e}")))?;

    let mut input_id: Option<NodeId> = None;
    let mut output_id: Option<NodeId> = None;
    for r in results {
        if let Some(refs) = r.references {
            for rf in refs {
                let name = rf.browse_name.name.to_string();
                if name == "InputArguments" {
                    input_id = Some(rf.node_id.node_id.clone());
                } else if name == "OutputArguments" {
                    output_id = Some(rf.node_id.node_id.clone());
                }
            }
        }
    }

    let inputs = match input_id {
        Some(id) => decode_argument_array(session, &id).await?,
        None => Vec::new(),
    };
    let outputs = match output_id {
        Some(id) => decode_argument_array(session, &id).await?,
        None => Vec::new(),
    };

    Ok(MethodArgumentsInfo { inputs, outputs })
}

async fn decode_argument_array(
    session: &Arc<Session>,
    args_node_id: &NodeId,
) -> Result<Vec<ArgumentInfo>, OpcUaSimError> {
    let read_request = vec![ReadValueId {
        node_id: args_node_id.clone(),
        attribute_id: AttributeId::Value as u32,
        index_range: NumericRange::None,
        data_encoding: QualifiedName::null(),
    }];
    let dvs: Vec<DataValue> = session
        .read(&read_request, TimestampsToReturn::Neither, 0.0)
        .await
        .map_err(|e| OpcUaSimError::SubscriptionError(format!("Read args failed: {e}")))?;

    let dv = dvs.into_iter().next().ok_or_else(|| {
        OpcUaSimError::SubscriptionError("Read args returned no DataValue".into())
    })?;
    let value = dv.value.unwrap_or(Variant::Empty);

    let extension_objects: Vec<ExtensionObject> = match value {
        Variant::Array(arr) => arr
            .values
            .into_iter()
            .filter_map(|v| match v {
                Variant::ExtensionObject(eo) => Some(eo),
                _ => None,
            })
            .collect(),
        Variant::ExtensionObject(eo) => vec![eo],
        _ => return Ok(Vec::new()),
    };

    let mut out = Vec::with_capacity(extension_objects.len());
    for eo in extension_objects {
        if let Some(arg) = eo.into_inner_as::<Argument>() {
            out.push(ArgumentInfo {
                name: arg.name.to_string(),
                data_type_id: arg.data_type.clone(),
                data_type: data_type_label(&arg.data_type),
                description: arg.description.text.to_string(),
            });
        }
    }
    Ok(out)
}

fn data_type_label(id: &NodeId) -> String {
    use opcua_types::{DataTypeId, Identifier};
    if id.namespace == 0 {
        if let Identifier::Numeric(n) = &id.identifier {
            if let Ok(d) = DataTypeId::try_from(*n) {
                return format!("{d:?}");
            }
        }
    }
    format!("{id}")
}

#[derive(Debug, Clone)]
pub struct MethodCallOutcome {
    pub status: StatusCode,
    pub outputs: Vec<Variant>,
}

pub async fn call_method(
    session: &Arc<Session>,
    object_id: &NodeId,
    method_id: &NodeId,
    inputs: Vec<Variant>,
) -> Result<MethodCallOutcome, OpcUaSimError> {
    let req: CallMethodRequest = (object_id.clone(), method_id.clone(), Some(inputs)).into();
    let result = session
        .call_one(req)
        .await
        .map_err(|e| OpcUaSimError::ConnectionFailed(format!("Call failed: {e}")))?;
    Ok(MethodCallOutcome {
        status: result.status_code,
        outputs: result.output_arguments.unwrap_or_default(),
    })
}
