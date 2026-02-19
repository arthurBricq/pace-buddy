use std::env;
use std::fs;

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
    pub openrouter_api_key: Option<String>,
    pub strava_webhook_verify_token: Option<String>,
    pub base_url: Option<String>,
}

impl Config {
    pub fn from_env() -> Self {
        // BASE_URL unifies FRONTEND_URL, WEBAUTHN_RP_ORIGIN, STRAVA_REDIRECT_URI, and WEBAUTHN_RP_ID.
        // Example: BASE_URL=https://myapp.fly.dev
        // Individual env vars still take precedence for flexibility.
        let base_url = env::var("BASE_URL").ok();

        let default_origin = base_url
            .as_deref()
            .unwrap_or("https://pacebuddy:5173")
            .to_string();

        let default_rp_id = base_url
            .as_deref()
            .and_then(|u| u.split("://").nth(1))
            .map(|host| host.split(':').next().unwrap_or(host).to_string())
            .unwrap_or_else(|| "pacebuddy".to_string());

        let default_redirect_uri = base_url
            .as_deref()
            .map(|u| format!("{u}/api/strava/callback"))
            .unwrap_or_else(|| "http://localhost:8080/api/strava/callback".to_string());

        Self {
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:data.db?mode=rwc".to_string()),
            jwt_secret: env::var("JWT_SECRET")
                .unwrap_or_else(|_| "dev-secret-change-in-production".to_string()),
            webauthn_rp_id: env::var("WEBAUTHN_RP_ID").unwrap_or(default_rp_id),
            webauthn_rp_origin: env::var("WEBAUTHN_RP_ORIGIN").unwrap_or(default_origin.clone()),
            strava_client_id: env::var("STRAVA_CLIENT_ID").unwrap_or_else(|_| {
                fs::read_to_string("strava_client_id")
                    .unwrap_or_default()
                    .trim()
                    .to_string()
            }),
            strava_client_secret: env::var("STRAVA_CLIENT_SECRET").unwrap_or_else(|_| {
                fs::read_to_string("strava_client_secret")
                    .unwrap_or_default()
                    .trim()
                    .to_string()
            }),
            strava_redirect_uri: env::var("STRAVA_REDIRECT_URI").unwrap_or(default_redirect_uri),
            host: env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            port: env::var("PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .unwrap_or(8080),
            frontend_url: env::var("FRONTEND_URL").unwrap_or(default_origin),
            openrouter_api_key: env::var("OPENROUTER_API_KEY").ok().or_else(|| {
                fs::read_to_string("openrouter_key")
                    .ok()
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
            }),
            strava_webhook_verify_token: env::var("STRAVA_WEBHOOK_VERIFY_TOKEN").ok(),
            base_url,
        }
    }
}
