use crate::auth::decode_jwt;
use axum::{
    body::Body,
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
};
use uuid::Uuid;

pub async fn auth(mut req: Request<Body>, next: Next) -> Result<Response, StatusCode> {
    let header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let token = header
        .strip_prefix("Bearer ")
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let claims = decode_jwt(token).ok_or(StatusCode::UNAUTHORIZED)?;

    req.extensions_mut().insert::<Uuid>(claims.sub);

    Ok(next.run(req).await)
}
