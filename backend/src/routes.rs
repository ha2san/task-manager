use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::{CreateTaskRequest, UpdateTaskRequest};

/// Point d'entrée pour les routes de l'API des tâches.
/// Renommé en 'routes' pour correspondre à l'appel dans main.rs. [cite: 18]
pub fn routes(pool: PgPool) -> Router {
    Router::new()
        .route("/tasks", get(get_today_tasks).post(create_task))
        .route("/tasks/all", get(get_all_tasks))
        .route(
            "/tasks/:id",
            post(update_task).delete(delete_task).patch(toggle_archive),
        )
        .route("/tasks/:id/toggle", post(toggle_task))
        .route("/stats", get(get_stats))
        .with_state(pool)
}

// --- GESTIONNAIRES (HANDLERS) ---

/// Récupère les tâches prévues pour aujourd'hui pour l'utilisateur connecté.
pub async fn get_today_tasks(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<Uuid>, // Utilise Uuid comme défini dans le middleware [cite: 23]
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    let rows = sqlx::query!(
        r#"
        SELECT t.id, t.title, t.active, 
               COALESCE(tc.completed, false) as "completed!"
        FROM tasks t
        JOIN task_days td ON t.id = td.task_id
        LEFT JOIN task_completions tc ON t.id = tc.task_id AND tc.date = current_date
        WHERE t.user_id = $1 
          AND td.day_of_week = extract(isodow from current_date)
          AND t.active = true AND t.deleted = false
        "#,
        user_id
    )
    .fetch_all(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let tasks = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "title": r.title,
                "active": r.active,
                "completed": r.completed
            })
        })
        .collect();

    Ok(Json(tasks))
}

/// Crée une nouvelle tâche et définit ses jours de récurrence.
pub async fn create_task(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<Uuid>,
    Json(payload): Json<CreateTaskRequest>,
) -> Result<StatusCode, StatusCode> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let task = sqlx::query!(
        "INSERT INTO tasks (user_id, title) VALUES ($1, $2) RETURNING id",
        user_id,
        payload.title
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    for day in payload.days {
        sqlx::query!(
            "INSERT INTO task_days (task_id, day_of_week) VALUES ($1, $2)",
            task.id,
            day
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    tx.commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::CREATED)
}

/// Liste toutes les tâches de l'utilisateur (pour la page de gestion).
pub async fn get_all_tasks(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<Uuid>,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    let rows = sqlx::query!(
        r#"
        SELECT t.id, t.title, t.active, array_agg(td.day_of_week) as "days!"
        FROM tasks t
        LEFT JOIN task_days td ON t.id = td.task_id
        WHERE t.user_id = $1 AND t.deleted = false
        GROUP BY t.id
        "#,
        user_id
    )
    .fetch_all(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let tasks = rows
        .into_iter()
        .map(|r| {
            serde_json::json!({
                "id": r.id,
                "title": r.title,
                "active": r.active,
                "days": r.days
            })
        })
        .collect();

    Ok(Json(tasks))
}

/// Marque une tâche comme complétée ou non pour la journée actuelle.
pub async fn toggle_task(
    Path(id): Path<i32>,
    State(pool): State<PgPool>,
) -> Result<StatusCode, StatusCode> {
    sqlx::query!(
        r#"
        INSERT INTO task_completions (task_id, date, completed)
        VALUES ($1, current_date, true)
        ON CONFLICT (task_id, date) DO UPDATE SET completed = NOT task_completions.completed
        "#,
        id
    )
    .execute(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}

/// Met à jour les informations d'une tâche (titre ou jours).
pub async fn update_task(
    Path(id): Path<i32>,
    State(pool): State<PgPool>,
    Json(payload): Json<UpdateTaskRequest>,
) -> Result<StatusCode, StatusCode> {
    // 1. Mise à jour du titre
    if let Some(title) = payload.title {
        sqlx::query!("UPDATE tasks SET title = $1 WHERE id = $2", title, id)
            .execute(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    // 2. Mise à jour du statut actif/archivé (Correction du warning "never read")
    if let Some(active) = payload.active {
        sqlx::query!("UPDATE tasks SET active = $1 WHERE id = $2", active, id)
            .execute(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    // 3. Mise à jour des jours
    if let Some(days) = payload.days {
        let mut tx = pool
            .begin()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        sqlx::query!("DELETE FROM task_days WHERE task_id = $1", id)
            .execute(&mut *tx)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        for day in days {
            sqlx::query!(
                "INSERT INTO task_days (task_id, day_of_week) VALUES ($1, $2)",
                id,
                day
            )
            .execute(&mut *tx)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
        tx.commit()
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    Ok(StatusCode::OK)
}

/// Marque une tâche comme supprimée (Soft delete).
pub async fn delete_task(Path(id): Path<i32>, State(pool): State<PgPool>) -> StatusCode {
    let _ = sqlx::query!("UPDATE tasks SET deleted = true WHERE id = $1", id)
        .execute(&pool)
        .await;
    StatusCode::OK
}

/// Active ou archive une tâche.
pub async fn toggle_archive(Path(id): Path<i32>, State(pool): State<PgPool>) -> StatusCode {
    let _ = sqlx::query!("UPDATE tasks SET active = NOT active WHERE id = $1", id)
        .execute(&pool)
        .await;
    StatusCode::OK
}

/// Calcule les statistiques de complétion pour la heatmap des 30 derniers jours.
// Dans get_stats, modifiez la structure de retour
pub async fn get_stats(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<Uuid>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // 1. Requête pour la heatmap (identique à avant)
    let rows = sqlx::query!(
        r#"
        WITH day_series AS (
            SELECT generate_series(current_date - interval '29 days', current_date, '1 day')::date AS stats_date
        ),
        scheduled_counts AS (
            SELECT 
                d.stats_date,
                COUNT(td.task_id) as total_scheduled
            FROM day_series d
            LEFT JOIN task_days td ON td.day_of_week = extract(isodow from d.stats_date)
            LEFT JOIN tasks t ON t.id = td.task_id AND t.user_id = $1 AND t.deleted = false AND t.active = true
            GROUP BY d.stats_date
        ),
        completed_counts AS (
            SELECT 
                date,
                COUNT(task_id) as total_completed
            FROM task_completions
            WHERE task_id IN (SELECT id FROM tasks WHERE user_id = $1) AND completed = true
            GROUP BY date
        )
        SELECT 
            s.stats_date as "date!",
            COALESCE(c.total_completed, 0)::int as "completed!",
            COALESCE(s.total_scheduled, 0)::int as "scheduled!"
        FROM scheduled_counts s
        LEFT JOIN completed_counts c ON c.date = s.stats_date
        ORDER BY s.stats_date ASC
        "#,
        user_id
    )
    .fetch_all(&pool).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 2. Requête pour les totaux globaux
    let totals = sqlx::query!(
        r#"
        SELECT 
            (SELECT COUNT(*) FROM tasks WHERE user_id = $1 AND deleted = false) as total_tasks,
            (SELECT COUNT(*) FROM task_completions tc 
             JOIN tasks t ON t.id = tc.task_id 
             WHERE t.user_id = $1 AND tc.completed = true) as total_done
        "#,
        user_id
    )
    .fetch_one(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let history: Vec<serde_json::Value> = rows
        .into_iter()
        .map(|r| {
            let percent = if r.scheduled > 0 {
                (r.completed as f64 / r.scheduled as f64 * 100.0) as i32
            } else {
                0
            };
            serde_json::json!({ "date": r.date, "percent": percent })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "history": history,
        "summary": {
            "total_created": totals.total_tasks,
            "total_completed_ever": totals.total_done
        }
    })))
}
