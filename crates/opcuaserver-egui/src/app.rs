use tokio::sync::mpsc::UnboundedReceiver;

use crate::events::{BackendEvent, UiCommand};
use crate::model::AppModel;
use crate::panels::{address_tree, node_table, property_editor, status_bar, toolbar};
use crate::runtime::BackendHandle;

pub struct ServerApp {
    backend: BackendHandle,
    event_rx: UnboundedReceiver<BackendEvent>,
    model: AppModel,
}

impl ServerApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        crate::fonts::install_cjk_fonts(&cc.egui_ctx);
        let (backend, event_rx) = BackendHandle::new(cc.egui_ctx.clone());
        backend.send(UiCommand::RefreshAddressSpace);
        backend.send(UiCommand::RefreshStatus);
        Self {
            backend,
            event_rx,
            model: AppModel::default(),
        }
    }

    fn drain_events(&mut self) {
        while let Ok(ev) = self.event_rx.try_recv() {
            match ev {
                BackendEvent::Status(s) => {
                    self.model.status = s;
                }
                BackendEvent::AddressSpace(dto) => {
                    self.model.address_space = dto;
                }
                BackendEvent::SimValues { seq, values } => {
                    for (nid, val) in values {
                        self.model.current_values.insert(nid, val);
                    }
                    self.model.last_sim_seq = seq;
                }
                BackendEvent::Config(c) => {
                    self.model.config = c;
                }
                BackendEvent::Toast { level, message } => {
                    self.model.push_toast(level, message);
                }
            }
        }
    }

    fn render_toasts(&mut self, ctx: &egui::Context) {
        let now = std::time::Instant::now();
        self.model
            .toasts
            .retain(|t| now.duration_since(t.created_at).as_secs() < 4);
        if self.model.toasts.is_empty() {
            return;
        }
        egui::Area::new("toasts".into())
            .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-16.0, -40.0))
            .show(ctx, |ui| {
                for t in &self.model.toasts {
                    let color = match t.level {
                        crate::events::ToastLevel::Info => egui::Color32::LIGHT_BLUE,
                        crate::events::ToastLevel::Error => egui::Color32::LIGHT_RED,
                    };
                    egui::Frame::popup(ui.style())
                        .fill(egui::Color32::from_black_alpha(230))
                        .show(ui, |ui| {
                            ui.colored_label(color, &t.message);
                        });
                }
            });
    }
}

impl eframe::App for ServerApp {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.drain_events();

        egui::Panel::top("toolbar")
            .default_size(72.0)
            .show_inside(ui, |ui| {
                toolbar::show(ui, &mut self.model, &self.backend);
            });

        egui::Panel::bottom("status")
            .default_size(28.0)
            .resizable(false)
            .show_inside(ui, |ui| {
                status_bar::show(ui, &self.model);
            });

        egui::Panel::left("address_tree")
            .resizable(true)
            .default_size(260.0)
            .min_size(180.0)
            .show_inside(ui, |ui| {
                address_tree::show(ui, &mut self.model, &self.backend);
            });

        egui::Panel::right("property")
            .resizable(true)
            .default_size(320.0)
            .min_size(240.0)
            .show_inside(ui, |ui| {
                property_editor::show(ui, &mut self.model, &self.backend);
            });

        egui::CentralPanel::default().show_inside(ui, |ui| {
            node_table::show(ui, &mut self.model);
        });

        self.render_toasts(ui.ctx());
    }
}
