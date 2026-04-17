use rand::Rng;

use super::models::{DataType, SimulationMode};

/// Generate the next f64 value for a simulation mode.
/// Returns None for Static mode (no automatic generation).
pub fn generate_value(mode: &SimulationMode, elapsed_secs: f64, iteration: u64) -> Option<f64> {
    match mode {
        SimulationMode::Static { .. } => None,
        SimulationMode::Random { min, max, .. } => {
            Some(rand::thread_rng().gen_range(*min..=*max))
        }
        SimulationMode::Sine { amplitude, offset, period_ms, .. } => {
            let period_secs = *period_ms as f64 / 1000.0;
            Some(*offset + *amplitude * (2.0 * std::f64::consts::PI * elapsed_secs / period_secs).sin())
        }
        SimulationMode::Linear { start, step, min, max, mode, .. } => {
            let range = max - min;
            if range <= 0.0 {
                return Some(*start);
            }
            let raw = start + step * iteration as f64;
            match mode {
                super::models::LinearMode::Repeat => {
                    Some(min + (raw - min).rem_euclid(range))
                }
                super::models::LinearMode::Bounce => {
                    let pos = (raw - min) / range;
                    let cycle = pos.floor() as i64;
                    let frac = pos - pos.floor();
                    if cycle % 2 == 0 {
                        Some(min + frac * range)
                    } else {
                        Some(max - frac * range)
                    }
                }
            }
        }
        SimulationMode::Script { .. } => {
            // Phase 2: evalexpr integration
            Some(0.0)
        }
    }
}

/// Convert an f64 value to a string representation appropriate for the data type.
pub fn f64_to_string(value: f64, data_type: &DataType) -> String {
    match data_type {
        DataType::Boolean => if value > 0.5 { "true" } else { "false" }.to_string(),
        DataType::Int16 => (value.clamp(i16::MIN as f64, i16::MAX as f64) as i16).to_string(),
        DataType::Int32 => (value.clamp(i32::MIN as f64, i32::MAX as f64) as i32).to_string(),
        DataType::Int64 => (value.clamp(i64::MIN as f64, i64::MAX as f64) as i64).to_string(),
        DataType::UInt16 => (value.clamp(0.0, u16::MAX as f64) as u16).to_string(),
        DataType::UInt32 => (value.clamp(0.0, u32::MAX as f64) as u32).to_string(),
        DataType::UInt64 => (value.clamp(0.0, u64::MAX as f64) as u64).to_string(),
        DataType::Float => format!("{:.6}", value as f32),
        DataType::Double => format!("{:.6}", value),
        DataType::String => format!("{:.2}", value),
        DataType::DateTime | DataType::ByteString => format!("{:.2}", value),
    }
}
