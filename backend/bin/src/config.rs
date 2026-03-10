use std::env;
use std::fs;

pub struct Config {
    /// SQLite connection URL. Env: `DATABASE_URL`. Default: `sqlite:data.db?mode=rwc`.
    pub database_url: String,
    /// Secret used to sign and verify JWT session tokens. Env: `JWT_SECRET`.
    pub jwt_secret: String,
    /// Strava OAuth2 client ID. Env: `STRAVA_CLIENT_ID`, or read from `strava_client_id` file.
    pub strava_client_id: String,
    /// Strava OAuth2 client secret. Env: `STRAVA_CLIENT_SECRET`, or read from `strava_client_secret` file.
    pub strava_client_secret: String,
    /// Strava OAuth2 redirect URI. Env: `STRAVA_REDIRECT_URI`. Derived from `BASE_URL` if not set.
    pub strava_redirect_uri: String,
    /// Address the HTTP server binds to. Env: `HOST`. Default: `127.0.0.1`.
    pub host: String,
    /// Port the HTTP server listens on. Env: `PORT`. Default: `8080`.
    pub port: u16,
    /// Frontend origin URL used for CORS and redirects. Env: `FRONTEND_URL`.
    /// Derived from `BASE_URL` if not set.
    pub frontend_url: String,
    /// OpenRouter API key for LLM-powered insights. Env: `OPENROUTER_API_KEY`,
    /// or read from `openrouter_key` file. `None` disables LLM features.
    pub openrouter_api_key: Option<String>,
    /// Verification token for Strava webhook subscription validation.
    /// Env: `STRAVA_WEBHOOK_VERIFY_TOKEN`. `None` disables webhook setup.
    pub strava_webhook_verify_token: Option<String>,
    /// Unified base URL that derives defaults for `frontend_url` and `strava_redirect_uri`.
    /// Env: `BASE_URL`.
    pub base_url: Option<String>,
    /// Strava athlete ID of the admin user. Env: `ADMIN_STRAVA_ATHLETE_ID`.
    /// `None` disables the admin dashboard for all users.
    pub admin_strava_athlete_id: Option<i64>,
    /// Markup ratio applied to LLM costs when deducting from user quota.
    /// Env: `QUOTA_MARKUP_RATIO`.
    pub quota_markup_ratio: f64,
}

impl Config {
    pub fn from_env() -> Self {
        // BASE_URL unifies FRONTEND_URL and STRAVA_REDIRECT_URI.
        // Example: BASE_URL=https://myapp.fly.dev
        // Individual env vars still take precedence for flexibility.
        let base_url = env::var("BASE_URL").ok();

        let default_origin = base_url
            .as_deref()
            .unwrap_or("https://pace-buddy:5173")
            .to_string();

        let default_redirect_uri = base_url
            .as_deref()
            .map(|u| format!("{u}/api/strava/callback"))
            .unwrap_or_else(|| "https://pace-buddy:5173/api/strava/callback".to_string());

        Self {
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite:data.db?mode=rwc".to_string()),
            jwt_secret: env::var("JWT_SECRET")
                .unwrap_or_else(|_| "dev-secret-change-in-production".to_string()),
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
            admin_strava_athlete_id: env::var("ADMIN_STRAVA_ID")
                .ok()
                .and_then(|v| v.parse().ok()),
            quota_markup_ratio: env::var("QUOTA_MARKUP_RATIO")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(5.0),
        }
    }
}
