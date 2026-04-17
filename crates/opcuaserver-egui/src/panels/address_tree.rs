use std::collections::BTreeMap;

use crate::events::UiCommand;
use crate::model::AppModel;
use crate::runtime::BackendHandle;

pub fn show(ui: &mut egui::Ui, model: &mut AppModel, backend: &BackendHandle) {
    ui.heading("地址空间");
    ui.separator();

    // Build parent -> children index
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
            egui::CollapsingHeader::new("📁 Objects")
                .id_salt("addr_root")
                .default_open(true)
                .show(ui, |ui| {
                    render_children(ui, &root, &children, model, backend);
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
                let label = format!("📁 {}", display_name);
                let resp = egui::CollapsingHeader::new(label)
                    .id_salt(("folder", node_id))
                    .default_open(false)
                    .show(ui, |ui| {
                        render_children(ui, node_id, children, model, backend);
                    });
                resp.header_response.context_menu(|ui| {
                    if ui.button("🗑 删除").clicked() {
                        backend.send(UiCommand::RemoveNode(node_id.clone()));
                        ui.close();
                    }
                });
            }
            Child::Node {
                node_id,
                display_name,
            } => {
                let label = format!("📊 {}", display_name);
                let selected = model.selected_node_id.as_deref() == Some(node_id);
                let resp = ui.selectable_label(selected, label);
                if resp.clicked() {
                    model.selected_node_id = Some(node_id.clone());
                }
                resp.context_menu(|ui| {
                    if ui.button("🗑 删除").clicked() {
                        backend.send(UiCommand::RemoveNode(node_id.clone()));
                        ui.close();
                    }
                });
            }
        }
    }
}
