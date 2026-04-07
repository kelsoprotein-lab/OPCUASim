use std::collections::HashMap;
use std::sync::RwLock;

pub struct AppState {
    pub connections: RwLock<HashMap<String, ()>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            connections: RwLock::new(HashMap::new()),
        }
    }
}
