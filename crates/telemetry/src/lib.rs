use std::str::FromStr;

use sentry::ClientInitGuard;
use tracing_error::ErrorLayer;
use tracing_subscriber::{
    fmt::{self, writer::BoxMakeWriter, MakeWriter},
    prelude::*,
    EnvFilter,
};

pub struct TelemetryGuard {
    #[cfg(feature = "sentry")]
    client: Option<ClientInitGuard>,
}

pub fn install(
    telemetry_enabled: bool,
    file_writer: Option<BoxMakeWriter>,
    monitor_writer: Option<BoxMakeWriter>,
) -> TelemetryGuard
where
{
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    tracing_subscriber::registry()
        .with(ErrorLayer::default())
        .with(filter_layer)
        .with(file_writer.map(|file_writer| {
            fmt::layer()
                .pretty()
                .with_ansi(false)
                .without_time()
                .with_writer(file_writer)
        }))
        .with(monitor_writer.map(|writer| {
            fmt::layer()
                .compact()
                .with_ansi(true)
                .without_time()
                .with_writer(writer)
        }))
        .with(telemetry_enabled.then(sentry::integrations::tracing::layer))
        .init();

    #[cfg(feature = "sentry")]
    let client = {
        use sentry::types::Dsn;

        let sentry_dsn = option_env!("SENTRY_DSN").and_then(|dsn| Dsn::from_str(dsn).ok());
        if sentry_dsn.is_none() {
            eprintln!("No Sentry DSN provided, but crash reporting was enabled");
        }

        telemetry_enabled.then(|| {
            sentry::init(sentry::ClientOptions {
                release: Some(env!("CARGO_PKG_VERSION").into()),
                debug: cfg!(debug_assertions),
                traces_sample_rate: 1.0,
                dsn: sentry_dsn,

                ..Default::default()
            })
        })
    };

    TelemetryGuard {
        #[cfg(feature = "sentry")]
        client,
    }
}
