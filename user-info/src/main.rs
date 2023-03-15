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
use db::{bags::{self, Bag}, users};
use db::bags::{list_user_bags, Bags};
use serde::{Deserialize, Serialize};
use serde_json::Map;
use service_errors::DiscoError;
use service_signals::shutdown_signal;
use sqlx::{
    postgres::PgPool,
    types::{JsonValue, Uuid},
};
use utoipa::{OpenApi, ToSchema};
use utoipa_swagger_ui::SwaggerUi;

#[derive(Parser)]
#[command(author, version, about, long_about=None)]
struct Cli {
    #[arg(short, long)]
    /// The connection string for the database in the format postgres:://user:password@host:port/database
    database_url: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ID {
    pub id: Uuid,
}

#[derive(Debug, Serialize, ToSchema)]
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

/// Get all of a user's bags.
/// 
/// Returns a complete listing of all of the user's bags. Mostly useful for administrative purposes.
#[utoipa::path(
    get,
    path = "/bags/{username}",
    params(
        ("username" = String, Path, description = "A username"),
    ),
    responses(
        (status = 200, description = "Lists all of a user's bags", body = Bags),
        (status = 400, description = "Bad request.", body = DiscoError),
        (status = 404, description = "User didn't exist.", body = DiscoError),
        (status = 500, description = "Internal error.", body = DiscoError)
    ),
    tag = "bag"
)]
async fn get_user_bags(
    State(conn): State<PgPool>,   // Extracts the pool from the state.
    Path(username): Path<String>, // Pulls the username out out of the Path and turns it into a String.
) -> response::Result<Json<Bags>, DiscoError> {
    let mut tx = conn.begin().await?;

    if !users::username_exists(&mut tx, &username).await? {
        return Err(DiscoError::NotFound(format!("user {} was not found", username)));
    }

    Ok(Json(list_user_bags(&mut tx, &username).await?))
}

/// Deletes all of a user's bags.
///
/// You probably don't want to actually call this.
#[utoipa::path(
    delete,
    path = "/bags/{username}",
    params(
        ("username" = String, Path, description = "A username"),
    ),
    responses(
        (status = 200, description="Deleted all of the user's bags."),
        (status = 400, description = "Bad request.", body = DiscoError, example = json!(DiscoError::BadRequest("bad request".to_owned()).create_service_error()) ),
        (status = 404, description = "User didn't exist.", body = DiscoError, example = json!(DiscoError::NotFound("user doesn't exist".to_owned()).create_service_error())),
        (status = 500, description = "Internal error.", body = DiscoError, example = json!(DiscoError::Internal("internal error".to_owned()).create_service_error())),
    ),
    tag = "bag"
)]
async fn delete_user_bags(
    State(conn): State<PgPool>,
    Path(username): Path<String>,
) -> response::Result<(), DiscoError> {
    let mut tx = conn.begin().await?;

    if !users::username_exists(&mut tx, &username).await? {
        return Err(DiscoError::NotFound(format!("user {} was not found", username)));
    }

    bags::delete_user_bags(&mut tx, &username).await?;

    Ok(())
}

/// Add a bag for a user.
/// 
/// Adds a new bag for a user. It is not set to the default.
#[utoipa::path(
    put,
    path = "/bags/{username}",
    params(
        ("username" = String, Path, description = "A username"),
    ),
    responses(
        (status = 200, description = "Adds a bag for a user", body = ID),
        (status = 400, description = "Bad request.", 
            body = DiscoError, 
            example = json!(DiscoError::BadRequest("bad request".to_owned()).create_service_error()) ),
        (status = 404, description = "User didn't exist.", 
            body = DiscoError, 
            example = json!(DiscoError::NotFound("user doesn't exist".to_owned()).create_service_error())),
        (status = 500, description = "Internal error.", 
            body = DiscoError, 
            example = json!(DiscoError::Internal("internal error".to_owned()).create_service_error()))
    ),
    tag = "bag"
)]
async fn add_user_bag(
    State(conn): State<PgPool>,
    Path(username): Path<String>,
    Json(bag): Json<Map<String, JsonValue>>,
) -> response::Result<Json<ID>, DiscoError> {
    let mut tx = conn.begin().await?;

    if !users::username_exists(&mut tx, &username).await? {
        return Err(DiscoError::NotFound(format!("user {} was not found", username)));
    }

    let u = bags::add_user_bag(&mut tx, &username, bag).await?;

    let b = ID { id: u };
    
    Ok(Json(b))
}

/// Returns whether the user has bags.
/// 
/// Check the status code to tell whether the user has any bags defined.
#[utoipa::path(
    head,
    path = "/bags/{username}",
    params(
        ("username" = String, Path, description = "A username"),
    ),
    responses(
        (status = 200, description = "The user had one or more bags."),
        (status = 404, description = "The user had no bags."),
        (status = 400, description = "Bad request.", 
            body = DiscoError, 
            example = json!(DiscoError::BadRequest("bad request".to_owned()).create_service_error()) ),
        (status = 500, description = "Internal error.", 
            body = DiscoError, 
            example = json!(DiscoError::Internal("internal error".to_owned()).create_service_error()))
    ),
    tag = "bag"
)]
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

/// Get a particular bag owned by a user.
/// 
/// Returns a bag owned by a user. Does not have to be the default bag.
#[utoipa::path(
    get,
    path = "/bags/{username}/{bag_id}",
    params(
        ("username" = String, Path, description = "A username"),
        ("bag_id" = String, Path, description = "A bag's UUID"),
    ),
    responses(
        (status = 200, description = "The user's bag.", body = Bag),
        (status = 404, description = "The user or bag was not found.", 
            body = DiscoError, 
            example = json!(DiscoError::NotFound("user doesn't exist".to_owned()).create_service_error())),
        (status = 500, description = "Internal error.", 
            body = DiscoError, 
            example = json!(DiscoError::Internal("internal error".to_owned()).create_service_error())),
    ),
    tag = "bag"
)]
async fn get_bag(
    State(conn): State<PgPool>,
    Path(username): Path<String>,
    Path(bag_id): Path<Uuid>,
) -> response::Result<Json<Bag>, DiscoError> {
    let mut tx = conn.begin().await?;

    if !users::username_exists(&mut tx, &username).await? {
        return Err(DiscoError::NotFound(format!("user {} was not found", username)));
    }

    if !bags::bag_exists(&mut tx, &username, &bag_id).await? {
        return Err(DiscoError::NotFound(format!("bag {} was not found", bag_id)));
    }

    Ok(Json(bags::get_bag(&mut tx, &username, &bag_id).await?))
}

/// Updates a particular bag for a user.
#[utoipa::path(
    post,
    path = "/bags/{username}/{bag_id}",
    params(
        ("username" = String, Path, description = "A username"),
        ("bag_id" = String, Path, description = "A bag's UUID"),
    ),
    request_body = Bag,
    responses(
        (status = 200, description = "The user's default bag.", body = Bag),
        (status = 404, description = "The user was not found.", body = DiscoError, example = json!(DiscoError::NotFound("user doesn't exist".to_owned()).create_service_error())),
        (status = 500, description = "Internal error.", body = DiscoError, example = json!(DiscoError::Internal("internal error".to_owned()).create_service_error())),
    ),
    tag = "bag"
)]
async fn update_bag(
    State(conn): State<PgPool>,
    Path(username): Path<String>,
    Path(bag_id): Path<Uuid>,
    Json(bag): Json<Map<String, JsonValue>>,
) -> response::Result<Json<Bag>, DiscoError> {
    let mut tx = conn.begin().await?;

    if !users::username_exists(&mut tx, &username).await? {
        return Err(DiscoError::NotFound(format!("user {} was not found", username)));
    }

    if !bags::bag_exists(&mut tx, &username, &bag_id).await? {
        return Err(DiscoError::NotFound(format!("bag {} was not found", bag_id)));
    }

    bags::update_bag(&mut tx, &username, &bag_id, bag).await?;
    
    Ok(Json(bags::get_bag(&mut tx, &username, &bag_id).await?))
}

/// Deletes a particular bag for a user.
#[utoipa::path(
    delete,
    path = "/bags/{username}/{bag_id}",
    params(
        ("username" = String, Path, description = "A username"),
        ("bag_id" = String, Path, description = "A bag's UUID"),
    ),
    responses(
        (status = 200, description = "The user's bag was deleted."),
        (status = 404, description = "The user was not found.", body = DiscoError, example = json!(DiscoError::NotFound("user doesn't exist".to_owned()).create_service_error())),
        (status = 500, description = "Internal error.", body = DiscoError, example = json!(DiscoError::Internal("internal error".to_owned()).create_service_error())),
    ),
    tag = "bag"
)]
async fn delete_bag(
    State(conn): State<PgPool>,
    Path(username): Path<String>,
    Path(bag_id): Path<Uuid>,
) -> response::Result<(), DiscoError> {
    let mut tx = conn.begin().await?;

    if !users::username_exists(&mut tx, &username).await? {
        return Err(DiscoError::NotFound(format!("user {} was not found", username)));
    }

    bags::delete_bag(&mut tx, &username, &bag_id).await?;
    Ok(())
}

/// Returns a user's default bag.
///
/// Creates the default bag first if it doesn't exist.
#[utoipa::path(
    get,
    path = "/bags/{username}/default",
    params(
        ("username" = String, Path, description = "A username"),
    ),
    responses(
        (status = 200, description = "The user's default bag.", body = Bag),
        (status = 404, description = "The user was not found.", body = DiscoError, example = json!(DiscoError::NotFound("user doesn't exist".to_owned()).create_service_error())),
        (status = 500, description = "Internal error,", body = DiscoError, example = json!(DiscoError::Internal("internal error".to_owned()).create_service_error())),
    ),
    tag = "bag"
)]
async fn get_default_bag(
    State(conn): State<PgPool>,
    Path(username): Path<String>,
) -> response::Result<Json<Bag>, DiscoError> {
    let mut tx = conn.begin().await?;

    if !users::username_exists(&mut tx, &username).await? {
        return Err(DiscoError::NotFound(format!("user {} was not found", username)));
    }

    if !bags::has_default_bag(&mut tx, &username).await? {
        let new_bag: Map<String, JsonValue> = Map::new();
        let new_bag_uuid = bags::add_user_bag(&conn, &username, new_bag).await?;
        bags::set_default_bag(&mut tx, &username, &new_bag_uuid).await?;
    }

    Ok(Json(bags::get_default_bag(&mut tx, &username).await?))
}

/// Updates the default bag owned by a user.
///
/// This will create the default bag if it doesn't exist,.
#[utoipa::path(
    post,
    path = "/bags/{username}/default",
    params(
        ("username" = String, Path, description = "A username"),
    ),
    request_body = Bag,
    responses(
        (status = 200, description = "The user's default bag.", body = Bag),
        (status = 404, description = "The user was not found.", body = DiscoError, example = json!(DiscoError::NotFound("user doesn't exist".to_owned()).create_service_error())),
        (status = 500, description = "Internal error.", body = DiscoError, example = json!(DiscoError::Internal("internal error".to_owned()).create_service_error())),
    ),
    tag = "bag"
)]
async fn update_default_bag(
    State(conn): State<PgPool>,
    Path(username): Path<String>,
    Json(bag): Json<Map<String, JsonValue>>,
) -> response::Result<Json<Bag>, DiscoError> {
    let mut tx = conn.begin().await?;

    if !users::username_exists(&mut tx, &username).await? {
        return Err(DiscoError::NotFound(format!("user {} was not found", username)));
    }

    if !bags::has_default_bag(&mut tx, &username).await? {
        let new_bag_uuid = bags::add_user_bag(&mut tx, &username, bag).await?;
        bags::set_default_bag(&mut tx, &username, &new_bag_uuid).await?;
    } else {
        bags::update_default_bag(&mut tx, &username, bag).await?;
    }

    Ok(Json(bags::get_default_bag(&mut tx, &username).await?))
}

/// Deletes a user's default bag.
#[utoipa::path(
    delete,
    path = "/bags/{username}/default",
    params(
        ("username" = String, Path, description = "A username"),
    ),
    responses(
        (status = 200, description = "The user's default bag was deleted."),
        (status = 404, description = "The user was not found.", body = DiscoError, example = json!(DiscoError::NotFound("user doesn't exist".to_owned()).create_service_error())),
        (status = 500, description = "Internal error.", body = DiscoError, example = json!(DiscoError::Internal("internal error".to_owned()).create_service_error())),
    ),
    tag = "bag"
)]
async fn delete_default_bag(
    State(conn): State<PgPool>,
    Path(username): Path<String>,
) -> response::Result<(), DiscoError> {
    let mut tx = conn.begin().await?;

    if !users::username_exists(&mut tx, &username).await? {
        return Err(DiscoError::NotFound(format!("user {} was not found", username)));
    }

    bags::delete_default_bag(&mut tx, &username).await?;
    Ok(())
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
            delete_user_bags,
            add_user_bag,
            user_has_bags,
            get_bag,
            update_bag,
            delete_bag,
            get_default_bag,
            update_default_bag,
            delete_default_bag,
        ),
        components(
            schemas(
                ID,
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
