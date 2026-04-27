use std::path::PathBuf;

use crate::events::{CertRoleDto, CertSummaryDto};
use crate::model::CertManagerState;

pub struct DialogActions {
    pub close: bool,
    pub move_to: Option<(PathBuf, CertRoleDto)>,
    pub delete: Option<PathBuf>,
    pub refresh: bool,
}

pub fn show(ctx: &egui::Context, state: &mut CertManagerState) -> DialogActions {
    let mut actions = DialogActions {
        close: false,
        move_to: None,
        delete: None,
        refresh: false,
    };

    egui::Window::new("证书管理")
        .collapsible(false)
        .resizable(true)
        .min_width(720.0)
        .default_width(900.0)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label("PKI 目录:");
                ui.code("./pki");
                if ui.button("🔄 刷新").clicked() {
                    actions.refresh = true;
                }
            });

            if let Some(err) = &state.error {
                ui.colored_label(egui::Color32::LIGHT_RED, err);
            }

            ui.separator();

            // Snapshot to avoid double-borrow of `state` inside the column closures.
            let trusted = state.trusted.clone();
            let rejected = state.rejected.clone();
            let selected = state.selected_path.clone();

            ui.columns(2, |cols| {
                let pane0 = render_pane(&mut cols[0], "Trusted", &trusted, selected.as_ref());
                let pane1 = render_pane(&mut cols[1], "Rejected", &rejected, selected.as_ref());
                if let Some(p) = pane0.selected.or(pane1.selected) {
                    state.selected_path = Some(p);
                }
                if let Some((p, target)) = pane0.move_to.or(pane1.move_to) {
                    actions.move_to = Some((p, target));
                }
                if let Some(p) = pane0.delete.or(pane1.delete) {
                    actions.delete = Some(p);
                }
            });

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("关闭").clicked() {
                    actions.close = true;
                }
            });
        });

    actions
}

struct PaneActions {
    selected: Option<PathBuf>,
    move_to: Option<(PathBuf, CertRoleDto)>,
    delete: Option<PathBuf>,
}

fn render_pane(
    ui: &mut egui::Ui,
    title: &str,
    certs: &[CertSummaryDto],
    selected: Option<&PathBuf>,
) -> PaneActions {
    let mut out = PaneActions {
        selected: None,
        move_to: None,
        delete: None,
    };
    ui.heading(title);
    ui.label(format!("{} 证书", certs.len()));
    egui::ScrollArea::vertical()
        .id_salt(format!("scroll_{title}"))
        .max_height(380.0)
        .show(ui, |ui| {
            for c in certs {
                let is_sel = selected.map(|p| p == &c.path).unwrap_or(false);
                let resp = ui.selectable_label(is_sel, format!("📄 {}", c.subject_cn));
                if resp.clicked() {
                    out.selected = Some(c.path.clone());
                }
                if is_sel {
                    egui::Frame::group(ui.style()).show(ui, |ui| {
                        ui.label(format!("文件: {}", c.file_name));
                        ui.label(format!("Issuer: {}", c.issuer_cn));
                        ui.label(format!("Thumbprint: {}", c.thumbprint));
                        ui.label(format!("有效期: {} → {}", c.valid_from, c.valid_to));
                        ui.horizontal(|ui| {
                            let (target_label, target) = match c.role {
                                CertRoleDto::Trusted => ("→ 拒绝", CertRoleDto::Rejected),
                                CertRoleDto::Rejected => ("→ 信任", CertRoleDto::Trusted),
                            };
                            if ui.button(target_label).clicked() {
                                out.move_to = Some((c.path.clone(), target));
                            }
                            if ui.button("🗑 删除").clicked() {
                                out.delete = Some(c.path.clone());
                            }
                        });
                    });
                }
            }
        });
    out
}
