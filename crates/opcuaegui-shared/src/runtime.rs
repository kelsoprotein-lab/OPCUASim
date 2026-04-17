use std::future::Future;
use std::time::Duration;

use tokio::runtime::Runtime;
use tokio::sync::mpsc::{self, UnboundedReceiver, UnboundedSender};
use tokio_util::sync::CancellationToken;

/// Generic backend handle owning a dedicated tokio runtime and a single
/// mpsc command channel.
///
/// The dispatcher future receives `(cmd_rx, event_tx, cancel, egui_ctx)` and
/// is expected to exit when either the cancellation token fires or `cmd_rx`
/// is closed (both happen automatically when the handle is dropped).
pub struct BackendHandle<Cmd> {
    runtime: Option<Runtime>,
    cmd_tx: UnboundedSender<Cmd>,
    cancel: CancellationToken,
}

impl<Cmd: Send + 'static> BackendHandle<Cmd> {
    pub fn new<Event, F, Fut>(
        egui_ctx: egui::Context,
        thread_name: &str,
        main: F,
    ) -> (Self, UnboundedReceiver<Event>)
    where
        Event: Send + 'static,
        F: FnOnce(
                UnboundedReceiver<Cmd>,
                UnboundedSender<Event>,
                CancellationToken,
                egui::Context,
            ) -> Fut
            + Send
            + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .thread_name(thread_name)
            .build()
            .expect("failed to build tokio runtime");

        let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let cancel = CancellationToken::new();

        runtime.spawn(main(cmd_rx, event_tx, cancel.clone(), egui_ctx));

        (
            Self {
                runtime: Some(runtime),
                cmd_tx,
                cancel,
            },
            event_rx,
        )
    }

    pub fn send(&self, cmd: Cmd) {
        if let Err(e) = self.cmd_tx.send(cmd) {
            log::debug!("backend cmd channel closed: {e}");
        }
    }
}

impl<Cmd> Drop for BackendHandle<Cmd> {
    fn drop(&mut self) {
        self.cancel.cancel();
        if let Some(rt) = self.runtime.take() {
            rt.shutdown_timeout(Duration::from_secs(3));
        }
    }
}
