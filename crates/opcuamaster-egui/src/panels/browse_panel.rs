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

    egui::CollapsingHeader::new("高级 (DataChangeFilter)")
        .id_salt("browse_advanced_filter")
        .default_open(false)
        .show(ui, |ui| {
            ui.checkbox(&mut model.browse.filter_enabled, "启用 DataChangeFilter");
            ui.add_enabled_ui(model.browse.filter_enabled, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Trigger:");
                    egui::ComboBox::from_id_salt("filter_trigger")
                        .selected_text(format!("{:?}", model.browse.trigger))
                        .show_ui(ui, |ui| {
                            for v in [
                                crate::events::DataChangeTriggerKindReq::Status,
                                crate::events::DataChangeTriggerKindReq::StatusValue,
                                crate::events::DataChangeTriggerKindReq::StatusValueTimestamp,
                            ] {
                                ui.selectable_value(
                                    &mut model.browse.trigger,
                                    v,
                                    format!("{v:?}"),
                                );
                            }
                        });
                });
                ui.horizontal(|ui| {
                    ui.label("Deadband:");
                    egui::ComboBox::from_id_salt("filter_deadband")
                        .selected_text(format!("{:?}", model.browse.deadband_kind))
                        .show_ui(ui, |ui| {
                            for v in [
                                crate::events::DeadbandKindReq::None,
                                crate::events::DeadbandKindReq::Absolute,
                                crate::events::DeadbandKindReq::Percent,
                            ] {
                                ui.selectable_value(
                                    &mut model.browse.deadband_kind,
                                    v,
                                    format!("{v:?}"),
                                );
                            }
                        });
                    ui.add_enabled(
                        !matches!(
                            model.browse.deadband_kind,
                            crate::events::DeadbandKindReq::None
                        ),
                        egui::DragValue::new(&mut model.browse.deadband_value)
                            .range(0.0..=100_000.0)
                            .speed(0.1),
                    );
                });
            });
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
                    opcuaegui_shared::widgets::empty_state(
                        ui,
                        "🌲",
                        "无可用根节点",
                        Some("请确认服务端已运行并暴露地址空间"),
                    );
                } else {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label("加载根节点…");
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
    let (display, has_children, is_variable, is_method, loading, children) = {
        let Some(st) = model.browse.nodes.get(node_id) else {
            return;
        };
        let icon = match st.item.node_class.as_str() {
            "Method" => "⚙",
            "Object" => "📁",
            "Variable" => "🔢",
            _ => "•",
        };
        (
            format!(
                "{icon}  {}  [{}]{}",
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
            st.item.node_class == "Method",
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
                        filter: model.current_filter_req(),
                    });
                    ui.close();
                }
            });
            if resp.fully_open() && children.is_none() && !loading {
                dispatch_browse(model, backend, conn_id, node_id);
            }
        });
    } else if is_variable {
        let display_name = model
            .browse
            .nodes
            .get(node_id)
            .map(|s| s.item.display_name.clone())
            .unwrap_or_else(|| node_id.to_string());
        let resp = ui
            .horizontal(|ui| {
                let mut checked = model.browse.selected.contains(node_id);
                if ui.checkbox(&mut checked, &display).changed() {
                    toggle_selection(model, node_id, checked);
                }
            })
            .response;
        resp.context_menu(|ui| {
            if ui.button("📈 查看历史").clicked() {
                open_history_tab(model, conn_id, node_id, &display_name);
                ui.close();
            }
        });
    } else if is_method {
        ui.horizontal(|ui| {
            let resp = ui.label(display);
            resp.context_menu(|ui| {
                if ui.button("⚙ 调用方法...").clicked() {
                    let parent_id = find_parent_object(model, node_id)
                        .unwrap_or_else(|| node_id.to_string());
                    let display_name = model
                        .browse
                        .nodes
                        .get(node_id)
                        .map(|s| s.item.display_name.clone())
                        .unwrap_or_else(|| node_id.to_string());
                    let req_id = model.alloc_req_id();
                    let mut s = crate::model::MethodCallState::new(
                        conn_id.to_string(),
                        parent_id,
                        node_id.to_string(),
                        display_name,
                    );
                    s.pending_args_req = Some(req_id);
                    model.modal = Some(crate::model::Modal::MethodCall(s));
                    backend.send(UiCommand::ReadMethodArgs {
                        conn_id: conn_id.to_string(),
                        method_id: node_id.to_string(),
                        req_id,
                    });
                    ui.close();
                }
            });
        });
    } else {
        ui.label(display);
    }
}

fn find_parent_object(model: &AppModel, node_id: &str) -> Option<String> {
    for (pid, st) in &model.browse.nodes {
        if let Some(kids) = &st.children {
            if kids.iter().any(|k| k == node_id) {
                return Some(pid.clone());
            }
        }
    }
    None
}

pub fn open_history_tab(
    model: &mut AppModel,
    conn_id: &str,
    node_id: &str,
    display_name: &str,
) {
    let idx = model.history_tabs.len();
    model.history_tabs.push(crate::model::HistoryTabState::new(
        conn_id.to_string(),
        node_id.to_string(),
        display_name.to_string(),
    ));
    model.central_tab = crate::model::CentralPanelTab::History(idx);
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
                    let filter_req = model.current_filter_req();
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
                            filter: filter_req,
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
