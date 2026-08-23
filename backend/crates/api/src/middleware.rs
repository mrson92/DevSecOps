use axum::http::StatusCode;
use axum::http::Request;
use axum::body::Body;
use axum::RequestExt;
use axum_extra::headers::{Authorization, authorization::Bearer};
use axum_extra::TypedHeader;

pub async fn require_auth(
    mut req: Request<Body>,
    next: axum::middleware::Next,
) -> Result<axum::response::Response, StatusCode> {
    let TypedHeader(Authorization(bearer)) = req
        .extract_parts::<TypedHeader<Authorization<Bearer>>>()
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // TODO: Validate JWT token
    // For now, just check that a bearer token exists
    if bearer.token().is_empty() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(req).await)
}
