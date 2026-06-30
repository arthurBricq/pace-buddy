use sha2::{Digest, Sha256};
use uuid::Uuid;

use domain::invite_code::InviteCode;
use domain::DomainError;

pub fn normalize_invite_code(raw: &str) -> Result<String, DomainError> {
    let normalized = raw.trim().to_uppercase();
    if normalized.is_empty() {
        return Err(DomainError::BadRequest("Invite code is required".into()));
    }

    if !normalized
        .chars()
        .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit() || ch == '-')
    {
        return Err(DomainError::BadRequest(
            "Invite code format is invalid".into(),
        ));
    }

    Ok(normalized)
}

pub fn hash_invite_code(normalized_code: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(normalized_code.as_bytes());
    let digest = hasher.finalize();
    to_hex(&digest)
}

pub fn generate_invite_code() -> String {
    let raw = Uuid::new_v4().to_string().replace('-', "").to_uppercase();
    format!(
        "PB-{}-{}-{}-{}",
        &raw[0..4],
        &raw[4..8],
        &raw[8..12],
        &raw[12..16]
    )
}

pub fn invite_code_is_valid_for_redemption(invite: &InviteCode) -> bool {
    if invite.used_at.is_some() || invite.revoked_at.is_some() {
        return false;
    }

    if let Some(expires_at) = invite.expires_at {
        return expires_at > chrono::Utc::now();
    }

    true
}

fn to_hex(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(nibble_to_hex((b >> 4) & 0x0f));
        out.push(nibble_to_hex(b & 0x0f));
    }
    out
}

fn nibble_to_hex(nibble: u8) -> char {
    match nibble {
        0..=9 => (b'0' + nibble) as char,
        10..=15 => (b'a' + (nibble - 10)) as char,
        _ => '0',
    }
}
