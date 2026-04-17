use crate::events::{MonitoredNodeReq, UiCommand};
use crate::model::{AppModel, BrowseNodeState};
use crate::runtime::BackendHandle;

pub fn show(ctx: &egui::Context, model: &mut AppModel, backend: &BackendHandle) {
    if !model.browse.open {
        return;
    }
    let Some(conn_id) = model.browse.conn_id.clone() else {
        return;
    };

    let mut open = model.browse.open;
    egui::Window::new("浏览节点")
        .collapsible(true)
        .resizable(true)
        .default_size(egui::vec2(620.0, 560.0))
        .open(&mut open)
        .show(ctx, |ui| {
            render_controls(ui, model);
            ui.separator();
            render_tree(ui, model, backend, &conn_id);
            ui.separator();
            render_footer(ui, model, backend, &conn_id);
        });
    model.browse.open = open;
}

fn render_controls(ui: &mut egui::Ui, model: &mut AppModel) {
    ui.horizontal(|ui| {
        ui.label("模式:");
        egui::ComboBox::from_id_salt("browse_access_mode")
            .selected_text(&model.browse.access_mode)
            .show_ui(ui, |ui| {
                ui.selectable_value(
                    &mut model.browse.access_mode,
                    "Subscription".into(),
                    "Subscription",
                );
                ui.selectable_value(
                    &mut model.browse.access_mode,
                    "Polling".into(),
                    "Polling",
                );
            });
        ui.separator();
        ui.label("间隔 (ms):");
        ui.add(
            egui::DragValue::new(&mut model.browse.interval_ms)
                .range(100.0..=60_000.0)
                .speed(10.0),
        );
        ui.separator();
        ui.label("深度:");
        ui.add(egui::DragValue::new(&mut model.browse.max_depth).range(1..=10));
    });
}

fn render_tree(
    ui: &mut egui::Ui,
    model: &mut AppModel,
    backend: &BackendHandle,
    conn_id: &str,
) {
    egui::ScrollArea::both()
        .auto_shrink([false, false])
        .max_height(360.0)
        .show(ui, |ui| {
            if !model.browse.root_loaded {
                if model.browse.pending.is_empty() {
                    ui.label("(无数据)");
                } else {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("加载根节点...");
                    });
                }
                return;
            }
            let roots = model.browse.roots.clone();
            for root in roots {
                render_node(ui, &root, model, backend, conn_id);
            }
        });
}

fn render_node(
    ui: &mut egui::Ui,
    node_id: &str,
    model: &mut AppModel,
    backend: &BackendHandle,
    conn_id: &str,
) {
    let (display, has_children, is_variable, loading, children) = {
        let Some(st) = model.browse.nodes.get(node_id) else {
            return;
        };
        (
            format!(
                "{}  [{}]{}",
                st.item.display_name,
                st.item.node_class,
                st.item
                    .data_type
                    .as_ref()
                    .map(|t| format!(" : {t}"))
                    .unwrap_or_default()
            ),
            st.item.has_children,
            st.item.node_class == "Variable",
            st.loading,
            st.children.clone(),
        )
    };

    let id = egui::Id::new(("browse_node", node_id));

    if has_children {
        ui.horizontal(|ui| {
            if is_variable {
                let mut checked = model.browse.selected.contains(node_id);
                if ui.checkbox(&mut checked, "").changed() {
                    toggle_selection(model, node_id, checked);
                }
            }
            let resp = egui::CollapsingHeader::new(display)
                .id_salt(id)
                .default_open(false)
                .show(ui, |ui| {
                    if loading {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("加载中...");
                        });
                    } else if let Some(ids) = &children {
                        let ids = ids.clone();
                        for cid in ids {
                            render_node(ui, &cid, model, backend, conn_id);
                        }
                    }
                });
            resp.header_response.context_menu(|ui| {
                if ui.button("➕ 添加此节点下所有变量").clicked() {
                    backend.send(UiCommand::AddVariablesUnderNode {
                        conn_id: conn_id.to_string(),
                        node_id: node_id.to_string(),
                        access_mode: model.browse.access_mode.clone(),
                        interval_ms: model.browse.interval_ms,
                        max_depth: model.browse.max_depth,
                    });
                    ui.close();
                }
            });
            if resp.fully_open() && children.is_none() && !loading {
                dispatch_browse(model, backend, conn_id, node_id);
            }
        });
    } else if is_variable {
        ui.horizontal(|ui| {
            let mut checked = model.browse.selected.contains(node_id);
            if ui.checkbox(&mut checked, &display).changed() {
                toggle_selection(model, node_id, checked);
            }
        });
    } else {
        ui.label(display);
    }
}

fn toggle_selection(model: &mut AppModel, node_id: &str, checked: bool) {
    if checked {
        model.browse.selected.insert(node_id.to_string());
    } else {
        model.browse.selected.remove(node_id);
    }
}

fn dispatch_browse(
    model: &mut AppModel,
    backend: &BackendHandle,
    conn_id: &str,
    node_id: &str,
) {
    let req_id = model.alloc_req_id();
    if let Some(st) = model.browse.nodes.get_mut(node_id) {
        st.loading = true;
        st.expanded = true;
    }
    model.browse.pending.insert(req_id);
    backend.send(UiCommand::BrowseNode {
        conn_id: conn_id.to_string(),
        node_id: node_id.to_string(),
        req_id,
    });
}

fn render_footer(
    ui: &mut egui::Ui,
    model: &mut AppModel,
    backend: &BackendHandle,
    conn_id: &str,
) {
    ui.horizontal(|ui| {
        let selected_count = model.browse.selected.len();
        ui.label(format!("已选 {selected_count} 个变量"));
        ui.separator();
        if ui.button("清空选择").clicked() {
            model.browse.selected.clear();
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_enabled_ui(selected_count > 0, |ui| {
                if ui.button(format!("➕ 添加选中 ({selected_count})")).clicked() {
                    let nodes: Vec<MonitoredNodeReq> = model
                        .browse
                        .selected
                        .iter()
                        .filter_map(|nid| model.browse.nodes.get(nid).map(|st| (nid, st)))
                        .map(|(nid, st)| MonitoredNodeReq {
                            node_id: nid.clone(),
                            display_name: st.item.display_name.clone(),
                            data_type: st.item.data_type.clone(),
                            access_mode: model.browse.access_mode.clone(),
                            interval_ms: model.browse.interval_ms,
                        })
                        .collect();
                    backend.send(UiCommand::AddMonitoredNodes {
                        conn_id: conn_id.to_string(),
                        nodes,
                    });
                    model.browse.selected.clear();
                }
            });
        });
    });
}

pub fn apply_browse_result(
    model: &mut AppModel,
    req_id: u64,
    parent: Option<String>,
    items: Vec<crate::events::BrowseItem>,
) {
    model.browse.pending.remove(&req_id);
    let ids: Vec<String> = items.iter().map(|it| it.node_id.clone()).collect();
    for it in items {
        let id = it.node_id.clone();
        model.browse.nodes.insert(
            id,
            BrowseNodeState {
                item: it,
                expanded: false,
                children: None,
                loading: false,
            },
        );
    }
    match parent {
        None => {
            model.browse.roots = ids;
            model.browse.root_loaded = true;
        }
        Some(pid) => {
            if let Some(st) = model.browse.nodes.get_mut(&pid) {
                st.loading = false;
                st.expanded = true;
                st.children = Some(ids);
            }
        }
    }
}
