use axum::{
    response::IntoResponse,
    routing::get,
    Json,
    Router
};
use std::sync::Arc;
use dotenv::dotenv;
use sqlx::{
    postgres::PgPoolOptions,
    Pool,
    Postgres
};

// Struct containing DB Pool
pub struct AppState {
    db: Pool<Postgres>,
}

// main()
#[tokio::main]
async fn main() {
    dotenv().ok(); // Load the .env

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let pool = match PgPoolOptions::new()
        .max_connections(10)
        .connect(&database_url)
        .await
    {
        Ok(pool) => {
            println!("✅Connection to the database is successful!");
            pool
        }
        Err(err) => {
            println!("❌ Failed to connect to the database: {:?}", err);
            std::process::exit(1);
        }
    };

    let app_state = Arc::new(AppState {     // Allows multiple parts of the application 
        db: pool.clone()                                   // shared ownership of the application state
    });
    let app = Router::new()
        .route("/api/healthchecker", get(health_checker_handler))
        .with_state(app_state);

    println!("🚀 Server started successfully at 127.0.0.1:8000");
    axum::Server::bind(&"127.0.0.1:8000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

// Handler for basic endpoint: /api/healthchecker
async fn health_checker_handler() -> impl IntoResponse {
    const MESSAGE: &str = "Simple CRUD API with Rust, SQLx, Postgres and Axum";

    let json_response = serde_json::json!({
        "status": "success",
        "message": MESSAGE
    });

    Json(json_response)
}
