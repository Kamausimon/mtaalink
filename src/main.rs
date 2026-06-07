use axum::http::header;
use axum::{Extension, Router, routing::get};
use std::sync::Arc;
use axum_server::bind;
use dotenvy::dotenv;
use sqlx::postgres::PgPoolOptions;
use std::env;
use std::net::SocketAddr;
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;
use tower_http::trace::TraceLayer;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod errors;
mod extractors;
mod routes;
mod utils;

pub use errors::{AppError, AppResult};

use routes::admin::admin_routes;
use routes::analytics::analytics_routes;
use routes::auth::auth_routes;
use routes::availability::availability_routes;
use routes::bookings::booking_routes;
use routes::businesses::businesses_routes;
use routes::categories::category_routes;
use routes::clients::client_routes;
use routes::dashboard::dashboard_routes;
use routes::favorites::favorites_routes;
use routes::locations::locations_routes;
use routes::messages::messages_routes;
use routes::posts::posts_routes;
use routes::reviews::reviews_routes;
use routes::service_providers::service_providers_routes;
use utils::attachments::attachments_routes;
use routes::notifications::notification_routes;
use routes::packages::package_routes;
use routes::payments::payment_routes;
use routes::search::search_routes;
use routes::services::services_routes;
use routes::wallet::wallet_routes;
use routes::ws::ws_routes;
use utils::ws_state::{WsConnections, new_ws_connections};

#[tokio::main]
async fn main() {
    dotenv().ok();
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer())
        .init();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    println!("Using database URL: {}", database_url);

    // FRONTEND_URL accepts comma-separated origins, e.g.:
    // FRONTEND_URL=https://mtaalink.vercel.app,http://localhost:3000
    let allowed_origins: Vec<axum::http::HeaderValue> =
        env::var("FRONTEND_URL")
            .unwrap_or_else(|_| "http://localhost:3000".to_string())
            .split(',')
            .map(|s| {
                s.trim()
                    .parse::<axum::http::HeaderValue>()
                    .expect("Invalid value in FRONTEND_URL")
            })
            .collect();

    let cors_layer = CorsLayer::new()
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::DELETE,
        ])
        .allow_headers([
            axum::http::header::AUTHORIZATION,
            axum::http::header::CONTENT_TYPE,
            header::CONTENT_LENGTH,
            header::ACCEPT,
        ])
        .allow_origin(allowed_origins)
        .allow_credentials(true);

    // Create a connection pool
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create pool");
    println!("Database connection pool created successfully");

    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run database migrations");
    println!("Database migrations applied successfully");

    let storage = Arc::new(utils::storage::AppStorage::init());
    let ws_connections: WsConnections = new_ws_connections();

    utils::reminders::start_reminder_task(pool.clone());

    let app = Router::new()
        .nest("/auth", auth_routes(pool.clone())) // Mount the auth routes
        .nest("/dashboard", dashboard_routes(pool.clone()))
        .nest("/service_providers", service_providers_routes(pool.clone())) // Mount the service providers routes
        .nest("/businesses", businesses_routes(pool.clone())) // Mount the businesses routes
        .nest("/clients", client_routes(pool.clone())) // Mount the clients routes
        .nest("/reviews", reviews_routes(pool.clone())) // Mount the reviews routes
        .nest("/favorites", favorites_routes(pool.clone())) // Mount the favorites routes
        .nest("/messages", messages_routes(pool.clone())) // Mount the messages routes
        .nest("/categories", category_routes(pool.clone())) // Mount the categories routes
        .nest("/bookings", booking_routes(pool.clone())) // Mount the bookings routes
        .nest("/admin", admin_routes(pool.clone())) // Mount the admin routes
        .nest("/locations", locations_routes(pool.clone())) // Mount the locations routes
        .nest("/posts", posts_routes(pool.clone())) // Mount the posts routes
        .nest("/attachments", attachments_routes(pool.clone())) // Mount the attachments routes
        .nest("/services", services_routes(pool.clone()))
        .nest("/payments", payment_routes(pool.clone()))
        .nest("/notifications", notification_routes(pool.clone()))
        .nest("/packages", package_routes(pool.clone()))
        .nest("/search", search_routes(pool.clone()))
        .nest("/analytics", analytics_routes(pool.clone()))
        .nest("/availability", availability_routes(pool.clone()))
        .nest("/wallet", wallet_routes(pool.clone()))
        .nest("/ws", ws_routes())
        .nest_service("/uploads", ServeDir::new("uploads")) // Serve static files from the uploads directory
        .layer(Extension(ws_connections))
        .layer(Extension(storage))
        .layer(cors_layer)
        .layer(TraceLayer::new_for_http())
        .route("/", get(root));

    let port = env::var("PORT").unwrap_or_else(|_| "7878".to_string());
    let addr = SocketAddr::from(([127, 0, 0, 1], port.parse::<u16>().unwrap()));
    println!("listening on http://{}", addr);

    bind(addr).serve(app.into_make_service()).await.unwrap();
}

async fn root() -> &'static str {
    "mtaalink is running!"
}
