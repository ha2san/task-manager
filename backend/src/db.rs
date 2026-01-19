use sqlx::PgPool;
use sqlx::postgres::PgPoolOptions; // Recommandé pour configurer le pool
use std::env;

// Renommage de pool -> init_pool pour correspondre au main.rs
pub async fn init_pool() -> PgPool {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to database")
}

// Ajout de la fonction manquante
pub async fn run_migrations(pool: &PgPool) {
    sqlx::migrate!("./migrations")
        .run(pool)
        .await
        .expect("Failed to run migrations");

    println!("Migrations executed successfully");
}
