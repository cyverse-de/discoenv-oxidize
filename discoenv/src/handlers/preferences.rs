use axum::{
    extract::{Json, Path, State},
    response,
};
use serde_json::Map;
use service_errors::DiscoError;
use sqlx::{postgres::PgPool, types::JsonValue};

use db::preferences::{self, Preferences};
use db::users;

use super::common;
use super::config;

pub async fn get_user_preferences(
    State((conn, cfg)): State<(PgPool, config::HandlerConfiguration)>,
    Path(username): Path<String>,
) -> response::Result<Json<Preferences>, DiscoError> {
    let user = common::fix_username(&username, &cfg);
    let mut tx = conn.begin().await?;

    if !users::username_exists(&mut tx, &user).await? {
        return Err(DiscoError::NotFound(format!("user {} was not found", user)));
    }

    Ok(Json(preferences::user_preferences(&mut tx, &user).await?))
}

pub async fn add_user_preferences(
    State((conn, cfg)): State<(PgPool, config::HandlerConfiguration)>,
    Path(username): Path<String>,
    Json(preferences): Json<Map<String, JsonValue>>,
) -> response::Result<Json<common::ID>, DiscoError> {
    let user = common::fix_username(&username, &cfg);
    let mut tx = conn.begin().await?;

    if !users::username_exists(&mut tx, &user).await? {
        return Err(DiscoError::NotFound(format!("user {} was not found", user)));
    }

    let pref_str =
        serde_json::to_string(&preferences).map_err(|e| DiscoError::BadRequest(e.to_string()))?;

    let id = preferences::add_user_preferences(&mut tx, &username, &pref_str).await?;

    tx.commit().await?;

    let p = common::ID { id };

    Ok(Json(p))
}

pub async fn update_user_preferences(
    State((conn, cfg)): State<(PgPool, config::HandlerConfiguration)>,
    Path(username): Path<String>,
    Json(preferences): Json<Map<String, JsonValue>>,
) -> response::Result<Json<Preferences>, DiscoError> {
    let user = common::fix_username(&username, &cfg);
    let mut tx = conn.begin().await?;

    if !users::username_exists(&mut tx, &user).await? {
        return Err(DiscoError::NotFound(format!("user {} was not found", user)));
    }

    let pref_str =
        serde_json::to_string(&preferences).map_err(|e| DiscoError::BadRequest(e.to_string()))?;

    preferences::update_user_preferences(&mut tx, &user, &pref_str).await?;

    let retval = preferences::user_preferences(&mut tx, &user).await?;

    tx.commit().await?;

    Ok(Json(retval))
}

pub async fn delete_user_preferences(
    State((conn, cfg)): State<(PgPool, config::HandlerConfiguration)>,
    Path(username): Path<String>,
) -> response::Result<(), DiscoError> {
    let user = common::fix_username(&username, &cfg);
    let mut tx = conn.begin().await?;

    if !users::username_exists(&mut tx, &user).await? {
        return Err(DiscoError::NotFound(format!("user {} was not found", user)));
    }

    preferences::delete_user_preferences(&mut tx, &user).await?;

    tx.commit().await?;

    Ok(())
}
