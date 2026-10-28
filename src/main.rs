use std::net::SocketAddr;
use std::process::ExitCode;
use std::time::Duration;

use axum::ServiceExt;
use axum::extract::Request;
use eabsensi_mobile_api::config::{Config, LogFormat, RateLimitStoreKind};
use eabsensi_mobile_api::rate_limit::RateLimiter;
use eabsensi_mobile_api::{AppState, app};
use sqlx::postgres::{PgConnectOptions, PgPoolOptions};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> ExitCode {
    // .env hanya kemudahan lokal; di server semua nilai datang dari environment (unit systemd).
    let _ = dotenvy::dotenv();

    let config = match Config::from_env() {
        Ok(config) => config,
        Err(error) => {
            eprintln!("konfigurasi tidak valid: {error}");
            return ExitCode::FAILURE;
        }
    };
    init_tracing(&config.log_format);

    match run(config).await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!(%error, "service berhenti karena galat");
            ExitCode::FAILURE
        }
    }
}

fn init_tracing(format: &LogFormat) {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,sqlx=warn"));
    let builder = tracing_subscriber::fmt().with_env_filter(filter);
    match format {
        LogFormat::Json => builder.json().flatten_event(true).init(),
        LogFormat::Pretty => builder.init(),
    }
}

async fn run(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    // Sesi database selalu UTC, sama dengan Laravel (config/database.php).
    let connect: PgConnectOptions = config
        .database_url
        .parse::<PgConnectOptions>()?
        .options([("TimeZone", "UTC")]);
    // Lazy: service tetap naik walau database belum siap; /healthz melaporkan 503 sampai tersambung.
    let db = PgPoolOptions::new()
        .max_connections(config.db_max_connections)
        .min_connections(config.db_min_connections)
        .acquire_timeout(config.db_acquire_timeout)
        .connect_lazy_with(connect);

    let limiter = match &config.rate_limit_store {
        RateLimitStoreKind::Memory => {
            let limiter = RateLimiter::memory();
            let purger = limiter.clone();
            tokio::spawn(async move {
                let mut tick = tokio::time::interval(Duration::from_secs(60));
                loop {
                    tick.tick().await;
                    purger.purge_expired();
                }
            });
            limiter
        }
        RateLimitStoreKind::Redis(url) => RateLimiter::redis(url).await?,
    };

    let addr = config.listen_addr;
    let drain = config.shutdown_drain;
    let state = AppState::new(db, config, limiter);
    let shutdown_state = state.clone();
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(%addr, "eabsensi-mobile-api mendengarkan");

    let service =
        ServiceExt::<Request>::into_make_service_with_connect_info::<SocketAddr>(app(state));
    axum::serve(listener, service)
        .with_graceful_shutdown(async move {
            shutdown_signal().await;
            if !drain.is_zero() {
                // Health check gagal dulu agar load balancer berhenti mengirim request, lalu berhenti.
                shutdown_state.start_draining();
                tracing::info!(secs = drain.as_secs(), "drain sebelum berhenti");
                tokio::time::sleep(drain).await;
            }
        })
        .await?;
    tracing::info!("berhenti dengan rapi");
    Ok(())
}

/// Ctrl+C atau SIGTERM (systemd/load balancer drain): berhenti menerima koneksi baru, selesaikan yang berjalan.
async fn shutdown_signal() {
    let ctrl_c = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        () = ctrl_c => {},
        () = terminate => {},
    }
    tracing::info!("sinyal berhenti diterima");
}
