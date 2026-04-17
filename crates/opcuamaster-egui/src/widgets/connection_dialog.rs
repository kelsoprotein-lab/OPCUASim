use crate::events::{AuthKindReq, CreateConnectionReq};

pub const SECURITY_POLICIES: &[&str] = &[
    "None",
    "Basic128Rsa15",
    "Basic256",
    "Basic256Sha256",
    "Aes128_Sha256_RsaOaep",
    "Aes256_Sha256_RsaPss",
];

pub const SECURITY_MODES: &[&str] = &["None", "Sign", "SignAndEncrypt"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthKind {
    Anonymous,
    UserPassword,
    Certificate,
}

impl AuthKind {
    fn label(self) -> &'static str {
        match self {
            Self::Anonymous => "Anonymous",
            Self::UserPassword => "UserPassword",
            Self::Certificate => "Certificate",
        }
    }
}

pub struct ConnDialogState {
    pub name: String,
    pub endpoint_url: String,
    pub security_policy: String,
    pub security_mode: String,
    pub auth: AuthKind,
    pub username: String,
    pub password: String,
    pub cert_path: String,
    pub key_path: String,
    pub timeout_ms: u64,
    pub error: Option<String>,
}

impl Default for ConnDialogState {
    fn default() -> Self {
        Self {
            name: String::from("New Connection"),
            endpoint_url: String::from("opc.tcp://localhost:4840"),
            security_policy: String::from("None"),
            security_mode: String::from("None"),
            auth: AuthKind::Anonymous,
            username: String::new(),
            password: String::new(),
            cert_path: String::new(),
            key_path: String::new(),
            timeout_ms: 5000,
            error: None,
        }
    }
}

impl ConnDialogState {
    pub fn validate(&self) -> Result<CreateConnectionReq, String> {
        if self.name.trim().is_empty() {
            return Err("连接名不能为空".into());
        }
        if self.endpoint_url.trim().is_empty() {
            return Err("Endpoint URL 不能为空".into());
        }
        let auth = match self.auth {
            AuthKind::Anonymous => AuthKindReq::Anonymous,
            AuthKind::UserPassword => {
                if self.username.trim().is_empty() {
                    return Err("用户名不能为空".into());
                }
                AuthKindReq::UserPassword {
                    username: self.username.clone(),
                    password: self.password.clone(),
                }
            }
            AuthKind::Certificate => {
                if self.cert_path.trim().is_empty() || self.key_path.trim().is_empty() {
                    return Err("证书与私钥路径都不能为空".into());
                }
                AuthKindReq::Certificate {
                    cert_path: self.cert_path.clone(),
                    key_path: self.key_path.clone(),
                }
            }
        };
        Ok(CreateConnectionReq {
            name: self.name.trim().to_string(),
            endpoint_url: self.endpoint_url.trim().to_string(),
            security_policy: self.security_policy.clone(),
            security_mode: self.security_mode.clone(),
            auth,
            timeout_ms: self.timeout_ms,
        })
    }
}

/// Renders the dialog; returns Some(submitted_request) if the user confirmed.
/// Sets `close` to true if the user chose to cancel or submitted successfully.
pub fn show(
    ctx: &egui::Context,
    state: &mut ConnDialogState,
    close: &mut bool,
) -> Option<CreateConnectionReq> {
    let mut submitted: Option<CreateConnectionReq> = None;

    egui::Window::new("新建连接")
        .collapsible(false)
        .resizable(false)
        .default_width(440.0)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .show(ctx, |ui| {
            egui::Grid::new("conn_dialog_grid")
                .num_columns(2)
                .spacing([10.0, 8.0])
                .show(ui, |ui| {
                    ui.label("名称");
                    ui.text_edit_singleline(&mut state.name);
                    ui.end_row();

                    ui.label("Endpoint URL");
                    ui.text_edit_singleline(&mut state.endpoint_url);
                    ui.end_row();

                    ui.label("Security Policy");
                    egui::ComboBox::from_id_salt("sec_policy")
                        .selected_text(&state.security_policy)
                        .show_ui(ui, |ui| {
                            for opt in SECURITY_POLICIES {
                                ui.selectable_value(
                                    &mut state.security_policy,
                                    (*opt).to_string(),
                                    *opt,
                                );
                            }
                        });
                    ui.end_row();

                    ui.label("Security Mode");
                    egui::ComboBox::from_id_salt("sec_mode")
                        .selected_text(&state.security_mode)
                        .show_ui(ui, |ui| {
                            for opt in SECURITY_MODES {
                                ui.selectable_value(
                                    &mut state.security_mode,
                                    (*opt).to_string(),
                                    *opt,
                                );
                            }
                        });
                    ui.end_row();

                    ui.label("认证方式");
                    egui::ComboBox::from_id_salt("auth_kind")
                        .selected_text(state.auth.label())
                        .show_ui(ui, |ui| {
                            ui.selectable_value(
                                &mut state.auth,
                                AuthKind::Anonymous,
                                AuthKind::Anonymous.label(),
                            );
                            ui.selectable_value(
                                &mut state.auth,
                                AuthKind::UserPassword,
                                AuthKind::UserPassword.label(),
                            );
                            ui.selectable_value(
                                &mut state.auth,
                                AuthKind::Certificate,
                                AuthKind::Certificate.label(),
                            );
                        });
                    ui.end_row();

                    match state.auth {
                        AuthKind::UserPassword => {
                            ui.label("用户名");
                            ui.text_edit_singleline(&mut state.username);
                            ui.end_row();
                            ui.label("密码");
                            ui.add(
                                egui::TextEdit::singleline(&mut state.password).password(true),
                            );
                            ui.end_row();
                        }
                        AuthKind::Certificate => {
                            ui.label("证书路径");
                            ui.text_edit_singleline(&mut state.cert_path);
                            ui.end_row();
                            ui.label("私钥路径");
                            ui.text_edit_singleline(&mut state.key_path);
                            ui.end_row();
                        }
                        AuthKind::Anonymous => {}
                    }

                    ui.label("超时 (ms)");
                    ui.add(egui::DragValue::new(&mut state.timeout_ms).range(500..=60_000));
                    ui.end_row();
                });

            if let Some(err) = &state.error {
                ui.colored_label(egui::Color32::LIGHT_RED, err);
            }

            ui.separator();
            ui.horizontal(|ui| {
                if ui.button("取消").clicked() {
                    *close = true;
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button("确认").clicked() {
                        match state.validate() {
                            Ok(req) => {
                                submitted = Some(req);
                                *close = true;
                            }
                            Err(e) => state.error = Some(e),
                        }
                    }
                });
            });
        });

    submitted
}
