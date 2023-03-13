use anyhow::Context;
use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    response,
    routing::get,
    Router,
};
use axum_tracing_opentelemetry::{opentelemetry_tracing_layer, response_with_trace_layer};
use clap::Parser;
use db::bags;
use db::bags::{list_user_bags, Bags};
use serde::{Deserialize, Serialize};
use serde_json::Map;
use service_errors::DiscoError;
use service_signals::shutdown_signal;
use sqlx::{
    postgres::PgPool,
    types::{JsonValue, Uuid},
};

#[derive(Parser)]
#[command(author, version, about, long_about=None)]
struct Cli {
    #[arg(short, long)]
    /// The connection string for the database in the format postgres:://user:password@host:port/database
    database_url: String,
}

#[derive(Serialize, Deserialize)]
struct ID {
    id: Uuid,
}

#[derive(Debug, Serialize)]
struct OtelReport {
    trace_id: String,
}

async fn report_otel() -> response::Result<Json<OtelReport>, DiscoError> {
    let trace_id = axum_tracing_opentelemetry::find_current_trace_id()
        .context("failed to get trace id")
        .map_err(|a| DiscoError::Internal(a.to_string()))?;

    Ok(Json(OtelReport { trace_id: trace_id }))
}

async fn get_user_bags(
    State(conn): State<PgPool>,   // Extracts the pool from the state.
    Path(username): Path<String>, // Pulls the username out out of the Path and turns it into a String.
) -> response::Result<Json<Bags>, DiscoError> {
    Ok(Json(list_user_bags(&conn, &username).await?))
}

async fn add_user_bag(
    State(conn): State<PgPool>,
    Path(username): Path<String>,
    Json(bag): Json<Map<String, JsonValue>>,
) -> response::Result<Json<ID>, DiscoError> {
    let u = bags::add_user_bag(&conn, &username, bag).await?;
    let b = ID { id: u };
    Ok(Json(b))
}

async fn delete_user_bags(
    State(conn): State<PgPool>,
    Path(username): Path<String>,
) -> response::Result<(), DiscoError> {
    bags::delete_user_bag(&conn, &username).await?;
    Ok(())
}

async fn user_has_bags(
    State(conn): State<PgPool>,
    Path(username): Path<String>,
) -> response::Result<StatusCode, DiscoError> {
    let mut status_code = StatusCode::OK;
    let has_bag = bags::user_has_bags(&conn, &username).await?;
    if !has_bag {
        status_code = StatusCode::NOT_FOUND
    }
    status_code
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let pool = match PgPool::connect(&cli.database_url).await {
        Ok(pool) => pool,
        Err(e) => {
            println!("error connecting to the database: {}", e);
            return;
        }
    };

    match axum_tracing_opentelemetry::tracing_subscriber_ext::init_subscribers() {
        Ok(_) => {}
        Err(e) => {
            println!("error setting up opentelemetry: {}", e);
            return;
        }
    };

    let bag_routes = Router::new()
        .route("/", get(|| async {}))
        .route(
            "/:username",
            get(get_user_bags)
                .head(user_has_bags)
                .put(add_user_bag)
                .delete(delete_user_bags),
        )
        .route(
            "/:username/default",
            get(|| async {}).post(|| async {}).delete(|| async {}),
        )
        .route(
            "/:username/:bag_id",
            get(|| async {}).post(|| async {}).delete(|| async {}),
        );

    let app = Router::new()
        .nest("/bags", bag_routes)
        .route("/otel", get(report_otel))
        .layer(response_with_trace_layer())
        .layer(opentelemetry_tracing_layer())
        .with_state(pool);

    let addr = match "0.0.0.0:60000".parse() {
        Ok(v) => v,
        Err(e) => {
            println!("error parsing address: {:?}", e);
            return;
        }
    };

    match axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
    {
        Ok(_) => {}
        Err(e) => {
            println!("{:?}", e);
            return;
        }
    };
}
