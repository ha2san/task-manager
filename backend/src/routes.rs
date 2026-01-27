use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    http::StatusCode,
    routing::{get, post},
};
use sqlx::PgPool;
use uuid::Uuid;
use serde::Deserialize;

use crate::models::{CreateTaskRequest, UpdateTaskRequest, CreateSubtaskRequest, UpdateSubtaskRequest, ToggleSubtaskRequest};

pub fn routes(pool: PgPool) -> Router {
    Router::new()
        .route("/tasks", get(get_today_tasks).post(create_task))
        .route("/tasks/all", get(get_all_tasks))
        .route(
            "/tasks/:id",
            post(update_task).delete(delete_task).patch(toggle_archive),
        )
        .route("/tasks/:id/toggle", post(toggle_task))
        .route("/tasks/:id/subtasks", get(get_subtasks).post(create_subtask))
        .route("/tasks/:task_id/subtasks/:subtask_id", 
            post(update_subtask).delete(delete_subtask))
        .route("/subtasks/toggle", post(toggle_subtask))
        .route("/stats", get(get_stats))
        .route("/tasks/priorities", post(update_task_priorities))
        .with_state(pool)
}

// --- GESTIONNAIRES (HANDLERS) ---

/// Récupère les tâches prévues pour aujourd'hui avec leurs sous-tâches
pub async fn get_today_tasks(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<Uuid>,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    // Récupérer les tâches du jour avec ou sans completion
    let tasks = sqlx::query!(
        r#"
        SELECT t.id, t.title, t.active, 
               COALESCE(tc.completed, false) as "completed!",
               COALESCE(tc.priority, 0) as "priority!",
               EXISTS(SELECT 1 FROM subtasks s WHERE s.task_id = t.id) as "has_subtasks!"
        FROM tasks t
        JOIN task_days td ON t.id = td.task_id
        LEFT JOIN task_completions tc ON t.id = tc.task_id AND tc.date = current_date
        WHERE t.user_id = $1 
          AND td.day_of_week = extract(isodow from current_date)
          AND t.active = true 
          AND t.deleted = false
        ORDER BY tc.priority ASC, t.id ASC
        "#,
        user_id
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        eprintln!("Erreur lors de la récupération des tâches: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut result = Vec::new();
    
    for task in tasks {
        // Récupérer les sous-tâches SI la tâche en a
        let subtasks = if task.has_subtasks {
            sqlx::query!(
                r#"
                SELECT id, title, completed, priority
                FROM subtasks
                WHERE task_id = $1
                ORDER BY priority ASC, id ASC
                "#,
                task.id
            )
            .fetch_all(&pool)
            .await
            .map_err(|e| {
                eprintln!("Erreur lors de la récupération des sous-tâches: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
        } else {
            vec![]
        };

        // Calculer si la tâche est complétée (toutes les sous-tâches doivent être complétées)
        let all_subtasks_completed = !subtasks.is_empty() && subtasks.iter().all(|st| st.completed);
        let completed = if task.has_subtasks { 
            all_subtasks_completed 
        } else { 
            task.completed 
        };

        // Calculer le pourcentage de complétion des sous-tâches
        let subtask_completion = if subtasks.is_empty() {
            0
        } else {
            let completed_count = subtasks.iter().filter(|st| st.completed).count();
            (completed_count as f64 / subtasks.len() as f64 * 100.0) as i32
        };

        let task_json = serde_json::json!({
            "id": task.id,
            "title": task.title,
            "active": task.active,
            "completed": completed,
            "priority": task.priority,
            "has_subtasks": task.has_subtasks,
            "subtasks_count": subtasks.len(),
            "subtask_completion": subtask_completion,
            "subtasks": subtasks.into_iter().map(|st| serde_json::json!({
                "id": st.id,
                "title": st.title,
                "completed": st.completed,
                "priority": st.priority
            })).collect::<Vec<_>>()
        });
        
        result.push(task_json);
    }

    Ok(Json(result))
}

/// Crée une nouvelle tâche avec éventuellement des sous-tâches
pub async fn create_task(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<Uuid>,
    Json(payload): Json<CreateTaskRequest>,
) -> Result<StatusCode, StatusCode> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Créer la tâche principale
    let task = sqlx::query!(
        "INSERT INTO tasks (user_id, title) VALUES ($1, $2) RETURNING id",
        user_id,
        payload.title
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Ajouter les jours de récurrence
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

    // Ajouter les sous-tâches si elles existent
    if let Some(subtask_titles) = payload.subtasks {
        for (priority, title) in subtask_titles.into_iter().enumerate() {
            sqlx::query!(
                "INSERT INTO subtasks (task_id, title, priority) VALUES ($1, $2, $3)",
                task.id,
                title,
                priority as i32
            )
            .execute(&mut *tx)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
        
        // Marquer que la tâche a des sous-tâches
        sqlx::query!(
            "UPDATE tasks SET has_subtasks = true WHERE id = $1",
            task.id
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

/// Liste toutes les tâches avec leurs sous-tâches
pub async fn get_all_tasks(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<Uuid>,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    let rows = sqlx::query!(
        r#"
        SELECT t.id, t.title, t.active, t.has_subtasks,
               array_agg(DISTINCT td.day_of_week) as "days!"
        FROM tasks t
        LEFT JOIN task_days td ON t.id = td.task_id
        WHERE t.user_id = $1 AND t.deleted = false
        GROUP BY t.id
        ORDER BY t.id
        "#,
        user_id
    )
    .fetch_all(&pool)
    .await
    .map_err(|e| {
        eprintln!("Erreur récupération tâches: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    let mut result = Vec::new();
    
    for row in rows {
        // Récupérer les sous-tâches si la tâche en a
        let subtasks = if row.has_subtasks {
            sqlx::query!(
                "SELECT id, title, completed, priority FROM subtasks WHERE task_id = $1 ORDER BY priority ASC",
                row.id
            )
            .fetch_all(&pool)
            .await
            .map_err(|e| {
                eprintln!("Erreur récupération sous-tâches: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?
        } else {
            vec![]
        };

        let task_json = serde_json::json!({
            "id": row.id,
            "title": row.title,
            "active": row.active,
            "has_subtasks": row.has_subtasks,
            "days": row.days,
            "subtasks_count": subtasks.len(),
            "subtasks": subtasks.into_iter().map(|st| serde_json::json!({
                "id": st.id,
                "title": st.title,
                "completed": st.completed,
                "priority": st.priority
            })).collect::<Vec<_>>()
        });
        
        result.push(task_json);
    }

    Ok(Json(result))
}

/// Marque une tâche comme complétée ou non pour la journée actuelle
pub async fn toggle_task(
    Path(id): Path<i32>,
    State(pool): State<PgPool>,
    Extension(user_id): Extension<Uuid>,
) -> Result<StatusCode, StatusCode> {
    // Vérifier si la tâche a des sous-tâches
    let has_subtasks = sqlx::query!(
        "SELECT has_subtasks FROM tasks WHERE id = $1 AND user_id = $2",
        id,
        user_id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    match has_subtasks {
        Some(row) if row.has_subtasks => {
            // Si la tâche a des sous-tâches, basculer l'état de toutes les sous-tâches
            // Récupérer l'état actuel pour déterminer le nouvel état
            let current_completion = sqlx::query!(
                r#"
                SELECT tc.completed 
                FROM task_completions tc
                WHERE tc.task_id = $1 AND tc.date = current_date
                "#,
                id
            )
            .fetch_optional(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            let new_completed = match current_completion {
                Some(row) => !row.completed,
                None => true, // Si pas d'enregistrement, marquer comme complété
            };

            // Mettre à jour/marquer la tâche principale
            sqlx::query!(
                r#"
                INSERT INTO task_completions (task_id, date, completed)
                VALUES ($1, current_date, $2)
                ON CONFLICT (task_id, date) DO UPDATE SET completed = $2
                "#,
                id,
                new_completed
            )
            .execute(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

            // Mettre à jour toutes les sous-tâches
            sqlx::query!(
                "UPDATE subtasks SET completed = $1 WHERE task_id = $2",
                new_completed,
                id
            )
            .execute(&pool)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        }
        _ => {
            // Tâche sans sous-tâches, comportement normal
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
        }
    }

    Ok(StatusCode::OK)
}

/// Basculer l'état d'une sous-tâche et vérifier si la tâche parente est complète
pub async fn toggle_subtask(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<Uuid>,
    Json(payload): Json<ToggleSubtaskRequest>,
) -> Result<StatusCode, StatusCode> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Basculer l'état de la sous-tâche
    sqlx::query!(
        "UPDATE subtasks SET completed = NOT completed WHERE id = $1 AND task_id = $2",
        payload.subtask_id,
        payload.task_id
    )
    .execute(&mut *tx)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Vérifier si toutes les sous-tâches sont complétées
    let remaining_subtasks = sqlx::query!(
        "SELECT COUNT(*) as count FROM subtasks WHERE task_id = $1 AND completed = false",
        payload.task_id
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Mettre à jour l'état de la tâche parente
    let all_completed = remaining_subtasks.count.unwrap_or(0) == 0;
    
    sqlx::query!(
        r#"
        INSERT INTO task_completions (task_id, date, completed)
        VALUES ($1, current_date, $2)
        ON CONFLICT (task_id, date) DO UPDATE SET completed = $2
        "#,
        payload.task_id,
        all_completed
    )
    .execute(&mut *tx)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    tx.commit()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::OK)
}

/// Récupérer les sous-tâches d'une tâche
pub async fn get_subtasks(
    Path(task_id): Path<i32>,
    State(pool): State<PgPool>,
    Extension(user_id): Extension<Uuid>,
) -> Result<Json<Vec<serde_json::Value>>, StatusCode> {
    // Vérifier que l'utilisateur a accès à cette tâche
    let task_exists = sqlx::query!(
        "SELECT id FROM tasks WHERE id = $1 AND user_id = $2",
        task_id,
        user_id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if task_exists.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    let subtasks = sqlx::query!(
        "SELECT id, title, completed, priority FROM subtasks WHERE task_id = $1 ORDER BY priority ASC",
        task_id
    )
    .fetch_all(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let result = subtasks
        .into_iter()
        .map(|st| serde_json::json!({
            "id": st.id,
            "title": st.title,
            "completed": st.completed,
            "priority": st.priority
        }))
        .collect();

    Ok(Json(result))
}

/// Créer une sous-tâche pour une tâche existante
pub async fn create_subtask(
    Path(task_id): Path<i32>,
    State(pool): State<PgPool>,
    Extension(user_id): Extension<Uuid>,
    Json(payload): Json<CreateSubtaskRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let mut tx = pool
        .begin()
        .await
        .map_err(|e| {
            eprintln!("Erreur début transaction: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Vérifier que l'utilisateur a accès à cette tâche
    let task_exists = sqlx::query!(
        "SELECT id, has_subtasks FROM tasks WHERE id = $1 AND user_id = $2",
        task_id,
        user_id
    )
    .fetch_optional(&mut *tx)
    .await
    .map_err(|e| {
        eprintln!("Erreur vérification tâche: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?
    .ok_or(StatusCode::NOT_FOUND)?;

    // Trouver la priorité la plus élevée pour ajouter à la fin
    let max_priority = sqlx::query!(
        "SELECT COALESCE(MAX(priority), -1) as max_priority FROM subtasks WHERE task_id = $1",
        task_id
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        eprintln!("Erreur récupération priorité max: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Créer la sous-tâche
    let subtask = sqlx::query!(
        "INSERT INTO subtasks (task_id, title, priority) VALUES ($1, $2, $3) RETURNING id",
        task_id,
        payload.title,
        max_priority.max_priority.unwrap_or(-1) + 1
    )
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| {
        eprintln!("Erreur création sous-tâche: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    // Marquer que la tâche a des sous-tâches (si ce n'est pas déjà fait)
    if !task_exists.has_subtasks {
        sqlx::query!(
            "UPDATE tasks SET has_subtasks = true WHERE id = $1",
            task_id
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            eprintln!("Erreur mise à jour has_subtasks: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    }

    tx.commit()
        .await
        .map_err(|e| {
            eprintln!("Erreur commit transaction: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    Ok(Json(serde_json::json!({
        "success": true,
        "subtask_id": subtask.id,
        "message": "Sous-tâche créée avec succès"
    })))
}


/// Mettre à jour une sous-tâche
pub async fn update_subtask(
    Path((task_id, subtask_id)): Path<(i32, i32)>,
    State(pool): State<PgPool>,
    Extension(user_id): Extension<Uuid>,
    Json(payload): Json<UpdateSubtaskRequest>,
) -> Result<StatusCode, StatusCode> {
    // Vérifier que l'utilisateur a accès à cette tâche
    let task_exists = sqlx::query!(
        "SELECT id FROM tasks WHERE id = $1 AND user_id = $2",
        task_id,
        user_id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if task_exists.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    // Mettre à jour le titre si fourni
    if let Some(title) = payload.title {
        sqlx::query!(
            "UPDATE subtasks SET title = $1 WHERE id = $2 AND task_id = $3",
            title,
            subtask_id,
            task_id
        )
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    // Mettre à jour l'état de complétion si fourni
    if let Some(completed) = payload.completed {
        sqlx::query!(
            "UPDATE subtasks SET completed = $1 WHERE id = $2 AND task_id = $3",
            completed,
            subtask_id,
            task_id
        )
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // Vérifier si toutes les sous-tâches sont complétées
        let remaining_subtasks = sqlx::query!(
            "SELECT COUNT(*) as count FROM subtasks WHERE task_id = $1 AND completed = false",
            task_id
        )
        .fetch_one(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        // Mettre à jour l'état de la tâche parente
        let all_completed = remaining_subtasks.count.unwrap_or(0) == 0;
        
        sqlx::query!(
            r#"
            INSERT INTO task_completions (task_id, date, completed)
            VALUES ($1, current_date, $2)
            ON CONFLICT (task_id, date) DO UPDATE SET completed = $2
            "#,
            task_id,
            all_completed
        )
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    Ok(StatusCode::OK)
}

/// Supprimer une sous-tâche
pub async fn delete_subtask(
    Path((task_id, subtask_id)): Path<(i32, i32)>,
    State(pool): State<PgPool>,
    Extension(user_id): Extension<Uuid>,
) -> Result<StatusCode, StatusCode> {
    // Vérifier que l'utilisateur a accès à cette tâche
    let task_exists = sqlx::query!(
        "SELECT id FROM tasks WHERE id = $1 AND user_id = $2",
        task_id,
        user_id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if task_exists.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    // Supprimer la sous-tâche
    sqlx::query!(
        "DELETE FROM subtasks WHERE id = $1 AND task_id = $2",
        subtask_id,
        task_id
    )
    .execute(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Vérifier s'il reste des sous-tâches
    let remaining_subtasks = sqlx::query!(
        "SELECT COUNT(*) as count FROM subtasks WHERE task_id = $1",
        task_id
    )
    .fetch_one(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Si plus de sous-tâches, mettre à jour la tâche parente
    if remaining_subtasks.count.unwrap_or(0) == 0 {
        sqlx::query!(
            "UPDATE tasks SET has_subtasks = false WHERE id = $1",
            task_id
        )
        .execute(&pool)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    Ok(StatusCode::OK)
}

/// Met à jour les informations d'une tâche (titre, jours, statut et sous-tâches).
pub async fn update_task(
    Path(id): Path<i32>,
    State(pool): State<PgPool>,
    Extension(user_id): Extension<Uuid>,
    Json(payload): Json<UpdateTaskRequest>,
) -> Result<StatusCode, StatusCode> {
    // Vérifier que l'utilisateur a accès à cette tâche
    let task_exists = sqlx::query!(
        "SELECT id FROM tasks WHERE id = $1 AND user_id = $2",
        id,
        user_id
    )
    .fetch_optional(&pool)
    .await
    .map_err(|e| {
        eprintln!("Erreur vérification tâche: {}", e);
        StatusCode::INTERNAL_SERVER_ERROR
    })?;

    if task_exists.is_none() {
        return Err(StatusCode::NOT_FOUND);
    }

    // 1. Mise à jour du titre
    if let Some(title) = &payload.title {
        sqlx::query!("UPDATE tasks SET title = $1 WHERE id = $2", title, id)
            .execute(&pool)
            .await
            .map_err(|e| {
                eprintln!("Erreur mise à jour titre: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
    }

    // 2. Mise à jour du statut actif/archivé
    if let Some(active) = payload.active {
        sqlx::query!("UPDATE tasks SET active = $1 WHERE id = $2", active, id)
            .execute(&pool)
            .await
            .map_err(|e| {
                eprintln!("Erreur mise à jour statut: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
    }

    // 3. Mise à jour des jours
    if let Some(days) = &payload.days {
        let mut tx = pool
            .begin()
            .await
            .map_err(|e| {
                eprintln!("Erreur début transaction: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        
        sqlx::query!("DELETE FROM task_days WHERE task_id = $1", id)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                eprintln!("Erreur suppression jours: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        
        for day in days {
            sqlx::query!(
                "INSERT INTO task_days (task_id, day_of_week) VALUES ($1, $2)",
                id,
                day
            )
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                eprintln!("Erreur insertion jour {}: {}", day, e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
        }
        
        tx.commit()
            .await
            .map_err(|e| {
                eprintln!("Erreur commit transaction: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
    }

    // 4. Mise à jour des sous-tâches si fournies
    if let Some(subtasks) = &payload.subtasks {
        let mut tx = pool
            .begin()
            .await
            .map_err(|e| {
                eprintln!("Erreur début transaction sous-tâches: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        // Supprimer les sous-tâches existantes
        sqlx::query!("DELETE FROM subtasks WHERE task_id = $1", id)
            .execute(&mut *tx)
            .await
            .map_err(|e| {
                eprintln!("Erreur suppression sous-tâches: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;

        // Ajouter les nouvelles sous-tâches
        for (priority, subtask) in subtasks.iter().enumerate() {
            if !subtask.title.trim().is_empty() {
                sqlx::query!(
                    "INSERT INTO subtasks (task_id, title, priority) VALUES ($1, $2, $3)",
                    id,
                    subtask.title,
                    priority as i32
                )
                .execute(&mut *tx)
                .await
                .map_err(|e| {
                    eprintln!("Erreur insertion sous-tâche: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR
                })?;
            }
        }

        // Mettre à jour le flag has_subtasks
        let has_subtasks = !subtasks.is_empty();
        sqlx::query!(
            "UPDATE tasks SET has_subtasks = $1 WHERE id = $2",
            has_subtasks,
            id
        )
        .execute(&mut *tx)
        .await
        .map_err(|e| {
            eprintln!("Erreur mise à jour has_subtasks: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

        tx.commit()
            .await
            .map_err(|e| {
                eprintln!("Erreur commit sous-tâches: {}", e);
                StatusCode::INTERNAL_SERVER_ERROR
            })?;
    }

    Ok(StatusCode::OK)
}

/// Marque une tâche comme supprimée (Soft delete).
pub async fn delete_task(
    Path(id): Path<i32>, 
    State(pool): State<PgPool>, 
    Extension(user_id): Extension<Uuid>
) -> StatusCode {
    let _ = sqlx::query!(
        "UPDATE tasks SET deleted = true WHERE id = $1 AND user_id = $2",
        id,
        user_id
    )
    .execute(&pool)
    .await;
    StatusCode::OK
}

/// Active ou archive une tâche.
pub async fn toggle_archive(
    Path(id): Path<i32>, 
    State(pool): State<PgPool>, 
    Extension(user_id): Extension<Uuid>
) -> StatusCode {
    let _ = sqlx::query!(
        "UPDATE tasks SET active = NOT active WHERE id = $1 AND user_id = $2",
        id,
        user_id
    )
    .execute(&pool)
    .await;
    StatusCode::OK
}

/// Calcule les statistiques de complétion pour la heatmap des 30 derniers jours
pub async fn get_stats(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<Uuid>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    // 1. Requête pour la heatmap - compte maintenant les sous-tâches aussi
    let rows = sqlx::query!(
        r#"
        WITH day_series AS (
            SELECT generate_series(current_date - interval '29 days', current_date, '1 day')::date AS stats_date
        ),
        -- Compte toutes les tâches et sous-tâches prévues pour chaque jour
        scheduled_counts AS (
            SELECT 
                d.stats_date,
                COUNT(DISTINCT t.id) + COUNT(DISTINCT s.id) as total_scheduled
            FROM day_series d
            LEFT JOIN task_days td ON td.day_of_week = extract(isodow from d.stats_date)
            LEFT JOIN tasks t ON t.id = td.task_id AND t.user_id = $1 AND t.deleted = false AND t.active = true
            LEFT JOIN subtasks s ON s.task_id = t.id AND s.completed = false
            GROUP BY d.stats_date
        ),
        -- Compte toutes les complétions (tâches + sous-tâches) pour chaque jour
        completed_counts AS (
            SELECT 
                tc.date,
                COUNT(DISTINCT tc.task_id) + COUNT(DISTINCT s.id) as total_completed
            FROM task_completions tc
            LEFT JOIN tasks t ON t.id = tc.task_id AND t.user_id = $1
            LEFT JOIN subtasks s ON s.task_id = tc.task_id AND s.completed = true
            WHERE tc.completed = true
            GROUP BY tc.date
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
    .fetch_all(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // 2. Requête pour les totaux globaux CORRIGÉS
    let totals = sqlx::query!(
        r#"
        -- Total des tâches créées
        SELECT 
            (SELECT COUNT(*) FROM tasks WHERE user_id = $1 AND deleted = false) as total_tasks,
            -- Total des complétions (chaque jour où une tâche ou sous-tâche a été complétée)
            (SELECT COUNT(*) FROM task_completions tc 
             JOIN tasks t ON t.id = tc.task_id 
             WHERE t.user_id = $1 AND tc.completed = true) as total_completion_days,
            -- Total des jours où des tâches étaient prévues (pour calculer le taux)
            (SELECT COUNT(*) FROM (
                SELECT DISTINCT td.day_of_week, t.id
                FROM tasks t
                JOIN task_days td ON t.id = td.task_id
                WHERE t.user_id = $1 AND t.deleted = false AND t.active = true
            ) as scheduled) as total_scheduled_days
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

    // 3. Taux du jour (corrigé pour inclure les sous-tâches)
    let today_stats = sqlx::query!(
        r#"
        -- Tâches principales prévues aujourd'hui
        WITH scheduled_tasks AS (
            SELECT t.id
            FROM tasks t
            JOIN task_days td ON t.id = td.task_id
            WHERE t.user_id = $1 AND t.deleted = false AND t.active = true
                AND td.day_of_week = extract(isodow from current_date)
        ),
        -- Sous-tâches non complétées des tâches prévues
        scheduled_subtasks AS (
            SELECT s.id
            FROM scheduled_tasks st
            JOIN subtasks s ON s.task_id = st.id AND s.completed = false
        ),
        -- Tâches complétées aujourd'hui
        completed_tasks AS (
            SELECT tc.task_id
            FROM task_completions tc
            JOIN scheduled_tasks st ON st.id = tc.task_id
            WHERE tc.date = current_date AND tc.completed = true
        ),
        -- Sous-tâches complétées aujourd'hui
        completed_subtasks AS (
            SELECT s.id
            FROM scheduled_tasks st
            JOIN subtasks s ON s.task_id = st.id AND s.completed = true
        )
        SELECT 
            (SELECT COUNT(*) FROM scheduled_tasks) as scheduled_tasks,
            (SELECT COUNT(*) FROM scheduled_subtasks) as scheduled_subtasks,
            (SELECT COUNT(*) FROM completed_tasks) as completed_tasks,
            (SELECT COUNT(*) FROM completed_subtasks) as completed_subtasks
        "#,
        user_id
    )
    .fetch_one(&pool)
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let total_scheduled_today = (today_stats.scheduled_tasks.unwrap_or(0) + today_stats.scheduled_subtasks.unwrap_or(0)) as f64;
    let total_completed_today = (today_stats.completed_tasks.unwrap_or(0) + today_stats.completed_subtasks.unwrap_or(0)) as f64;

    let today_percent = if total_scheduled_today > 0.0 {
        (total_completed_today / total_scheduled_today * 100.0).round() as i32
    } else {
        0
    };

    // Calcul du taux de réussite corrigé
    let total_scheduled = totals.total_scheduled_days.unwrap_or(0) as f64;
    let total_completed = totals.total_completion_days.unwrap_or(0) as f64;
    
    let success_rate = if total_scheduled > 0.0 {
        ((total_completed / total_scheduled) * 100.0).round() as i32
    } else {
        0
    };

    Ok(Json(serde_json::json!({
        "history": history,
        "summary": {
            "total_created": totals.total_tasks,
            "total_completed_ever": totals.total_completion_days,
            "total_scheduled_days": totals.total_scheduled_days,
            "success_rate": success_rate,
            "today_percent": today_percent
        }
    })))
}

#[derive(Deserialize)]
pub struct UpdatePrioritiesRequest {
    pub ordered_task_ids: Vec<i32>,
}

pub async fn update_task_priorities(
    State(pool): State<PgPool>,
    Extension(user_id): Extension<Uuid>,
    Json(payload): Json<UpdatePrioritiesRequest>,
) -> Result<StatusCode, StatusCode> {
    let mut tx = pool.begin().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    for (priority, task_id) in payload.ordered_task_ids.iter().enumerate() {
        sqlx::query!(
            r#"
            INSERT INTO task_completions (task_id, date, completed, priority)
            VALUES ($1, current_date, COALESCE(
                (SELECT completed FROM task_completions WHERE task_id=$1 AND date=current_date), false), $2)
            ON CONFLICT (task_id, date)
            DO UPDATE SET priority = $2
            "#,
            task_id,
            priority as i32
        )
        .execute(&mut *tx)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    tx.commit().await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(StatusCode::OK)
}
