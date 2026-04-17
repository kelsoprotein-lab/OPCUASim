#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod backend;
mod events;
mod fonts;
mod model;
mod panels;
mod runtime;
mod widgets;

use app::MasterApp;

fn main() -> eframe::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("OPCUAMaster")
            .with_inner_size([1280.0, 800.0])
            .with_min_inner_size([960.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "OPCUAMaster",
        options,
        Box::new(|cc| Ok(Box::new(MasterApp::new(cc)))),
    )
}
