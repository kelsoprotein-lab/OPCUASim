#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use opcuaegui_shared::settings;
use opcuamaster_egui::{app::MasterApp, APP_ID};

fn main() -> eframe::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let (w, h) = settings::load(APP_ID)
        .map(|s| (s.width, s.height))
        .unwrap_or((1280.0, 800.0));

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("OPCUAMaster")
            .with_inner_size([w, h])
            .with_min_inner_size([960.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "OPCUAMaster",
        options,
        Box::new(|cc| Ok(Box::new(MasterApp::new(cc)))),
    )
}
