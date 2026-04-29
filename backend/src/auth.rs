use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::{Duration, Utc};
use hmac::{Hmac, Mac};
use poem::http::{HeaderMap, StatusCode, header::AUTHORIZATION};
use serde::{Deserialize, Serialize};
use sha2::Sha256;

use crate::domain::RegisterRequest;

const TOKEN_VERSION: &str = "primer-v1";
const DEFAULT_ACTIVATION_CODE: &str = "X4G6S2HjK";
const DEFAULT_SESSION_TTL_SECONDS: i64 = 60 * 60 * 24 * 7;
const MIN_SESSION_SECRET_LEN: usize = 32;
const DEV_SESSION_SECRET: &str = "primerlab-dev-session-secret-change-before-production-2026";

type HmacSha256 = Hmac<Sha256>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthenticatedStudent {
    pub student_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthError {
    pub status: StatusCode,
    pub message: &'static str,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SessionClaims {
    student_id: String,
    exp: i64,
}

pub fn validate_activation_code(request: &RegisterRequest) -> Result<(), AuthError> {
    let Some(code) = request
        .activation_code
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Err(forbidden("Activation code is required."));
    };

    if constant_time_eq(code.as_bytes(), expected_activation_code().as_bytes()) {
        Ok(())
    } else {
        Err(forbidden("Activation code is invalid."))
    }
}

pub fn session_for_student(student_id: &str) -> serde_json::Value {
    serde_json::json!({
        "token": issue_session_token(student_id),
        "type": "signed"
    })
}

pub fn issue_session_token(student_id: &str) -> String {
    let claims = SessionClaims {
        student_id: student_id.to_string(),
        exp: (Utc::now() + Duration::seconds(session_ttl_seconds())).timestamp(),
    };
    let payload = serde_json::to_vec(&claims).expect("session claims serialize");
    let payload = URL_SAFE_NO_PAD.encode(payload);
    let signed_part = format!("{TOKEN_VERSION}.{payload}");
    let signature = URL_SAFE_NO_PAD.encode(sign_bytes(signed_part.as_bytes()));

    format!("{signed_part}.{signature}")
}

pub fn authenticate(headers: &HeaderMap) -> Result<AuthenticatedStudent, AuthError> {
    let token = bearer_token(headers)?;
    let claims = verify_session_token(token)?;

    Ok(AuthenticatedStudent {
        student_id: claims.student_id,
    })
}

pub fn authorize_student(
    authenticated: &AuthenticatedStudent,
    requested_student_id: &str,
) -> Result<(), AuthError> {
    if authenticated.student_id == requested_student_id {
        Ok(())
    } else {
        Err(forbidden("You are not authorized to access that student."))
    }
}

pub fn authorized_student_id(
    authenticated: &AuthenticatedStudent,
    requested_student_id: Option<&str>,
) -> Result<String, AuthError> {
    match requested_student_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        Some(student_id) => {
            authorize_student(authenticated, student_id)?;
            Ok(student_id.to_string())
        }
        None => Ok(authenticated.student_id.clone()),
    }
}

fn verify_session_token(token: &str) -> Result<SessionClaims, AuthError> {
    let mut parts = token.split('.');
    let version = parts.next();
    let payload = parts.next();
    let signature = parts.next();

    if version != Some(TOKEN_VERSION)
        || payload.is_none()
        || signature.is_none()
        || parts.next().is_some()
    {
        return Err(unauthorized("Bearer token is invalid."));
    }

    let payload = payload.expect("checked above");
    let signature = signature.expect("checked above");
    let signed_part = format!("{TOKEN_VERSION}.{payload}");
    let expected_signature = URL_SAFE_NO_PAD.encode(sign_bytes(signed_part.as_bytes()));
    if !constant_time_eq(signature.as_bytes(), expected_signature.as_bytes()) {
        return Err(unauthorized("Bearer token is invalid."));
    }

    let claims: SessionClaims = serde_json::from_slice(
        &URL_SAFE_NO_PAD
            .decode(payload)
            .map_err(|_| unauthorized("Bearer token is invalid."))?,
    )
    .map_err(|_| unauthorized("Bearer token is invalid."))?;

    if claims.student_id.trim().is_empty() || claims.exp <= Utc::now().timestamp() {
        return Err(unauthorized("Bearer token is expired or invalid."));
    }

    Ok(claims)
}

fn bearer_token(headers: &HeaderMap) -> Result<&str, AuthError> {
    let Some(value) = headers.get(AUTHORIZATION) else {
        return Err(unauthorized("Authorization bearer token is required."));
    };
    let value = value
        .to_str()
        .map_err(|_| unauthorized("Authorization bearer token is invalid."))?;
    value
        .strip_prefix("Bearer ")
        .map(str::trim)
        .filter(|token| !token.is_empty())
        .ok_or_else(|| unauthorized("Authorization bearer token is required."))
}

fn sign_bytes(value: &[u8]) -> Vec<u8> {
    let mut mac = HmacSha256::new_from_slice(&session_secret_bytes()).expect("HMAC accepts key");
    mac.update(value);
    mac.finalize().into_bytes().to_vec()
}

fn session_secret_bytes() -> Vec<u8> {
    if let Some(secret) = std::env::var("PRIMER_SESSION_SECRET")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| value.len() >= MIN_SESSION_SECRET_LEN)
    {
        return secret.into_bytes();
    }

    if cfg!(debug_assertions) {
        return DEV_SESSION_SECRET.as_bytes().to_vec();
    }

    panic!("PRIMER_SESSION_SECRET must be set to at least 32 characters");
}

fn session_ttl_seconds() -> i64 {
    std::env::var("PRIMER_SESSION_TTL_SECONDS")
        .ok()
        .and_then(|value| value.parse::<i64>().ok())
        .filter(|value| *value > 0)
        .unwrap_or(DEFAULT_SESSION_TTL_SECONDS)
}

fn expected_activation_code() -> String {
    std::env::var("PRIMER_ACTIVATION_CODE")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_ACTIVATION_CODE.to_string())
}

fn unauthorized(message: &'static str) -> AuthError {
    AuthError {
        status: StatusCode::UNAUTHORIZED,
        message,
    }
}

fn forbidden(message: &'static str) -> AuthError {
    AuthError {
        status: StatusCode::FORBIDDEN,
        message,
    }
}

fn constant_time_eq(left: &[u8], right: &[u8]) -> bool {
    let max_len = left.len().max(right.len());
    let mut diff = left.len() ^ right.len();

    for index in 0..max_len {
        let left_byte = left.get(index).copied().unwrap_or(0);
        let right_byte = right.get(index).copied().unwrap_or(0);
        diff |= usize::from(left_byte ^ right_byte);
    }

    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use poem::http::HeaderValue;

    fn register_request(activation_code: Option<&str>) -> RegisterRequest {
        RegisterRequest {
            username: "learner".to_string(),
            password: "correct horse battery staple".to_string(),
            activation_code: activation_code.map(ToString::to_string),
            display_name: Some("Learner".to_string()),
            age: Some(11),
            age_band: None,
            biography: Some("Curious about reefs.".to_string()),
            interests: vec!["marine biology".to_string()],
        }
    }

    #[test]
    fn activation_code_is_required_and_exact() {
        assert_eq!(
            validate_activation_code(&register_request(None))
                .unwrap_err()
                .status,
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            validate_activation_code(&register_request(Some("wrong")))
                .unwrap_err()
                .status,
            StatusCode::FORBIDDEN
        );
        assert!(validate_activation_code(&register_request(Some("X4G6S2HjK"))).is_ok());
    }

    #[test]
    fn signed_session_tokens_authenticate_and_are_not_predictable() {
        let token = issue_session_token("student-123");
        assert!(!token.contains("local-demo"));
        assert!(!token.ends_with("student-123"));

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
        );
        assert_eq!(
            authenticate(&headers).unwrap(),
            AuthenticatedStudent {
                student_id: "student-123".to_string()
            }
        );
    }

    #[test]
    fn tampered_session_tokens_are_rejected() {
        let mut token = issue_session_token("student-123");
        token.push('x');

        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
        );
        assert_eq!(
            authenticate(&headers).unwrap_err().status,
            StatusCode::UNAUTHORIZED
        );
    }
}
