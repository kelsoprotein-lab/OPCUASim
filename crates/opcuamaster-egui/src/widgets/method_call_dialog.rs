use crate::events::MethodArgValue;
use crate::model::MethodCallState;

pub struct DialogActions {
    pub close: bool,
    pub call: Option<Vec<MethodArgValue>>,
}

pub fn show(ctx: &egui::Context, state: &mut MethodCallState) -> DialogActions {
    let mut actions = DialogActions {
        close: false,
        call: None,
    };

    egui::Window::new(format!("调用方法: {}", state.display_name))
        .collapsible(false)
        .resizable(true)
        .min_width(560.0)
        .default_width(720.0)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            ui.label(format!("Method: {}", state.method_id));
            ui.label(format!("Object: {}", state.object_id));
            ui.separator();

            ui.heading("输入参数");
            if state.inputs_meta.is_empty() && state.pending_args_req.is_some() {
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label("加载参数...");
                });
            } else if state.inputs_meta.is_empty() {
                ui.label("(无入参)");
            } else {
                if state.input_values.len() != state.inputs_meta.len() {
                    state.input_values =
                        state.inputs_meta.iter().map(default_for_type).collect();
                }
                for (i, arg) in state.inputs_meta.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(format!("{} ({}):", arg.name, arg.data_type));
                        ui.text_edit_singleline(&mut state.input_values[i]);
                    });
                }
            }

            if let Some(err) = &state.error {
                ui.colored_label(egui::Color32::LIGHT_RED, err);
            }

            ui.separator();

            ui.heading("输出参数");
            if state.last_result_status.is_some() {
                ui.label(format!(
                    "Status: {}",
                    state.last_result_status.as_deref().unwrap_or("?")
                ));
                if state.outputs_meta.is_empty() && state.last_result_outputs.is_empty() {
                    ui.label("(无返回)");
                } else {
                    for (i, out) in state.last_result_outputs.iter().enumerate() {
                        let name = state
                            .outputs_meta
                            .get(i)
                            .map(|m| m.name.clone())
                            .unwrap_or_else(|| format!("[{i}]"));
                        ui.label(format!("{} ({}) = {}", name, out.data_type, out.value));
                    }
                }
            } else {
                ui.label("(尚未执行)");
            }

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("关闭").clicked() {
                    actions.close = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let busy = state.pending_call_req.is_some();
                    let label = if busy { "执行中…" } else { "执行" };
                    if ui.add_enabled(!busy, egui::Button::new(label)).clicked() {
                        actions.call = Some(
                            state
                                .inputs_meta
                                .iter()
                                .zip(state.input_values.iter())
                                .map(|(meta, v)| MethodArgValue {
                                    data_type: meta.data_type.clone(),
                                    value: v.clone(),
                                })
                                .collect(),
                        );
                    }
                });
            });
        });

    actions
}

fn default_for_type(arg: &crate::events::MethodArgInfo) -> String {
    match arg.data_type.as_str() {
        "Boolean" => "false".into(),
        "String" => "".into(),
        "Float" | "Double" => "0.0".into(),
        _ => "0".into(),
    }
}
