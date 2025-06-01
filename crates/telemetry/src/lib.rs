use std::str::FromStr;

use sentry::ClientInitGuard;
use tracing_error::ErrorLayer;
use tracing_subscriber::{
    fmt::{self, MakeWriter},
    prelude::*,
    EnvFilter,
};

pub struct TelemetryGuard {
    #[cfg(feature = "sentry")]
    client: ClientInitGuard,
}

pub fn install<W>(telemetry: bool, writer: W) -> TelemetryGuard
where
    W: for<'writer> MakeWriter<'writer> + 'static + Send + Sync,
{
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();

    tracing_subscriber::registry()
        .with(ErrorLayer::default())
        .with(filter_layer)
        .with(
            fmt::layer()
                .compact()
                .with_ansi(true)
                .without_time()
                .with_writer(writer),
        )
        .with(sentry::integrations::tracing::layer())
        .init();

    #[cfg(feature = "sentry")]
    let client = {
        use sentry::types::Dsn;

        let sentry_dsn = option_env!("SENTRY_DSN").and_then(|dsn| Dsn::from_str(dsn).ok());
        if sentry_dsn.is_none() {
            eprintln!("No Sentry DSN provided, but crash reporting was enabled");
        }

        sentry::init(sentry::ClientOptions {
            debug: cfg!(debug_assertions),
            traces_sample_rate: 1.0,
            dsn: sentry_dsn,

            ..Default::default()
        })
    };

    TelemetryGuard {
        #[cfg(feature = "sentry")]
        client,
    }
}
