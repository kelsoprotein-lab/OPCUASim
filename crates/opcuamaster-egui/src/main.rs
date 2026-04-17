#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod backend;
mod events;
mod fonts;
mod model;
mod panels;
mod runtime;
mod settings;
mod widgets;

use app::MasterApp;

const APP_ID: &str = "opcuamaster";

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
