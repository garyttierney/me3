use std::sync::{Arc, Mutex};

use ipc_channel::ipc::IpcSender;
use me3_launcher_attach_protocol::HostMessage;
use tracing::Subscriber;
use tracing_subscriber::{
    field::VisitOutput, fmt::format::JsonVisitor, registry::LookupSpan, Layer,
};

pub struct HostTracingLayer {
    pub(crate) socket: Arc<Mutex<IpcSender<HostMessage>>>,
}

impl<S> Layer<S> for HostTracingLayer
where
    S: Subscriber + for<'lookup> LookupSpan<'lookup>,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let mut output = String::new();
        let mut visitor = JsonVisitor::new(&mut output);

        event.record(&mut visitor);

        if visitor.finish().is_ok() {
            self.socket
                .lock()
                .expect("lock poisoned")
                .send(HostMessage::Trace(output))
                .unwrap()
        }
    }
}
