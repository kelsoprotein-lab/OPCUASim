use std::collections::BTreeMap;

use opcuaegui_shared::theme;
use opcuaegui_shared::widgets::empty_state;

use crate::events::UiCommand;
use crate::model::AppModel;
use crate::runtime::BackendHandle;

pub fn show(ui: &mut egui::Ui, model: &mut AppModel, backend: &BackendHandle) {
    ui.label(
        egui::RichText::new("ADDRESS SPACE")
            .strong()
            .small()
            .color(theme::TEXT_MUTED),
    );
    ui.separator();

    if model.address_space.folders.is_empty() && model.address_space.nodes.is_empty() {
        empty_state(
            ui,
            "🗂",
            "地址空间为空",
            Some("使用顶部 📁 / 📊 添加文件夹与变量"),
        );
        return;
    }

    // parent_id -> child list
    let mut children: BTreeMap<String, Vec<Child>> = BTreeMap::new();
    for f in &model.address_space.folders {
        children
            .entry(f.parent_id.clone())
            .or_default()
            .push(Child::Folder {
                node_id: f.node_id.clone(),
                display_name: f.display_name.clone(),
            });
    }
    for n in &model.address_space.nodes {
        children
            .entry(n.parent_id.clone())
            .or_default()
            .push(Child::Node {
                node_id: n.node_id.clone(),
                display_name: n.display_name.clone(),
            });
    }

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let root = "Objects".to_string();
            let resp = egui::CollapsingHeader::new(
                egui::RichText::new("📁 Objects")
                    .color(theme::TEXT_PRIMARY)
                    .strong(),
            )
            .id_salt("addr_root")
            .default_open(true)
            .show(ui, |ui| {
                render_children(ui, &root, &children, model, backend);
            });
            resp.header_response.context_menu(|ui| {
                add_subfolder_menu(ui, "Objects", model, backend);
            });
        });
}

enum Child {
    Folder { node_id: String, display_name: String },
    Node { node_id: String, display_name: String },
}

fn render_children(
    ui: &mut egui::Ui,
    parent: &str,
    children: &BTreeMap<String, Vec<Child>>,
    model: &mut AppModel,
    backend: &BackendHandle,
) {
    let Some(list) = children.get(parent) else {
        return;
    };
    for ch in list {
        match ch {
            Child::Folder {
                node_id,
                display_name,
            } => {
                let label = egui::RichText::new(format!("📁 {}", display_name))
                    .color(theme::TEXT_PRIMARY);
                let resp = egui::CollapsingHeader::new(label)
                    .id_salt(("folder", node_id))
                    .default_open(false)
                    .show(ui, |ui| {
                        render_children(ui, node_id, children, model, backend);
                    });
                let nid = node_id.clone();
                resp.header_response.context_menu(|ui| {
                    add_subfolder_menu(ui, &nid, model, backend);
                    if ui.button("🗑 删除文件夹").clicked() {
                        backend.send(UiCommand::RemoveNode(nid.clone()));
                        ui.close();
                    }
                });
            }
            Child::Node {
                node_id,
                display_name,
            } => {
                let label = egui::RichText::new(format!("📊 {}", display_name))
                    .color(theme::TEXT_PRIMARY);
                let selected = model.selected_node_id.as_deref() == Some(node_id);
                let resp = ui.selectable_label(selected, label);
                if resp.clicked() {
                    model.selected_node_id = Some(node_id.clone());
                }
                let nid = node_id.clone();
                resp.context_menu(|ui| {
                    if ui.button("🗑 删除节点").clicked() {
                        backend.send(UiCommand::RemoveNode(nid));
                        ui.close();
                    }
                });
            }
        }
    }
}

/// Right-click menu for "add subfolder under <parent>".
fn add_subfolder_menu(
    ui: &mut egui::Ui,
    parent_id: &str,
    model: &mut AppModel,
    backend: &BackendHandle,
) {
    ui.label(
        egui::RichText::new("新建子文件夹")
            .small()
            .color(theme::TEXT_MUTED),
    );
    let buf = model
        .subfolder_inputs
        .entry(parent_id.to_string())
        .or_default();
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(buf)
                .hint_text("display name")
                .desired_width(140.0),
        );
        let enabled = !buf.trim().is_empty();
        ui.add_enabled_ui(enabled, |ui| {
            if ui.button("➕").clicked() {
                let name = buf.trim().to_string();
                let node_id = format!("ns=2;s={}_{}", parent_id.replace(':', "_"), name.replace(' ', "_"));
                backend.send(UiCommand::AddFolder {
                    node_id,
                    display_name: name,
                    parent_id: parent_id.to_string(),
                });
                buf.clear();
                ui.close();
            }
        });
    });
}
