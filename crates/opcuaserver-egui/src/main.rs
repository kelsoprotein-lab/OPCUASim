#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod backend;
mod events;
mod fonts;
mod model;
mod panels;
mod runtime;
mod settings;

use app::ServerApp;

pub const APP_ID: &str = "opcuaserver";

fn main() -> eframe::Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let (w, h) = settings::load(APP_ID)
        .map(|s| (s.width, s.height))
        .unwrap_or((1200.0, 760.0));

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("OPCUAServer")
            .with_inner_size([w, h])
            .with_min_inner_size([960.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "OPCUAServer",
        options,
        Box::new(|cc| Ok(Box::new(ServerApp::new(cc)))),
    )
}
