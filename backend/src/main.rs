mod auth;
mod db;
mod middleware;
mod models;
mod routes;
mod routes_auth;

use crate::routes_auth::auth_routes;
use axum::{Router, middleware::from_fn};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tower_http::services::ServeDir;

#[tokio::main]
async fn main() {
    let pool = db::init_pool().await;
    db::run_migrations(&pool).await;

    let frontend_path =
        std::env::var("FRONTEND_PATH").unwrap_or_else(|_| "../frontend".to_string());

    let app = Router::new()
        .nest("/api/auth", auth_routes(pool.clone()))
        .nest(
            "/api",
            routes::routes(pool).layer(from_fn(middleware::auth)),
        )
        .fallback_service(ServeDir::new(frontend_path));

    let addr = SocketAddr::from(([0, 0, 0, 0], 3000));
    let listener = TcpListener::bind(addr).await.unwrap();

    println!("Server listening on http://{}", addr);
    axum::serve(listener, app).await.unwrap();
}
