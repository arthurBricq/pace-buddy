use std::sync::Arc;

use auth::{JwtService, WebAuthnService};
use llm::open_router::OpenRouterClient;
use storage::SqliteStorage;
use strava_client::StravaClient;

pub struct AppState {
    pub storage: Arc<SqliteStorage>,
    pub strava_client: Arc<StravaClient>,
    pub webauthn: Arc<WebAuthnService>,
    pub jwt: Arc<JwtService>,
    pub frontend_url: String,
    pub llm_client: Option<Arc<OpenRouterClient>>,
    pub strava_webhook_verify_token: Option<String>,
    pub admin_strava_athlete_id: Option<i64>,
    pub quota_markup_ratio: f64,
}
