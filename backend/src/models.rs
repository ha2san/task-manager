use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

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

// Structure pour les Tâches
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
    #[sqlx(default)]
    pub has_subtasks: bool,
    #[sqlx(default)]
    pub subtasks: Vec<Subtask>,
}

// Structure pour les Sous-tâches
#[derive(Serialize, Deserialize, FromRow, Clone)]
pub struct Subtask {
    pub id: i32,
    pub task_id: i32,
    pub title: String,
    pub completed: bool,
    pub priority: i32,
}

#[derive(Deserialize)]
pub struct CreateTaskRequest {
    pub title: String,
    pub days: Vec<i32>,
    pub subtasks: Option<Vec<String>>,
}

#[derive(Deserialize)]
pub struct UpdateTaskRequest {
    pub title: Option<String>,
    pub days: Option<Vec<i32>>,
    pub active: Option<bool>,
    pub subtasks: Option<Vec<SubtaskUpdate>>, // Nouveau champ pour les sous-tâches
}

#[derive(Deserialize, Clone)]
pub struct SubtaskUpdate {
    pub id: Option<i32>, // Optionnel pour les nouvelles sous-tâches
    pub title: String,
    pub completed: Option<bool>,
}

#[derive(Deserialize)]
pub struct CreateSubtaskRequest {
    pub title: String,
}

#[derive(Deserialize)]
pub struct UpdateSubtaskRequest {
    pub completed: Option<bool>,
    pub title: Option<String>,
}

#[derive(Deserialize)]
pub struct ToggleSubtaskRequest {
    pub task_id: i32,
    pub subtask_id: i32,
}

#[derive(Deserialize)]
pub struct UpdatePrioritiesRequest {
    pub ordered_task_ids: Vec<i32>,
}
