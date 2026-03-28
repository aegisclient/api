mod store;
mod routes;
mod models;

use axum::Router;
use tower_http::cors::{CorsLayer, Any};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use std::sync::Arc;
use tokio::sync::RwLock;

use store::Store;

pub type AppState = Arc<RwLock<Store>>;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "aegis_api=info,tower_http=info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let store = Store::load_or_create();
    let state: AppState = Arc::new(RwLock::new(store));

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        // Public endpoints (used by launcher)
        .route("/api/validate-key", axum::routing::post(routes::validate_key))
        .route("/api/check-update", axum::routing::get(routes::check_update))
        .route("/api/config/sync", axum::routing::post(routes::sync_config))
        .route("/api/config/{username}", axum::routing::get(routes::get_config))
        .route("/api/cape/{username}", axum::routing::get(routes::get_cape))
        // Admin endpoints (for you to manage keys from laptop)
        .route("/admin/keys", axum::routing::get(routes::admin_list_keys))
        .route("/admin/keys", axum::routing::post(routes::admin_create_key))
        .route("/admin/keys/{key_id}", axum::routing::delete(routes::admin_revoke_key))
        .route("/admin/stats", axum::routing::get(routes::admin_stats))
        .layer(cors)
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".into());
    let addr = format!("0.0.0.0:{}", port);

    tracing::info!("Aegis API starting on {}", addr);
    tracing::info!("Admin panel: http://localhost:{}/admin/keys", port);
    tracing::info!("Data stored in: ./aegis-data.json");

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
