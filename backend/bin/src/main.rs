mod config;
mod errors;
mod helpers;
mod middleware;
mod routes;
mod state;

use std::sync::Arc;

use actix_cors::Cors;
use actix_web::{web, App, HttpServer};
use clap::Parser;

use auth::{JwtService, WebAuthnService};
use storage::SqliteStorage;
use strava_client::StravaClient;

use crate::config::Config;
use crate::state::AppState;

#[derive(Parser)]
#[command(name = "pace-buddy", about = "Pace Buddy backend server")]
struct Cli {
    /// Path to a frontend dist directory to serve as static files.
    /// When omitted, only the API is served.
    #[arg(long)]
    static_serving: Option<String>,

    /// Delete the database file before starting, giving a clean slate.
    #[arg(long)]
    fresh_start: bool,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let cli = Cli::parse();
    let cfg = Config::from_env();

    if cli.fresh_start {
        // Extract file path from "sqlite:path?..." URL
        let db_path = cfg
            .database_url
            .strip_prefix("sqlite:")
            .unwrap_or(&cfg.database_url)
            .split('?')
            .next()
            .unwrap_or("data.db");
        for suffix in ["", "-journal", "-wal", "-shm"] {
            let path = format!("{db_path}{suffix}");
            if std::path::Path::new(&path).exists() {
                std::fs::remove_file(&path).ok();
                log::info!("Removed {path}");
            }
        }
        log::info!("Fresh start: database cleared");
    }

    // Check if the database file already exists before initializing
    let db_path = cfg
        .database_url
        .strip_prefix("sqlite:")
        .unwrap_or(&cfg.database_url)
        .split('?')
        .next()
        .unwrap_or("data.db");
    let db_existed = std::path::Path::new(db_path).exists();

    let storage = SqliteStorage::new(&cfg.database_url)
        .await
        .expect("Failed to initialize storage");

    if db_existed {
        log::info!("Database loaded from {db_path}");
    } else {
        log::info!("Database created at {db_path}");
    }

    // Log registered user count
    {
        use storage::Storage;
        match storage.list_users().await {
            Ok(users) => log::info!("{} registered user(s)", users.len()),
            Err(e) => log::warn!("Could not count users: {e}"),
        }
    }

    let webauthn = WebAuthnService::new(&cfg.webauthn_rp_id, &cfg.webauthn_rp_origin)
        .expect("Failed to initialize WebAuthn");

    let jwt = JwtService::new(&cfg.jwt_secret);

    let strava_client = StravaClient::new(
        cfg.strava_client_id.clone(),
        cfg.strava_client_secret.clone(),
        cfg.strava_redirect_uri.clone(),
    );

    let llm_client = cfg.openrouter_api_key.as_ref().map(|key| {
        log::info!("OpenRouter API key configured, LLM insights enabled");
        Arc::new(llm::open_router::OpenRouterClient::new(key.clone()))
    });

    let stream_cache_ttl_hours: i64 = std::env::var("STREAM_CACHE_TTL_HOURS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(24);

    let storage = Arc::new(storage);

    // Background task: purge cached streams older than the TTL
    {
        let storage = Arc::clone(&storage);
        let max_age_days = stream_cache_ttl_hours.max(24) / 24; // at least 1 day
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(3600));
            loop {
                interval.tick().await;
                match storage.purge_old_streams(max_age_days).await {
                    Ok(0) => {}
                    Ok(n) => log::info!("Purged {n} expired stream rows"),
                    Err(e) => log::warn!("Stream purge failed: {e}"),
                }
            }
        });
    }

    let strava_client = Arc::new(strava_client);

    let app_state = web::Data::new(AppState {
        storage,
        strava_client: Arc::clone(&strava_client),
        webauthn: Arc::new(webauthn),
        jwt: Arc::new(jwt),
        frontend_url: cfg.frontend_url.clone(),
        llm_client,
        strava_webhook_verify_token: cfg.strava_webhook_verify_token.clone(),
        admin_strava_athlete_id: cfg.admin_strava_athlete_id,
        quota_markup_ratio: cfg.quota_markup_ratio,
    });

    // Background task: check/create Strava webhook subscription
    if let (Some(base_url), Some(verify_token)) = (&cfg.base_url, &cfg.strava_webhook_verify_token)
    {
        let callback_url = format!("{base_url}/api/strava/webhook");
        let verify_token = verify_token.clone();
        let client = Arc::clone(&strava_client);
        tokio::spawn(async move {
            match client.view_webhook_subscriptions().await {
                Ok(subs) if !subs.is_empty() => {
                    log::info!(
                        "Strava webhook subscription already exists (id={})",
                        subs[0].id
                    );
                }
                Ok(_) => {
                    log::info!("No Strava webhook subscription found, creating one...");
                    match client
                        .create_webhook_subscription(&callback_url, &verify_token)
                        .await
                    {
                        Ok(sub) => {
                            log::info!("Strava webhook subscription created (id={})", sub.id)
                        }
                        Err(e) => log::error!("Failed to create Strava webhook subscription: {e}"),
                    }
                }
                Err(e) => log::error!("Failed to check Strava webhook subscriptions: {e}"),
            }
        });
    } else {
        log::warn!("Strava webhook subscription not configured");
    }

    let static_dir = cli.static_serving.clone();

    log::info!("Starting server at {}:{}", cfg.host, cfg.port);
    if let Some(ref dir) = static_dir {
        log::info!("Serving static files from {dir}");
    }

    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin(&cfg.frontend_url)
            .allow_any_method()
            .allow_any_header()
            .supports_credentials()
            .max_age(3600);

        let mut app = App::new()
            .wrap(cors)
            .app_data(app_state.clone())
            .configure(routes::configure);

        if let Some(ref dir) = static_dir {
            let dir = dir.clone();
            app = app.default_service(
                actix_files::Files::new("/", &dir)
                    .index_file("index.html")
                    .default_handler(web::to(move || {
                        let dir = dir.clone();
                        async move {
                            let index = format!("{dir}/index.html");
                            actix_web::HttpResponse::Ok()
                                .content_type("text/html")
                                .body(
                                    std::fs::read_to_string(&index)
                                        .unwrap_or_else(|_| "index.html not found".to_string()),
                                )
                        }
                    })),
            );
        }

        app
    })
    .bind(format!("{}:{}", cfg.host, cfg.port))?
    .run()
    .await
}
