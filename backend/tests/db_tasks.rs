use chrono::{Datelike, Local};
use sqlx::PgPool;

/// Helper pour appliquer les migrations avant chaque test
async fn apply_migrations(pool: &PgPool) {
    sqlx::migrate!("./migrations") // chemin **absolu** relatif au Cargo.toml
        .run(pool)
        .await
        .unwrap();
}

#[sqlx::test]
async fn create_task_works(pool: PgPool) {
    apply_migrations(&pool).await;

    let rec = sqlx::query!(
        "INSERT INTO tasks (title) VALUES ($1) RETURNING id",
        "Test task"
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(rec.id > 0);
}

#[sqlx::test]
async fn task_days_are_inserted(pool: PgPool) {
    apply_migrations(&pool).await;

    let rec = sqlx::query!(
        "INSERT INTO tasks (title) VALUES ($1) RETURNING id",
        "Task with days"
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    let task_id = rec.id;

    for day in [0, 1, 2] {
        sqlx::query!(
            "INSERT INTO task_days (task_id, day_of_week) VALUES ($1, $2)",
            task_id,
            day
        )
        .execute(&pool)
        .await
        .unwrap();
    }

    let count: i64 =
        sqlx::query_scalar!("SELECT COUNT(*) FROM task_days WHERE task_id = $1", task_id)
            .fetch_one(&pool)
            .await
            .unwrap()
            .expect("COUNT should not be NULL");

    assert_eq!(count, 3);
}

#[sqlx::test]
async fn soft_delete_hides_task(pool: PgPool) {
    apply_migrations(&pool).await;

    let rec = sqlx::query!(
        "INSERT INTO tasks (title) VALUES ($1) RETURNING id",
        "Soft delete task"
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    sqlx::query!("UPDATE tasks SET deleted = true WHERE id = $1", rec.id)
        .execute(&pool)
        .await
        .unwrap();

    let count: i64 = sqlx::query_scalar!(
        "SELECT COUNT(*) FROM tasks WHERE id = $1 AND deleted = false",
        rec.id
    )
    .fetch_one(&pool)
    .await
    .unwrap()
    .expect("should not be null");

    assert_eq!(count, 0);
}

#[sqlx::test]
async fn today_tasks_only_matching_day(pool: PgPool) {
    apply_migrations(&pool).await;

    let today = Local::now().weekday().num_days_from_monday() as i32;

    let rec = sqlx::query!(
        "INSERT INTO tasks (title) VALUES ($1) RETURNING id",
        "Today task"
    )
    .fetch_one(&pool)
    .await
    .unwrap();

    sqlx::query!(
        "INSERT INTO task_days (task_id, day_of_week) VALUES ($1, $2)",
        rec.id,
        today
    )
    .execute(&pool)
    .await
    .unwrap();

    let rows = sqlx::query!(
        r#"
        SELECT t.id
        FROM tasks t
        JOIN task_days d ON t.id = d.task_id
        WHERE d.day_of_week = $1
          AND t.active = true
          AND t.deleted = false
        "#,
        today
    )
    .fetch_all(&pool)
    .await
    .unwrap();

    assert_eq!(rows.len(), 1);
}
