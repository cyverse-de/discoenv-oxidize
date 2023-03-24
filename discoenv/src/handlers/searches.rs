use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    response,
};
use serde_json::Map;
use sqlx::{postgres::PgPool, types::JsonValue};

use db::searches::{self, SavedSearches};
use db::users;
use service_errors::DiscoError;

use super::common;
use super::config;

pub async fn get_saved_searches(
    State((conn, cfg)): State<(PgPool, config::HandlerConfiguration)>,
    Path(username): Path<String>,
) -> response::Result<Json<SavedSearches>, DiscoError> {
    let user = common::fix_username(&username, &cfg);
    let mut tx = conn.begin().await?;
    if !users::username_exists(&mut tx, &user).await? {
        return Err(DiscoError::NotFound(format!("user {} was not found", user)));
    }
    let retval = searches::get_saved_searches(&mut tx, &user).await?;
    Ok(Json(retval))
}

pub async fn has_saved_searches(
    State((conn, cfg)): State<(PgPool, config::HandlerConfiguration)>,
    Path(username): Path<String>,
) -> response::Result<StatusCode, DiscoError> {
    let user = common::fix_username(&username, &cfg);
    let mut status_code = StatusCode::OK;
    let has_saved_searches = searches::has_saved_searches(&conn, &user).await?;
    if !has_saved_searches {
        status_code = StatusCode::NOT_FOUND;
    }
    Ok(status_code)
}

pub async fn add_saved_searches(
    State((conn, cfg)): State<(PgPool, config::HandlerConfiguration)>,
    Path(username): Path<String>,
    Json(saved_searches): Json<Map<String, JsonValue>>,
) -> response::Result<Json<common::ID>, DiscoError> {
    let user = common::fix_username(&username, &cfg);
    let mut tx = conn.begin().await?;
    if !users::username_exists(&mut tx, &user).await? {
        return Err(DiscoError::NotFound(format!("user {} was not found", user)));
    }
    let searches_str = serde_json::to_string(&saved_searches)
        .map_err(|e| DiscoError::BadRequest(e.to_string()))?;
    let id = searches::add_saved_searches(&mut tx, &user, &searches_str).await?;
    tx.commit().await?;
    Ok(Json(common::ID { id }))
}

pub async fn update_saved_searches(
    State((conn, cfg)): State<(PgPool, config::HandlerConfiguration)>,
    Path(username): Path<String>,
    Json(saved_searches): Json<Map<String, JsonValue>>,
) -> response::Result<Json<SavedSearches>, DiscoError> {
    let user = common::fix_username(&username, &cfg);
    let mut tx = conn.begin().await?;
    if !users::username_exists(&mut tx, &user).await? {
        return Err(DiscoError::NotFound(format!("user {} was not found", user)));
    }
    let searches_str = serde_json::to_string(&saved_searches)
        .map_err(|e| DiscoError::BadRequest(e.to_string()))?;
    searches::update_saved_searches(&mut tx, &user, &searches_str).await?;
    let retval = searches::get_saved_searches(&mut tx, &user).await?;
    tx.commit().await?;
    Ok(Json(retval))
}

pub async fn delete_saved_searches(
    State((conn, cfg)): State<(PgPool, config::HandlerConfiguration)>,
    Path(username): Path<String>,
) -> Result<(), DiscoError> {
    let user = common::fix_username(&username, &cfg);
    let mut tx = conn.begin().await?;
    if !users::username_exists(&mut tx, &user).await? {
        return Err(DiscoError::NotFound(format!("user {} was not found", user)));
    }
    searches::delete_saved_searches(&mut tx, &user).await?;
    tx.commit().await?;
    Ok(())
}
