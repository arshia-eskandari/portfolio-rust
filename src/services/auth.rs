//! Authentication: password verification and the admin JWT cookie.
//!
//! Compatibility contract with the previous application:
//!
//! - stored passwords are unsalted hex-encoded SHA-512 digests
//!   (see `src/lib/password.ts` in the Next.js app) — verification here
//!   reproduces that exactly, so the existing admin account keeps working;
//! - the session token is an HS256 JWT signed with `JWT_SECRET`, carrying
//!   `userId`, `email`, `role`, `jti`, `iat`, `exp` claims with a one-hour
//!   lifetime, stored in an `HttpOnly` `token` cookie with
//!   `SameSite=Strict` (and `Secure` in production).

use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha512};
use subtle::ConstantTimeEq;

/// Cookie holding the admin JWT (same name as the previous application).
pub const AUTH_COOKIE: &str = "token";

/// Session lifetime in seconds (one hour, like the previous application).
pub const TOKEN_TTL_SECONDS: u64 = 3600;

/// Hex-encoded unsalted SHA-512 of `password` (legacy-compatible).
pub fn hash_password(password: &str) -> String {
    let digest = Sha512::digest(password.as_bytes());
    hex_encode(&digest)
}

/// Constant-time comparison of `password` against the stored hex digest.
pub fn verify_password(password: &str, stored_hex_digest: &str) -> bool {
    let computed = hash_password(password);
    // Compare the hex strings; both sides are the same length for genuine
    // digests, and ct_eq on unequal lengths returns false without leaking
    // a timing difference over the compared prefix.
    computed
        .as_bytes()
        .ct_eq(stored_hex_digest.as_bytes())
        .into()
}

fn hex_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        write!(out, "{byte:02x}").expect("writing to String cannot fail");
    }
    out
}

/// JWT claims, serialized with the exact names the previous `jose`
/// implementation used.
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    #[serde(rename = "userId")]
    pub user_id: String,
    pub email: String,
    pub role: String,
    pub jti: String,
    pub iat: u64,
    pub exp: u64,
}

/// Signs a one-hour admin session token.
pub fn create_token(
    secret: &str,
    user_id: &str,
    email: &str,
    role: &str,
) -> Result<String, jsonwebtoken::errors::Error> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock before UNIX epoch")
        .as_secs();
    let claims = Claims {
        user_id: user_id.to_string(),
        email: email.to_string(),
        role: role.to_string(),
        jti: uuid::Uuid::new_v4().to_string(),
        iat: now,
        exp: now + TOKEN_TTL_SECONDS,
    };
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

/// Verifies a token and returns its claims when it is a valid, unexpired
/// admin session. Non-admin roles are rejected here, server-side, exactly
/// like the previous middleware.
pub fn verify_admin_token(secret: &str, token: &str) -> Option<Claims> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;
    // `jose` did not set an issuer/audience; require none.
    validation.required_spec_claims = ["exp"].iter().map(|s| s.to_string()).collect();

    let data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .ok()?;
    if data.claims.role != "ADMIN" {
        return None;
    }
    Some(data.claims)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_password_like_the_legacy_node_implementation() {
        // node: crypto.createHash("sha512").update("secret").digest("hex")
        assert_eq!(
            hash_password("secret"),
            "bd2b1aaf7ef4f09be9f52ce2d8d599674d81aa9d6a4421696dc4d93dd0619d682ce56b4d64a9ef097761ced99e0f67265b5f76085e5b0ee7ca4696b2ad6fe2b2"
        );
    }

    #[test]
    fn verifies_correct_password_and_rejects_wrong_one() {
        let stored = hash_password("hunter2");
        assert!(verify_password("hunter2", &stored));
        assert!(!verify_password("hunter3", &stored));
        assert!(!verify_password("hunter2", "not-a-digest"));
    }

    #[test]
    fn token_roundtrip_accepts_admin() {
        let token = create_token("test-secret-test-secret", "abc123", "a@b.c", "ADMIN")
            .expect("token creation succeeds");
        let claims =
            verify_admin_token("test-secret-test-secret", &token).expect("valid admin token");
        assert_eq!(claims.user_id, "abc123");
        assert_eq!(claims.email, "a@b.c");
    }

    #[test]
    fn token_roundtrip_rejects_non_admin_and_bad_secret() {
        let token = create_token("test-secret-test-secret", "abc123", "a@b.c", "USER")
            .expect("token creation succeeds");
        assert!(verify_admin_token("test-secret-test-secret", &token).is_none());

        let admin = create_token("test-secret-test-secret", "abc123", "a@b.c", "ADMIN")
            .expect("token creation succeeds");
        assert!(verify_admin_token("other-secret-other-secret", &admin).is_none());
        assert!(verify_admin_token("test-secret-test-secret", "garbage").is_none());
    }
}
