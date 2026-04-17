use std::time::Duration;

use tokio::runtime::Runtime;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio_util::sync::CancellationToken;

use crate::backend::dispatcher;
use crate::events::{BackendEvent, UiCommand};

pub struct BackendHandle {
    runtime: Option<Runtime>,
    cmd_tx: UnboundedSender<UiCommand>,
    cancel_token: CancellationToken,
}

impl BackendHandle {
    pub fn new(egui_ctx: egui::Context) -> (Self, UnboundedReceiver<BackendEvent>) {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name("opcua-server-backend")
            .build()
            .expect("failed to build tokio runtime");

        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let cancel_token = CancellationToken::new();

        runtime.spawn(dispatcher::run(
            cmd_rx,
            event_tx,
            cancel_token.clone(),
            egui_ctx,
        ));

        (
            Self {
                runtime: Some(runtime),
                cmd_tx,
                cancel_token,
            },
            event_rx,
        )
    }

    pub fn send(&self, cmd: UiCommand) {
        let _ = self.cmd_tx.send(cmd);
    }

    fn shutdown(&mut self) {
        let _ = self.cmd_tx.send(UiCommand::Shutdown);
        self.cancel_token.cancel();
        if let Some(rt) = self.runtime.take() {
            rt.shutdown_timeout(Duration::from_secs(3));
        }
    }
}

impl Drop for BackendHandle {
    fn drop(&mut self) {
        self.shutdown();
    }
}
