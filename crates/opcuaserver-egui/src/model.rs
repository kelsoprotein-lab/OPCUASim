use std::collections::HashMap;

use opcuasim_core::server::models::{DataType, ServerConfig, SimulationMode};

use crate::events::{AddressSpaceDto, ServerStatus};

pub struct AppModel {
    pub status: ServerStatus,
    pub address_space: AddressSpaceDto,
    pub config: ServerConfig,
    pub selected_node_id: Option<String>,
    pub current_values: HashMap<String, String>, // node_id -> latest value
    pub last_sim_seq: u64,
    pub add_node_form: AddNodeForm,
    pub new_folder_name: String,
    pub toasts: Vec<Toast>,
}

impl Default for AppModel {
    fn default() -> Self {
        Self {
            status: ServerStatus {
                state: "Stopped".into(),
                node_count: 0,
                folder_count: 0,
                endpoint_url: "opc.tcp://0.0.0.0:4840".into(),
            },
            address_space: AddressSpaceDto::default(),
            config: ServerConfig::default(),
            selected_node_id: None,
            current_values: HashMap::new(),
            last_sim_seq: 0,
            add_node_form: AddNodeForm::default(),
            new_folder_name: String::new(),
            toasts: Vec::new(),
        }
    }
}

impl AppModel {
    pub fn push_toast(&mut self, level: crate::events::ToastLevel, msg: impl Into<String>) {
        self.toasts.push(Toast {
            level,
            message: msg.into(),
            created_at: std::time::Instant::now(),
        });
    }

    pub fn selected_node(&self) -> Option<&crate::events::NodeRow> {
        let id = self.selected_node_id.as_deref()?;
        self.address_space.nodes.iter().find(|n| n.node_id == id)
    }
}

pub struct AddNodeForm {
    pub node_id: String,
    pub display_name: String,
    pub parent_id: String,
    pub data_type: DataType,
    pub writable: bool,
    pub sim_kind: SimKind,
    pub static_value: String,
    pub random_min: f64,
    pub random_max: f64,
    pub random_interval_ms: u64,
    pub sine_amplitude: f64,
    pub sine_offset: f64,
    pub sine_period_ms: u64,
    pub sine_interval_ms: u64,
    pub linear_start: f64,
    pub linear_step: f64,
    pub linear_min: f64,
    pub linear_max: f64,
    pub linear_interval_ms: u64,
    pub linear_bounce: bool,
    pub script_expr: String,
    pub script_interval_ms: u64,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum SimKind {
    Static,
    Random,
    Sine,
    Linear,
    Script,
}

impl Default for AddNodeForm {
    fn default() -> Self {
        Self {
            node_id: String::new(),
            display_name: String::from("NewVar"),
            parent_id: String::from("Objects"),
            data_type: DataType::Double,
            writable: false,
            sim_kind: SimKind::Random,
            static_value: String::from("0"),
            random_min: 0.0,
            random_max: 100.0,
            random_interval_ms: 1000,
            sine_amplitude: 1.0,
            sine_offset: 0.0,
            sine_period_ms: 10_000,
            sine_interval_ms: 1000,
            linear_start: 0.0,
            linear_step: 1.0,
            linear_min: 0.0,
            linear_max: 100.0,
            linear_interval_ms: 1000,
            linear_bounce: false,
            script_expr: String::from("t * 0.1"),
            script_interval_ms: 1000,
        }
    }
}

impl AddNodeForm {
    pub fn build_simulation(&self) -> SimulationMode {
        use opcuasim_core::server::models::LinearMode;
        match self.sim_kind {
            SimKind::Static => SimulationMode::Static {
                value: self.static_value.clone(),
            },
            SimKind::Random => SimulationMode::Random {
                min: self.random_min,
                max: self.random_max,
                interval_ms: self.random_interval_ms,
            },
            SimKind::Sine => SimulationMode::Sine {
                amplitude: self.sine_amplitude,
                offset: self.sine_offset,
                period_ms: self.sine_period_ms,
                interval_ms: self.sine_interval_ms,
            },
            SimKind::Linear => SimulationMode::Linear {
                start: self.linear_start,
                step: self.linear_step,
                min: self.linear_min,
                max: self.linear_max,
                mode: if self.linear_bounce {
                    LinearMode::Bounce
                } else {
                    LinearMode::Repeat
                },
                interval_ms: self.linear_interval_ms,
            },
            SimKind::Script => SimulationMode::Script {
                expression: self.script_expr.clone(),
                interval_ms: self.script_interval_ms,
            },
        }
    }
}

pub struct Toast {
    pub level: crate::events::ToastLevel,
    pub message: String,
    pub created_at: std::time::Instant,
}
