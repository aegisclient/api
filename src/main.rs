mod store;
mod routes;
mod models;

use axum::{Router, extract::Request, middleware::{self, Next}, response::Response, http::StatusCode};
use tower_http::cors::{CorsLayer, Any};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use std::sync::Arc;
use tokio::sync::RwLock;

use store::Store;

pub type AppState = Arc<RwLock<Store>>;

/// Reads AEGIS_ADMIN_TOKEN from env. All /admin/* routes require
/// `Authorization: Bearer <token>` header matching this value.
fn get_admin_token() -> String {
    std::env::var("AEGIS_ADMIN_TOKEN").unwrap_or_else(|_| {
        tracing::error!("AEGIS_ADMIN_TOKEN not set! Admin endpoints will reject all requests.");
        tracing::error!("Set it: export AEGIS_ADMIN_TOKEN=your-secret-token");
        String::new()
    })
}

async fn admin_auth(req: Request, next: Next) -> Result<Response, StatusCode> {
    let token = get_admin_token();
    if token.is_empty() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let auth_header = req
        .headers()
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    let expected = format!("Bearer {}", token);
    if auth_header != expected {
        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(req).await)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(tracing_subscriber::EnvFilter::new(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "aegis_api=info,tower_http=info".into()),
        ))
        .with(tracing_subscriber::fmt::layer())
        .init();

    let admin_token = std::env::var("AEGIS_ADMIN_TOKEN").ok();
    if admin_token.is_none() {
        tracing::warn!("===========================================");
        tracing::warn!("  AEGIS_ADMIN_TOKEN is not set!");
        tracing::warn!("  Admin endpoints will reject all requests.");
        tracing::warn!("  Set it: export AEGIS_ADMIN_TOKEN=mysecret");
        tracing::warn!("===========================================");
    }

    let store = Store::load_or_create();
    let state: AppState = Arc::new(RwLock::new(store));

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Public routes — anyone can call (the launcher calls these)
    let public_routes = Router::new()
        .route("/api/validate-key", axum::routing::post(routes::validate_key))
        .route("/api/check-update", axum::routing::get(routes::check_update))
        .route("/api/config/sync", axum::routing::post(routes::sync_config))
        .route("/api/config/{username}", axum::routing::get(routes::get_config))
        .route("/api/cape/{username}", axum::routing::get(routes::get_cape));

    // Admin routes — require AEGIS_ADMIN_TOKEN in Authorization header
    let admin_routes = Router::new()
        .route("/admin/keys", axum::routing::get(routes::admin_list_keys))
        .route("/admin/keys", axum::routing::post(routes::admin_create_key))
        .route("/admin/keys/{key_id}", axum::routing::delete(routes::admin_revoke_key))
        .route("/admin/stats", axum::routing::get(routes::admin_stats))
        .layer(middleware::from_fn(admin_auth));

    let app = Router::new()
        .merge(public_routes)
        .merge(admin_routes)
        .layer(cors)
        .with_state(state);

    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".into());
    let addr = format!("0.0.0.0:{}", port);

    tracing::info!("Aegis API starting on {}", addr);
    tracing::info!("Data stored in: ./aegis-data.json");

    let listener = tokio::net::TcpListener::bind(&addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
