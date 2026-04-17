#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod backend;
mod events;
mod fonts;
mod model;
mod panels;
mod runtime;

use app::ServerApp;

fn main() -> eframe::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("OPCUAServer")
            .with_inner_size([1200.0, 760.0])
            .with_min_inner_size([960.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "OPCUAServer",
        options,
        Box::new(|cc| Ok(Box::new(ServerApp::new(cc)))),
    )
}
