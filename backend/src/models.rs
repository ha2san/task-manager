use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

// Utilisé dans routes_auth.rs (si vous voulez typer vos retours SQL)
#[derive(FromRow)]
pub struct User {
    pub id: Uuid,
    pub username: String,
    pub password_hash: String,
}

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct AuthResponse {
    pub token: String,
}

// Structure unique pour les Tâches
#[derive(Serialize, Deserialize, FromRow)]
pub struct Task {
    pub id: i32,
    pub title: String,
    pub active: bool,
    #[sqlx(default)]
    pub days: Vec<i32>,
    #[sqlx(default)]
    pub completed: bool,
    #[sqlx(default)]
    pub priority: i32,
}

#[derive(Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub days: Vec<i32>,
}

#[derive(Deserialize)]
pub struct UpdateTaskRequest {
    pub title: Option<String>,
    pub days: Option<Vec<i32>>,
    pub active: Option<bool>,
}
