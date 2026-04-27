//! Historical data access wrapper around Session::history_read with
//! ReadRawModifiedDetails. Loops continuation points up to max_values.

use std::sync::Arc;

use opcua_client::{HistoryReadAction, Session};
use opcua_types::{
    ContinuationPoint, DataValue, DateTime, HistoryData, HistoryReadResult, HistoryReadValueId,
    NodeId, NumericRange, QualifiedName, ReadRawModifiedDetails, TimestampsToReturn,
};

use crate::error::OpcUaSimError;

#[derive(Debug, Clone)]
pub struct HistoryDataPoint {
    pub source_timestamp: String,
    pub server_timestamp: String,
    pub value: String,
    pub numeric: Option<f64>,
    pub status: String,
}

pub async fn history_read_raw(
    session: &Arc<Session>,
    node_id: &NodeId,
    start: DateTime,
    end: DateTime,
    max_values: u32,
    return_bounds: bool,
) -> Result<Vec<HistoryDataPoint>, OpcUaSimError> {
    let mut out: Vec<HistoryDataPoint> = Vec::new();
    let mut continuation_point = ContinuationPoint::null();

    loop {
        let action = HistoryReadAction::ReadRawModifiedDetails(ReadRawModifiedDetails {
            is_read_modified: false,
            start_time: start,
            end_time: end,
            num_values_per_node: max_values.saturating_sub(out.len() as u32),
            return_bounds,
        });
        let nodes_to_read = vec![HistoryReadValueId {
            node_id: node_id.clone(),
            index_range: NumericRange::None,
            data_encoding: QualifiedName::null(),
            continuation_point: continuation_point.clone(),
        }];

        let results: Vec<HistoryReadResult> = session
            .history_read(action, TimestampsToReturn::Both, false, &nodes_to_read)
            .await
            .map_err(|e| OpcUaSimError::ConnectionFailed(format!("history_read failed: {e}")))?;

        let result = results
            .into_iter()
            .next()
            .ok_or_else(|| OpcUaSimError::ConnectionFailed("history_read empty result".into()))?;

        if !result.status_code.is_good() {
            return Err(OpcUaSimError::ConnectionFailed(format!(
                "history_read status: {}",
                result.status_code
            )));
        }

        let history_data: Option<Box<HistoryData>> =
            result.history_data.into_inner_as::<HistoryData>();
        let dvs: Vec<DataValue> = history_data
            .and_then(|hd| hd.data_values)
            .unwrap_or_default();

        for dv in dvs {
            out.push(map_data_value(dv));
            if out.len() as u32 >= max_values {
                break;
            }
        }

        if out.len() as u32 >= max_values || result.continuation_point.is_null() {
            break;
        }
        continuation_point = result.continuation_point;
    }

    Ok(out)
}

fn map_data_value(dv: DataValue) -> HistoryDataPoint {
    let value_str = dv
        .value
        .as_ref()
        .map(|v| format!("{v}"))
        .unwrap_or_default();
    let numeric = dv.value.as_ref().and_then(variant_to_f64);
    let status = dv
        .status
        .map(|s| format!("{s}"))
        .unwrap_or_else(|| "Good".to_string());
    let source_timestamp = dv
        .source_timestamp
        .map(|t| t.to_string())
        .unwrap_or_default();
    let server_timestamp = dv
        .server_timestamp
        .map(|t| t.to_string())
        .unwrap_or_default();
    HistoryDataPoint {
        source_timestamp,
        server_timestamp,
        value: value_str,
        numeric,
        status,
    }
}

fn variant_to_f64(v: &opcua_types::Variant) -> Option<f64> {
    use opcua_types::Variant;
    match v {
        Variant::Boolean(b) => Some(if *b { 1.0 } else { 0.0 }),
        Variant::SByte(x) => Some(*x as f64),
        Variant::Byte(x) => Some(*x as f64),
        Variant::Int16(x) => Some(*x as f64),
        Variant::UInt16(x) => Some(*x as f64),
        Variant::Int32(x) => Some(*x as f64),
        Variant::UInt32(x) => Some(*x as f64),
        Variant::Int64(x) => Some(*x as f64),
        Variant::UInt64(x) => Some(*x as f64),
        Variant::Float(x) => Some(*x as f64),
        Variant::Double(x) => Some(*x),
        _ => None,
    }
}
