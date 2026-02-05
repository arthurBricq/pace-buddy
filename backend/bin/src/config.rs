use std::env;

pub struct Config {
    pub database_url: String,
    pub jwt_secret: String,
    pub webauthn_rp_id: String,
    pub webauthn_rp_origin: String,
    pub strava_client_id: String,
    pub strava_client_secret: String,
    pub strava_redirect_uri: String,
    pub host: String,
    pub port: u16,
    pub frontend_url: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:data.db?mode=rwc".to_string()),
            jwt_secret: env::var("JWT_SECRET")
                .unwrap_or_else(|_| "dev-secret-change-in-production".to_string()),
            webauthn_rp_id: env::var("WEBAUTHN_RP_ID")
                .unwrap_or_else(|_| "localhost".to_string()),
            webauthn_rp_origin: env::var("WEBAUTHN_RP_ORIGIN")
                .unwrap_or_else(|_| "http://localhost:5173".to_string()),
            strava_client_id: env::var("STRAVA_CLIENT_ID")
                .unwrap_or_default(),
            strava_client_secret: env::var("STRAVA_CLIENT_SECRET")
                .unwrap_or_default(),
            strava_redirect_uri: env::var("STRAVA_REDIRECT_URI")
                .unwrap_or_else(|_| "http://localhost:8080/api/strava/callback".to_string()),
            host: env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .unwrap_or(8080),
            frontend_url: env::var("FRONTEND_URL")
                .unwrap_or_else(|_| "http://localhost:5173".to_string()),
        }
    }
}
