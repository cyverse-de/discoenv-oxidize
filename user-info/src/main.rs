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
use db::bags::{self, Bag};
use db::bags::{list_user_bags, Bags};
use serde::{Deserialize, Serialize};
use serde_json::Map;
use service_errors::DiscoError;
use service_signals::shutdown_signal;
use sqlx::{
    postgres::PgPool,
    types::{JsonValue, Uuid},
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

#[derive(Parser)]
#[command(author, version, about, long_about=None)]
struct Cli {
    #[arg(short, long)]
    /// The connection string for the database in the format postgres:://user:password@host:port/database
    database_url: String,
}

#[derive(Serialize, Deserialize)]
pub struct ID {
    pub id: Uuid,
}

#[derive(Debug, Serialize)]
struct OtelReport {
    trace_id: String,
}

/// Returns an open telementry trace ID back to the caller.
///
/// Just useful to make sure that the open telemetry middleware is working.
async fn report_otel() -> response::Result<Json<OtelReport>, DiscoError> {
    let trace_id = axum_tracing_opentelemetry::find_current_trace_id()
        .context("failed to get trace id")
        .map_err(|a| DiscoError::Internal(a.to_string()))?;

    Ok(Json(OtelReport { trace_id: trace_id }))
}

/// Get all a user's bags.
#[utoipa::path(
    get,
    path = "/bags/:username",
    responses(
        (status = 200, description = "Lists all of a user's bags", body = Bags),
        (status = 404, description = "User didn't exist", body = DiscoError),
        (status = 500, description = "Internal error", body = DiscoError)
    )
)]
async fn get_user_bags(
    State(conn): State<PgPool>,   // Extracts the pool from the state.
    Path(username): Path<String>, // Pulls the username out out of the Path and turns it into a String.
) -> response::Result<Json<Bags>, DiscoError> {
    Ok(Json(list_user_bags(&conn, &username).await?))
}

/// A a bag for a user.
async fn add_user_bag(
    State(conn): State<PgPool>,
    Path(username): Path<String>,
    Json(bag): Json<Map<String, JsonValue>>,
) -> response::Result<Json<ID>, DiscoError> {
    let u = bags::add_user_bag(&conn, &username, bag).await?;
    let b = ID { id: u };
    Ok(Json(b))
}

/// Deletes all of a user's bags.
///
/// You probably don't want to actually call this.
async fn delete_user_bags(
    State(conn): State<PgPool>,
    Path(username): Path<String>,
) -> response::Result<(), DiscoError> {
    bags::delete_user_bags(&conn, &username).await?;
    Ok(())
}

/// Returns whether the user has bags.
async fn user_has_bags(
    State(conn): State<PgPool>,
    Path(username): Path<String>,
) -> response::Result<StatusCode, DiscoError> {
    let mut status_code = StatusCode::OK;
    let has_bag = bags::user_has_bags(&conn, &username).await?;
    if !has_bag {
        status_code = StatusCode::NOT_FOUND
    }
    Ok(status_code)
}

/// Returns a user's default bag.
///
/// Creates the default bag first if it doesn't exist.
async fn get_default_bag(
    State(conn): State<PgPool>,
    Path(username): Path<String>,
) -> response::Result<Json<Bag>, DiscoError> {
    if !bags::has_default_bag(&conn, &username).await? {
        let new_bag: Map<String, JsonValue> = Map::new();
        let new_bag_uuid = bags::add_user_bag(&conn, &username, new_bag).await?;
        bags::set_default_bag(&conn, &username, &new_bag_uuid).await?;
    }

    Ok(Json(bags::get_default_bag(&conn, &username).await?))
}

/// Get a particular bag owned by a user.
async fn get_bag(
    State(conn): State<PgPool>,
    Path(username): Path<String>,
    Path(bag_id): Path<Uuid>,
) -> response::Result<Json<Bag>, DiscoError> {
    Ok(Json(bags::get_bag(&conn, &username, &bag_id).await?))
}

/// Updates the default bag owned by a user.
///
/// This will create the default bag if it doesn't exist,.
async fn update_default_bag(
    State(conn): State<PgPool>,
    Path(username): Path<String>,
    Json(bag): Json<Map<String, JsonValue>>,
) -> response::Result<Json<Bag>, DiscoError> {
    let mut tx = conn.begin().await?;
    if !bags::has_default_bag(&mut tx, &username).await? {
        let new_bag_uuid = bags::add_user_bag(&mut tx, &username, bag).await?;
        bags::set_default_bag(&mut tx, &username, &new_bag_uuid).await?;
    } else {
        bags::update_default_bag(&mut tx, &username, bag).await?;
    }
    Ok(Json(bags::get_default_bag(&mut tx, &username).await?))
}

/// Deletes a user's defaul bag.
async fn delete_default_bag(
    State(conn): State<PgPool>,
    Path(username): Path<String>,
) -> response::Result<(), DiscoError> {
    bags::delete_default_bag(&conn, &username).await?;
    Ok(())
}

/// Deletes a particular bag for a user.
async fn delete_bag(
    State(conn): State<PgPool>,
    Path(username): Path<String>,
    Path(bag_id): Path<Uuid>,
) -> response::Result<(), DiscoError> {
    bags::delete_bag(&conn, &username, &bag_id).await?;
    Ok(())
}

/// Updates a particular bag for a user.
async fn update_bag(
    State(conn): State<PgPool>,
    Path(username): Path<String>,
    Path(bag_id): Path<Uuid>,
    Json(bag): Json<Map<String, JsonValue>>,
) -> response::Result<Json<Bag>, DiscoError> {
    let mut tx = conn.begin().await?;
    bags::update_bag(&mut tx, &username, &bag_id, bag).await?;
    Ok(Json(bags::get_bag(&mut tx, &username, &bag_id).await?))
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

    #[derive(OpenApi)]
    #[openapi(
        paths(
            get_user_bags,
        ),
        components(
            schemas(
                bags::Bag, 
                bags::Bags, 
                service_errors::DiscoError,
            )
        ),
        tags(
            (name = "user-info", description="User information API")
        )
    )]

    struct ApiDoc;
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
            get(get_default_bag)
                .post(update_default_bag)
                .delete(delete_default_bag),
        )
        .route(
            "/:username/:bag_id",
            get(get_bag).post(update_bag).delete(delete_bag),
        );

    let app = Router::new()
        .merge(SwaggerUi::new("/docs").url("/openapi.json", ApiDoc::openapi()))
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
