use axum::{Json, Router, extract::State, http::StatusCode, routing::post};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth::{create_jwt, hash_password, verify_password};
use crate::models::{AuthResponse, LoginRequest, RegisterRequest};

pub fn auth_routes(pool: PgPool) -> Router {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .with_state(pool)
}

async fn register(
    State(pool): State<PgPool>,
    Json(payload): Json<RegisterRequest>,
) -> Result<StatusCode, StatusCode> {
    if payload.username.trim().is_empty() || payload.password.len() < 8 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let password_hash = hash_password(&payload.password);
    let user_id = Uuid::new_v4();

    sqlx::query!(
        "INSERT INTO users (id, username, password_hash) VALUES ($1, $2, $3)",
        user_id,
        payload.username,
        password_hash
    )
    .execute(&pool)
    .await
    .map_err(|_| StatusCode::CONFLICT)?;

    Ok(StatusCode::CREATED)
}

async fn login(
    State(pool): State<PgPool>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, StatusCode> {
    let user = sqlx::query!(
        "SELECT id, password_hash FROM users WHERE username = $1",
        payload.username
    )
    .fetch_optional(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .ok_or(StatusCode::UNAUTHORIZED)?;

    if !verify_password(&payload.password, &user.password_hash) {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let token = create_jwt(user.id);

    Ok(Json(AuthResponse { token }))
}
