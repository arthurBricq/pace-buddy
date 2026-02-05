use std::collections::HashMap;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use url::Url;
use uuid::Uuid;
use webauthn_rs::prelude::*;
use webauthn_rs::Webauthn;
use webauthn_rs::WebauthnBuilder;

use domain::DomainError;

/// Time-to-live for challenge state entries (5 minutes).
const CHALLENGE_TTL_SECS: u64 = 300;

pub struct WebAuthnService {
    webauthn: Arc<Webauthn>,
    reg_state: Arc<Mutex<HashMap<Uuid, (PasskeyRegistration, Instant)>>>,
    auth_state: Arc<Mutex<HashMap<Uuid, (PasskeyAuthentication, Instant)>>>,
}

impl WebAuthnService {
    /// Create a new WebAuthnService.
    ///
    /// - `rp_id`: the relying party identifier, typically the domain (e.g. "example.com").
    /// - `rp_origin`: the full origin URL (e.g. "https://example.com").
    pub fn new(rp_id: &str, rp_origin: &str) -> Result<Self, DomainError> {
        let origin = Url::parse(rp_origin)
            .map_err(|e| DomainError::Auth(format!("Invalid rp_origin URL: {e}")))?;

        let webauthn = WebauthnBuilder::new(rp_id, &origin)
            .map_err(|e| DomainError::Auth(format!("WebauthnBuilder error: {e}")))?
            .rp_name(rp_id)
            .build()
            .map_err(|e| DomainError::Auth(format!("Webauthn build error: {e}")))?;

        Ok(Self {
            webauthn: Arc::new(webauthn),
            reg_state: Arc::new(Mutex::new(HashMap::new())),
            auth_state: Arc::new(Mutex::new(HashMap::new())),
        })
    }

    /// Begin passkey registration for the given user.
    ///
    /// Returns the `CreationChallengeResponse` that must be sent to the client.
    /// The registration state is stored internally, keyed by `user_id`.
    pub async fn start_registration(
        &self,
        user_id: Uuid,
        username: &str,
    ) -> Result<CreationChallengeResponse, DomainError> {
        // Lazily clean up expired state entries.
        self.cleanup_expired().await;

        let (ccr, reg_state) = self
            .webauthn
            .start_passkey_registration(user_id, username, username, None)
            .map_err(|e| DomainError::Auth(e.to_string()))?;

        self.reg_state
            .lock()
            .await
            .insert(user_id, (reg_state, Instant::now()));

        Ok(ccr)
    }

    /// Complete passkey registration.
    ///
    /// Retrieves (and removes) the stored registration state for `user_id`,
    /// then verifies the client's `RegisterPublicKeyCredential`.
    /// Returns the resulting `Passkey` on success.
    pub async fn finish_registration(
        &self,
        user_id: Uuid,
        reg: &RegisterPublicKeyCredential,
    ) -> Result<Passkey, DomainError> {
        let (state, _ts) = self
            .reg_state
            .lock()
            .await
            .remove(&user_id)
            .ok_or_else(|| {
                DomainError::Auth("No pending registration state for this user".to_string())
            })?;

        let passkey = self
            .webauthn
            .finish_passkey_registration(reg, &state)
            .map_err(|e| DomainError::Auth(e.to_string()))?;

        Ok(passkey)
    }

    /// Begin passkey authentication for the given user.
    ///
    /// `passkeys` should be the user's stored passkeys.
    /// Returns the `RequestChallengeResponse` that must be sent to the client.
    pub async fn start_authentication(
        &self,
        user_id: Uuid,
        passkeys: &[Passkey],
    ) -> Result<RequestChallengeResponse, DomainError> {
        // Lazily clean up expired state entries.
        self.cleanup_expired().await;

        let (rcr, auth_state) = self
            .webauthn
            .start_passkey_authentication(passkeys)
            .map_err(|e| DomainError::Auth(e.to_string()))?;

        self.auth_state
            .lock()
            .await
            .insert(user_id, (auth_state, Instant::now()));

        Ok(rcr)
    }

    /// Complete passkey authentication.
    ///
    /// Retrieves (and removes) the stored authentication state for `user_id`,
    /// then verifies the client's `PublicKeyCredential`.
    /// Returns the `AuthenticationResult` on success.
    pub async fn finish_authentication(
        &self,
        user_id: Uuid,
        auth: &PublicKeyCredential,
    ) -> Result<AuthenticationResult, DomainError> {
        let (state, _ts) = self
            .auth_state
            .lock()
            .await
            .remove(&user_id)
            .ok_or_else(|| {
                DomainError::Auth("No pending authentication state for this user".to_string())
            })?;

        let result = self
            .webauthn
            .finish_passkey_authentication(auth, &state)
            .map_err(|e| DomainError::Auth(e.to_string()))?;

        Ok(result)
    }

    /// Remove entries older than 5 minutes from both the registration and
    /// authentication state maps.
    pub async fn cleanup_expired(&self) {
        let cutoff = Instant::now() - std::time::Duration::from_secs(CHALLENGE_TTL_SECS);

        {
            let mut map = self.reg_state.lock().await;
            map.retain(|_k, (_state, ts)| *ts > cutoff);
        }

        {
            let mut map = self.auth_state.lock().await;
            map.retain(|_k, (_state, ts)| *ts > cutoff);
        }
    }
}
