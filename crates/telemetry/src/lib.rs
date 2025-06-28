use std::fs::OpenOptions;

use me3_env::TelemetryVars;
use tracing::Level;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_error::ErrorLayer;
use tracing_subscriber::{
    fmt::{self, writer::BoxMakeWriter, MakeWriter},
    prelude::*,
    EnvFilter,
};

pub struct Telemetry {
    #[cfg(feature = "sentry")]
    client: Option<sentry::ClientInitGuard>,
}

pub fn with_root_span<T>(
    name: &str,
    op: &str,
    f: impl FnOnce() -> color_eyre::Result<T>,
) -> color_eyre::Result<T> {
    #[cfg(feature = "sentry")]
    let (transaction_is_root, transaction) = {
        use sentry::{Hub, TransactionContext, TransactionOrSpan};

        let hub = Hub::main();
        let trace_id = me3_env::deserialize_from_env()
            .ok()
            .and_then(|vars: TelemetryVars| vars.trace_id);

        let (transaction_is_root, transaction_context) = trace_id
            .and_then(|v| sentry::parse_headers([("sentry-trace", v.as_str())]))
            .map(|trace| {
                (
                    false,
                    TransactionContext::continue_from_sentry_trace(name, op, &trace, None),
                )
            })
            .unwrap_or_else(|| (true, TransactionContext::new(name, op)));

        if transaction_is_root {
            sentry::start_session();
        }

        let transaction = sentry::start_transaction(transaction_context);
        hub.configure_scope(|scope| {
            scope.set_span(Some(TransactionOrSpan::Transaction(transaction.clone())))
        });

        (transaction_is_root, transaction)
    };

    let result = f();

    if let Err(e) = result.as_ref() {
        report_fatal_error(e);
    }

    #[cfg(feature = "sentry")]
    {
        transaction.finish();

        if transaction_is_root {
            sentry::end_session_with_status(match &result {
                Ok(_) => sentry::protocol::SessionStatus::Ok,
                Err(_) => sentry::protocol::SessionStatus::Crashed,
            });
        }
    }

    result
}

impl Drop for Telemetry {
    fn drop(&mut self) {
        #[cfg(feature = "sentry")]
        if let Some(client) = self.client.take() {
            client.flush(Some(std::time::Duration::from_secs(10)));
        }
    }
}

impl TryFrom<TelemetryVars> for TelemetryConfig {
    type Error = color_eyre::Report;

    fn try_from(value: TelemetryVars) -> color_eyre::Result<Self> {
        let log = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&value.log_file_path)?;
        let monitor = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&value.monitor_file_path)?;

        Ok(TelemetryConfig::default()
            .enabled(value.enabled)
            .with_console_writer(monitor)
            .with_file_writer(log)
            .capture_panics(true))
    }
}

#[derive(Default, Debug)]
pub struct TelemetryConfig {
    enabled: bool,
    capture_panics: bool,
    file_writer: Option<TelemetryWriter>,
    console_writer: Option<TelemetryWriter>,
}

pub type TelemetryWriter = BoxMakeWriter;

pub fn report_fatal_error(error: &color_eyre::Report) {
    let backtrace = error
        .handler()
        .downcast_ref::<color_eyre::Context>()
        .and_then(|ctx| ctx.backtrace());

    #[cfg(feature = "sentry")]
    {
        use sentry::integrations::backtrace::backtrace_to_stacktrace;

        let mut event = sentry::event_from_error(&**error);

        event.stacktrace = backtrace.and_then(backtrace_to_stacktrace);
        event.level = sentry::Level::Fatal;
        event.message = Some(format!("{error:?}"));

        for exc in event.exception.iter_mut() {
            use sentry::protocol::Mechanism;

            exc.mechanism = Some(Mechanism {
                ty: "error".into(),
                handled: Some(false),
                ..Default::default()
            })
        }

        sentry::capture_event(event);
    }
}

pub fn trace_id() -> Option<String> {
    #[cfg(feature = "sentry")]
    {
        let hub = sentry::Hub::current();

        hub.configure_scope(|scope| {
            scope.get_span().map(|tx| {
                let cx = tx.get_trace_context();
                let trace =
                    sentry::SentryTrace::new(cx.trace_id, cx.span_id, Some(tx.is_sampled()));

                trace.to_string()
            })
        })
    }

    #[cfg(not(feature = "sentry"))]
    None
}

impl TelemetryConfig {
    pub fn with_console_writer<W2>(self, writer: W2) -> Self
    where
        W2: for<'a> MakeWriter<'a> + Send + Sync + 'static,
    {
        Self {
            console_writer: Some(BoxMakeWriter::new(writer)),
            ..self
        }
    }

    pub fn with_file_writer<W2>(self, writer: W2) -> Self
    where
        W2: for<'a> MakeWriter<'a> + Send + Sync + 'static,
    {
        Self {
            file_writer: Some(BoxMakeWriter::new(writer)),
            ..self
        }
    }

    pub fn capture_panics(self, capture_panics: bool) -> Self {
        Self {
            capture_panics,
            ..self
        }
    }
    pub fn enabled(self, enabled: bool) -> Self {
        Self { enabled, ..self }
    }
}

fn log_filter(env_var: &str, default_directive: Level) -> EnvFilter {
    EnvFilter::builder()
        .with_default_directive(default_directive.into())
        .with_env_var(env_var)
        .from_env_lossy()
}

pub fn install_error_handler() {
    unsafe {
        std::env::set_var("RUST_LIB_BACKTRACE", "1");
        std::env::set_var("RUST_BACKTRACE", "1");
    }

    color_eyre::install().expect("failed to install error handler");
}

pub fn install(config: TelemetryConfig) -> Telemetry {
    let file_layer = config.file_writer.map(|writer| {
        let filter_layer = log_filter("ME3_FILE_LOG_LEVEL", Level::DEBUG);

        fmt::layer()
            .with_ansi(false)
            .without_time()
            .with_writer(writer)
            .with_filter(filter_layer)
    });

    let console_layer = config.console_writer.map(|writer| {
        let filter_layer = log_filter("ME3_CONSOLE_LOG_LEVEL", Level::INFO);

        fmt::layer()
            .with_ansi(true)
            .without_time()
            .with_writer(writer)
            .with_filter(filter_layer)
    });

    #[cfg(not(feature = "sentry"))]
    let layer = tracing_subscriber::layer::Identity::new();

    #[cfg(feature = "sentry")]
    let layer = sentry::integrations::tracing::layer();

    tracing_subscriber::registry()
        .with(ErrorLayer::default())
        .with(file_layer)
        .with(console_layer.boxed())
        .with(layer)
        .init();

    #[cfg(feature = "sentry")]
    let client = {
        use std::str::FromStr;

        use sentry::types::Dsn;
        let sentry_dsn = option_env!("SENTRY_DSN").and_then(|dsn| Dsn::from_str(dsn).ok());

        let environment = if cfg!(debug_assertions) {
            "development"
        } else {
            "production"
        };

        config.enabled.then(|| {
            use std::sync::Arc;

            use sentry::{
                integrations::{
                    backtrace::{AttachStacktraceIntegration, ProcessStacktraceIntegration},
                    contexts::ContextIntegration,
                    debug_images::DebugImagesIntegration,
                },
                Integration,
            };

            let mut integrations: Vec<Arc<dyn Integration>> = vec![
                Arc::new(DebugImagesIntegration::new()),
                Arc::new(ContextIntegration::new()),
                Arc::new(AttachStacktraceIntegration::new()),
                Arc::new(ProcessStacktraceIntegration::new()),
            ];

            if config.capture_panics {
                use sentry::integrations::panic::PanicIntegration;

                integrations.push(Arc::new(PanicIntegration::default()));
            }

            sentry::init(sentry::ClientOptions {
                release: Some(env!("CARGO_PKG_VERSION").into()),
                debug: cfg!(debug_assertions),
                traces_sample_rate: 1.0,
                dsn: sentry_dsn,
                environment: Some(environment.into()),
                default_integrations: false,
                in_app_include: vec!["me3"],
                auto_session_tracking: false,
                session_mode: sentry::SessionMode::Application,
                integrations,
                ..Default::default()
            })
        })
    };

    Telemetry {
        #[cfg(feature = "sentry")]
        client,
    }
}
