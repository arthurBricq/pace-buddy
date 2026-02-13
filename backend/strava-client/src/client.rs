use domain::DomainError;
use reqwest::Client;

use crate::types::*;

pub struct StravaClient {
    client: Client,
    client_id: String,
    client_secret: String,
    redirect_uri: String,
}

impl StravaClient {
    pub fn new(client_id: String, client_secret: String, redirect_uri: String) -> Self {
        Self {
            client: Client::new(),
            client_id,
            client_secret,
            redirect_uri,
        }
    }

    pub fn is_configured(&self) -> bool {
        !self.client_id.is_empty() && !self.client_secret.is_empty()
    }

    pub fn authorize_url(&self, state: &str) -> String {
        format!(
            "https://www.strava.com/oauth/authorize?client_id={}&redirect_uri={}&response_type=code&scope=read,activity:read_all&approval_prompt=auto&state={state}",
            self.client_id,
            self.redirect_uri,
        )
    }

    pub async fn exchange_code(&self, code: &str) -> Result<TokenResponse, DomainError> {
        let resp = self
            .client
            .post("https://www.strava.com/oauth/token")
            .form(&[
                ("client_id", self.client_id.as_str()),
                ("client_secret", self.client_secret.as_str()),
                ("code", code),
                ("grant_type", "authorization_code"),
            ])
            .send()
            .await
            .map_err(|e| DomainError::StravaApi(e.to_string()))?;

        if resp.status() == 429 {
            return Err(DomainError::StravaRateLimited);
        }
        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(DomainError::StravaApi(format!(
                "Token exchange failed: {text}"
            )));
        }

        resp.json()
            .await
            .map_err(|e| DomainError::StravaApi(e.to_string()))
    }

    pub async fn refresh_token(&self, refresh_token: &str) -> Result<TokenResponse, DomainError> {
        let resp = self
            .client
            .post("https://www.strava.com/oauth/token")
            .form(&[
                ("client_id", self.client_id.as_str()),
                ("client_secret", self.client_secret.as_str()),
                ("refresh_token", refresh_token),
                ("grant_type", "refresh_token"),
            ])
            .send()
            .await
            .map_err(|e| DomainError::StravaApi(e.to_string()))?;

        if resp.status() == 429 {
            return Err(DomainError::StravaRateLimited);
        }
        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(DomainError::StravaApi(format!(
                "Token refresh failed: {text}"
            )));
        }

        resp.json()
            .await
            .map_err(|e| DomainError::StravaApi(e.to_string()))
    }

    /// Fetch activities from Strava. page is 1-indexed. per_page max 200.
    pub async fn get_activities(
        &self,
        access_token: &str,
        page: u32,
        per_page: u32,
        after: Option<i64>,
        before: Option<i64>,
    ) -> Result<Vec<StravaActivity>, DomainError> {
        let mut url = format!(
            "https://www.strava.com/api/v3/athlete/activities?page={page}&per_page={per_page}"
        );
        if let Some(after) = after {
            url.push_str(&format!("&after={after}"));
        }
        if let Some(before) = before {
            url.push_str(&format!("&before={before}"));
        }

        let resp = self
            .client
            .get(&url)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| DomainError::StravaApi(e.to_string()))?;

        if resp.status() == 429 {
            return Err(DomainError::StravaRateLimited);
        }
        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(DomainError::StravaApi(format!(
                "Get activities failed: {text}"
            )));
        }

        resp.json()
            .await
            .map_err(|e| DomainError::StravaApi(e.to_string()))
    }

    /// Fetch a single activity from Strava (used for on-demand polyline)
    pub async fn get_activity(
        &self,
        access_token: &str,
        activity_id: i64,
    ) -> Result<StravaActivity, DomainError> {
        let url = format!("https://www.strava.com/api/v3/activities/{activity_id}");

        let resp = self
            .client
            .get(&url)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| DomainError::StravaApi(e.to_string()))?;

        if resp.status() == 429 {
            return Err(DomainError::StravaRateLimited);
        }
        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(DomainError::StravaApi(format!(
                "Get activity failed: {text}"
            )));
        }

        resp.json()
            .await
            .map_err(|e| DomainError::StravaApi(e.to_string()))
    }

    /// Fetch streams for a specific activity
    pub async fn get_activity_streams(
        &self,
        access_token: &str,
        activity_id: i64,
    ) -> Result<Vec<StravaStream>, DomainError> {
        let url = format!(
            "https://www.strava.com/api/v3/activities/{activity_id}/streams?keys=time,distance,latlng,altitude,heartrate,cadence,watts,velocity_smooth,moving&key_by_type=true"
        );

        let resp = self
            .client
            .get(&url)
            .bearer_auth(access_token)
            .send()
            .await
            .map_err(|e| DomainError::StravaApi(e.to_string()))?;

        if resp.status() == 429 {
            return Err(DomainError::StravaRateLimited);
        }
        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(DomainError::StravaApi(format!(
                "Get streams failed: {text}"
            )));
        }

        let text = resp
            .text()
            .await
            .map_err(|e| DomainError::StravaApi(format!("Failed to read response body: {e}")))?;

        parse_streams_response(&text).map_err(DomainError::StravaApi)
    }
}
