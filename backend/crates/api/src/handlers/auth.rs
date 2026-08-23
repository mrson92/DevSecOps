use axum::extract::State;
use axum::http::HeaderMap;
use axum::Json;
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use aads_core::error::AppError;
use aads_core::state::AppState;

#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,
    pub email: Option<String>,
    pub preferred_username: Option<String>,
    pub realm_access: Option<RealmAccess>,
    pub exp: usize,
    pub iat: usize,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct RealmAccess {
    pub roles: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub code: String,
    pub redirect_uri: String,
}

pub async fn get_current_user(
    State(state): State<AppState>,
    headers: HeaderMap,
) -> Result<Json<Value>, AppError> {
    let token = extract_token(&headers)
        .ok_or_else(|| AppError::Unauthorized)?;

    let claims = verify_token(&token, &state.config.oidc.jwt_secret)
        .map_err(|_| AppError::Unauthorized)?;

    let roles = claims.realm_access
        .as_ref()
        .map(|ra| ra.roles.clone())
        .unwrap_or_default();

    Ok(Json(json!({
        "success": true,
        "data": {
            "id": claims.sub,
            "username": claims.preferred_username.unwrap_or_default(),
            "email": claims.email,
            "roles": roles
        }
    })))
}

pub async fn oidc_login(
    State(state): State<AppState>,
) -> Result<Json<Value>, AppError> {
    let oidc = &state.config.oidc;
    let realm = oidc.realm.as_deref().unwrap_or("master");
    let redirect_url = format!(
        "{}/realms/{}/protocol/openid-connect/auth?client_id={}&redirect_uri={}&response_type=code&scope=openid+profile+email",
        oidc.issuer_url, realm, oidc.client_id, oidc.redirect_url
    );

    Ok(Json(json!({
        "success": true,
        "data": { "redirect_url": redirect_url }
    })))
}

pub async fn oidc_callback(
    State(state): State<AppState>,
    Json(req): Json<LoginRequest>,
) -> Result<Json<Value>, AppError> {
    let oidc = &state.config.oidc;
    let realm = oidc.realm.as_deref().unwrap_or("master");

    let client = reqwest::Client::new();
    let token_url = format!(
        "{}/realms/{}/protocol/openid-connect/token",
        oidc.issuer_url, realm
    );

    let params = [
        ("grant_type", "authorization_code"),
        ("client_id", oidc.client_id.as_str()),
        ("client_secret", oidc.client_secret.as_str()),
        ("code", req.code.as_str()),
        ("redirect_uri", req.redirect_uri.as_str()),
    ];

    let resp = client.post(&token_url)
        .form(&params)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("OIDC token request failed: {}", e)))?;

    if !resp.status().is_success() {
        return Err(AppError::Unauthorized);
    }

    let token_data: serde_json::Value = resp.json().await
        .map_err(|e| AppError::Internal(format!("Failed to parse token response: {}", e)))?;

    let access_token = token_data["access_token"].as_str()
        .ok_or_else(|| AppError::Internal("No access_token in response".to_string()))?;

    let refresh_token = token_data["refresh_token"].as_str().map(|s| s.to_string());

    let claims = verify_token(access_token, &oidc.jwt_secret)
        .map_err(|_| AppError::Unauthorized)?;

    let roles = claims.realm_access
        .as_ref()
        .map(|ra| ra.roles.clone())
        .unwrap_or_default();

    Ok(Json(json!({
        "success": true,
        "data": {
            "access_token": access_token,
            "refresh_token": refresh_token,
            "expires_in": token_data["expires_in"].as_u64().unwrap_or(3600),
            "token_type": "Bearer",
            "user": {
                "id": claims.sub,
                "username": claims.preferred_username.unwrap_or_default(),
                "email": claims.email,
                "roles": roles
            }
        }
    })))
}

fn extract_token(headers: &HeaderMap) -> Option<String> {
    headers.get("authorization")?
        .to_str()
        .ok()?
        .strip_prefix("Bearer ")?
        .to_string()
        .into()
}

pub fn verify_token(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::default(),
    )?;
    Ok(token_data.claims)
}

pub fn create_token(claims: &Claims, secret: &str) -> Result<String, jsonwebtoken::errors::Error> {
    encode(
        &Header::default(),
        claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}
