mod config;
mod errors;
mod helpers;
mod middleware;
mod routes;
mod state;

use std::sync::Arc;

use actix_cors::Cors;
use actix_web::{web, App, HttpServer};

use auth::{JwtService, WebAuthnService};
use storage::SqliteStorage;
use strava_client::StravaClient;

use crate::config::Config;
use crate::state::AppState;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::init();

    let cfg = Config::from_env();

    let storage = SqliteStorage::new(&cfg.database_url)
        .await
        .expect("Failed to initialize storage");

    let webauthn = WebAuthnService::new(&cfg.webauthn_rp_id, &cfg.webauthn_rp_origin)
        .expect("Failed to initialize WebAuthn");

    let jwt = JwtService::new(&cfg.jwt_secret);

    let strava_client = StravaClient::new(
        cfg.strava_client_id.clone(),
        cfg.strava_client_secret.clone(),
        cfg.strava_redirect_uri.clone(),
    );

    let app_state = web::Data::new(AppState {
        storage: Arc::new(storage),
        strava_client: Arc::new(strava_client),
        webauthn: Arc::new(webauthn),
        jwt: Arc::new(jwt),
        frontend_url: cfg.frontend_url.clone(),
    });

    log::info!("Starting server at {}:{}", cfg.host, cfg.port);

    HttpServer::new(move || {
        let cors = Cors::default()
            .allowed_origin(&cfg.frontend_url)
            .allow_any_method()
            .allow_any_header()
            .supports_credentials()
            .max_age(3600);

        App::new()
            .wrap(cors)
            .app_data(app_state.clone())
            .configure(routes::configure)
            .default_service(
                actix_files::Files::new("/", "./static")
                    .index_file("index.html")
                    .default_handler(
                        actix_web::web::to(|| async {
                            actix_web::HttpResponse::Ok()
                                .content_type("text/html")
                                .body(
                                    std::fs::read_to_string("./static/index.html")
                                        .unwrap_or_else(|_| "Frontend not built".to_string()),
                                )
                        }),
                    ),
            )
    })
    .bind(format!("{}:{}", cfg.host, cfg.port))?
    .run()
    .await
}
