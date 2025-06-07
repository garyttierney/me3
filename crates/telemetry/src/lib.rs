use std::{
    any::Any,
    collections::HashMap,
    error::Error,
    fs::{File, OpenOptions},
    str::FromStr,
    sync::OnceLock,
    time::Duration,
};

use opentelemetry::{
    global::{self, BoxedTracer},
    trace::TracerProvider,
};
use opentelemetry_appender_tracing::layer::OpenTelemetryTracingBridge;
use opentelemetry_otlp::LogExporter;
use opentelemetry_sdk::{
    logs::{log_processor_with_async_runtime::BatchLogProcessor, SdkLoggerProvider},
    propagation::TraceContextPropagator,
    resource::EnvResourceDetector,
    runtime,
    trace::{
        span_processor_with_async_runtime::BatchSpanProcessor, RandomIdGenerator, Sampler,
        SdkTracerProvider,
    },
    Resource,
};
use sentry_opentelemetry::{SentryPropagator, SentrySpanProcessor};
use tokio::runtime::Runtime;
use tracing::{info, Level};
use tracing_appender::non_blocking::WorkerGuard;
use tracing_error::ErrorLayer;
use tracing_opentelemetry::{OpenTelemetryLayer, OpenTelemetrySpanExt};
use tracing_subscriber::{
    fmt::{self, writer::BoxMakeWriter},
    prelude::*,
    EnvFilter,
};

pub struct TelemetryGuard {
    resources_held: Vec<Box<dyn Any + Send + Sync>>,
    rt: Option<Runtime>,
    logger_provider: Option<SdkLoggerProvider>,
    tracer_provider: Option<SdkTracerProvider>,
}

impl Drop for TelemetryGuard {
    fn drop(&mut self) {
        if let Some(logger) = self.logger_provider.take() {
            let _ = logger.force_flush();
            let _ = logger.shutdown();
        }

        if let Some(tracer) = self.tracer_provider.take() {
            let _ = tracer.force_flush();
            let _ = tracer.shutdown();
        }

        if let Some(rt) = self.rt.take() {
            rt.shutdown_timeout(Duration::from_secs(5));
        }
    }
}

#[derive(Clone, Copy)]
pub enum OtelExporter {
    Sentry,
    Http,
}

#[derive(Default)]
pub struct TelemetryConfig {
    enabled: bool,
    file_writer: Option<TelemetryWriter>,
    console_writer: Option<TelemetryWriter>,
    otel_exporter: Option<OtelExporter>,
}

pub type TelemetryWriter = (BoxMakeWriter, WorkerGuard);
pub fn get_tracer() -> &'static BoxedTracer {
    static TRACER: OnceLock<BoxedTracer> = OnceLock::new();
    TRACER.get_or_init(|| global::tracer("me3"))
}

fn resource(name: &'static str) -> Resource {
    Resource::builder()
        .with_service_name(name)
        .with_detectors(&[Box::new(EnvResourceDetector::new())])
        .build()
}

pub fn inherit_trace_id() {
    if let Ok(trace_id) = std::env::var("ME3_TRACE_ID") {
        let current = tracing::Span::current();
        let mut map: HashMap<String, String> = HashMap::default();
        map.insert("traceparent".to_string(), trace_id.to_string());
        map.insert("sentry-trace".to_string(), trace_id.to_string());
        let cx =
            opentelemetry::global::get_text_map_propagator(|propagator| propagator.extract(&map));

        current.set_parent(cx);
    }
}

pub fn trace_id() -> Option<String> {
    let mut map: HashMap<String, String> = HashMap::default();
    opentelemetry::global::get_text_map_propagator(|propagator| {
        propagator.inject(&mut map);
    });

    info!(?map, "trace_id map");

    map.remove("traceparent").or(map.remove("sentry-trace"))
}

impl TelemetryConfig {
    pub fn with_otel_exporter(self, otel_exporter: OtelExporter) -> Self {
        Self {
            otel_exporter: Some(otel_exporter),
            ..self
        }
    }

    pub fn with_console_writer<W2>(self, writer: W2) -> Self
    where
        W2: std::io::Write + Send + Sync + 'static,
    {
        let (console_writer, console_guard) = tracing_appender::non_blocking(writer);
        Self {
            console_writer: Some((BoxMakeWriter::new(console_writer), console_guard)),
            ..self
        }
    }

    pub fn with_file_writer<W2>(self, writer: W2) -> Self
    where
        W2: std::io::Write + Send + Sync + 'static,
    {
        let (file_writer, file_guard) = tracing_appender::non_blocking(writer);
        Self {
            file_writer: Some((BoxMakeWriter::new(file_writer), file_guard)),
            ..self
        }
    }

    pub fn enabled(self, enabled: bool) -> Self {
        Self { enabled, ..self }
    }
}

impl TelemetryConfig {
    pub fn try_from_env() -> Result<Self, Box<dyn Error>> {
        fn open_log_file(env_var: &str) -> std::io::Result<File> {
            let file = std::env::var(env_var).map_err(std::io::Error::other)?;

            OpenOptions::new().create(true).append(true).open(&file)
        }

        let enabled = std::env::var("ME3_TELEMETRY").is_ok();
        let log_file = open_log_file("ME3_LOG_FILE")?;
        let (file_writer, file_guard) = tracing_appender::non_blocking(log_file);

        let console_file = open_log_file("ME3_MONITOR_LOG_FILE")?;
        let (console_writer, console_guard) = tracing_appender::non_blocking(console_file);

        let otel_exporter = std::env::var("ME3_OTEL")
            .ok()
            .and_then(|v| match v.as_str() {
                "sentry" => Some(OtelExporter::Sentry),
                "http" => Some(OtelExporter::Http),
                _ => None,
            });

        Ok(TelemetryConfig {
            enabled,
            file_writer: Some((BoxMakeWriter::new(file_writer), file_guard)),
            console_writer: Some((BoxMakeWriter::new(console_writer), console_guard)),
            otel_exporter,
        })
    }
}

fn log_filter(env_var: &str, default_directive: Level) -> EnvFilter {
    EnvFilter::builder()
        .with_default_directive(default_directive.into())
        .with_env_var(env_var)
        .from_env_lossy()
        .add_directive("reqwest=error".parse().unwrap())
        .add_directive("hyper_util=error".parse().unwrap())
        .add_directive("opentelemetry-http=error".parse().unwrap())
}

pub fn install(component: &'static str, config: TelemetryConfig) -> TelemetryGuard {
    let (file_writer, file_guard) = config.file_writer.unzip();
    let (console_writer, console_guard) = config.console_writer.unzip();

    let mut resources_held: Vec<Box<dyn Any + Send + Sync>> = vec![];

    let file_layer = file_writer.map(|writer| {
        let filter_layer = log_filter("ME3_FILE_LOG_LEVEL", Level::DEBUG);

        fmt::layer()
            .with_ansi(false)
            .without_time()
            .with_writer(writer)
            .with_filter(filter_layer)
    });

    let console_layer = console_writer.map(|writer| {
        let filter_layer = log_filter("ME3_CONSOLE_LOG_LEVEL", Level::INFO);

        fmt::layer()
            .with_ansi(true)
            .without_time()
            .with_writer(writer)
            .with_filter(filter_layer)
    });

    let rt = tokio::runtime::Builder::new_multi_thread()
        .enable_time()
        .enable_io()
        .build()
        .expect("couldn't create tokio runtime for telemetry");

    let _guard = rt.enter();

    let (tracer_provider, logger_provider) = match config.otel_exporter {
        Some(OtelExporter::Sentry) => {
            let tracer_provider = SdkTracerProvider::builder()
                .with_sampler(Sampler::AlwaysOn)
                .with_span_processor(SentrySpanProcessor::new())
                .with_resource(resource(component))
                .build();

            opentelemetry::global::set_tracer_provider(tracer_provider.clone());
            opentelemetry::global::set_text_map_propagator(SentryPropagator::new());

            (Some(tracer_provider), None)
        }
        Some(OtelExporter::Http) => {
            let exporter = opentelemetry_otlp::SpanExporter::builder()
                .with_http()
                .build()
                .unwrap();

            let tracer_provider = SdkTracerProvider::builder()
                .with_sampler(Sampler::AlwaysOn)
                .with_id_generator(RandomIdGenerator::default())
                .with_resource(resource(component))
                .with_span_processor(BatchSpanProcessor::builder(exporter, runtime::Tokio).build())
                .build();

            opentelemetry::global::set_text_map_propagator(TraceContextPropagator::new());
            opentelemetry::global::set_tracer_provider(tracer_provider.clone());

            let log_exporter = LogExporter::builder()
                .with_http()
                .build()
                .expect("Failed to create log exporter");

            let logger_provider = SdkLoggerProvider::builder()
                .with_log_processor(
                    BatchLogProcessor::builder(log_exporter, runtime::Tokio).build(),
                )
                .build();

            (Some(tracer_provider), Some(logger_provider))
        }
        _ => (None, None),
    };

    tracing_subscriber::registry()
        .with(
            tracer_provider
                .as_ref()
                .map(|provider| OpenTelemetryLayer::new(provider.tracer("me3"))),
        )
        .with(
            logger_provider
                .as_ref()
                .map(OpenTelemetryTracingBridge::new),
        )
        .with(file_layer)
        .with(console_layer)
        .with(ErrorLayer::default())
        .init();

    resources_held.push(Box::from(console_guard));
    resources_held.push(Box::from(file_guard));

    #[cfg(feature = "sentry")]
    {
        use sentry::types::Dsn;

        let sentry_dsn = option_env!("SENTRY_DSN").and_then(|dsn| Dsn::from_str(dsn).ok());
        if sentry_dsn.is_none() {
            eprintln!("No Sentry DSN provided, but crash reporting was enabled");
        }

        let client = config.enabled.then(|| {
            sentry::init(sentry::ClientOptions {
                release: Some(env!("CARGO_PKG_VERSION").into()),
                debug: cfg!(debug_assertions),
                traces_sample_rate: 1.0,
                dsn: sentry_dsn,

                ..Default::default()
            })
        });

        resources_held.push(Box::from(client));
    };

    TelemetryGuard {
        resources_held,
        tracer_provider,
        logger_provider,
        rt: Some(rt),
    }
}
