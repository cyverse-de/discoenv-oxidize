use axum::{
    extract::{Json, Path, State},
    response,
};
use serde_json::Map;
use sqlx::types::JsonValue;
use std::sync::Arc;

use crate::app_state::DiscoenvState;
use crate::db::sessions::{self, Session};
use crate::db::users;
use crate::errors::DiscoError;

use super::common;

pub async fn get_user_sessions(
    State(state): State<Arc<DiscoenvState>>,
    Path(username): Path<String>,
) -> response::Result<Json<Session>, DiscoError> {
    let conn = &state.pool;
    let cfg = &state.handler_config;
    let user = common::fix_username(&username, cfg);
    let mut tx = conn.begin().await?;

    if !users::username_exists(&mut tx, &user).await? {
        return Err(DiscoError::NotFound(format!("user {} was not found", user)));
    }

    Ok(Json(sessions::get_session(&mut tx, &user).await?))
}

pub async fn add_user_sessions(
    State(state): State<Arc<DiscoenvState>>,
    Path(username): Path<String>,
    Json(sessions): Json<Map<String, JsonValue>>,
) -> response::Result<Json<common::ID>, DiscoError> {
    let conn = &state.pool;
    let cfg = &state.handler_config;
    let user = common::fix_username(&username, cfg);
    let mut tx = conn.begin().await?;

    if !users::username_exists(&mut tx, &user).await? {
        return Err(DiscoError::NotFound(format!("user {} was not found", user)));
    }

    let sessions_str =
        serde_json::to_string(&sessions).map_err(|e| DiscoError::BadRequest(e.to_string()))?;

    let id = sessions::add_session(&mut tx, &username, &sessions_str).await?;

    tx.commit().await?;

    let p = common::ID { id };

    Ok(Json(p))
}

pub async fn update_user_sessions(
    State(state): State<Arc<DiscoenvState>>,
    Path(username): Path<String>,
    Json(sessions): Json<Map<String, JsonValue>>,
) -> response::Result<Json<Session>, DiscoError> {
    let conn = &state.pool;
    let cfg = &state.handler_config;
    let user = common::fix_username(&username, cfg);
    let mut tx = conn.begin().await?;

    if !users::username_exists(&mut tx, &user).await? {
        return Err(DiscoError::NotFound(format!("user {} was not found", user)));
    }

    let sessions_str =
        serde_json::to_string(&sessions).map_err(|e| DiscoError::BadRequest(e.to_string()))?;

    sessions::update_session(&mut tx, &user, &sessions_str).await?;

    let retval = sessions::get_session(&mut tx, &user).await?;

    tx.commit().await?;

    Ok(Json(retval))
}

pub async fn delete_user_sessions(
    State(state): State<Arc<DiscoenvState>>,
    Path(username): Path<String>,
) -> response::Result<(), DiscoError> {
    let conn = &state.pool;
    let cfg = &state.handler_config;
    let user = common::fix_username(&username, cfg);
    let mut tx = conn.begin().await?;

    if !users::username_exists(&mut tx, &user).await? {
        return Err(DiscoError::NotFound(format!("user {} was not found", user)));
    }

    sessions::delete_session(&mut tx, &user).await?;

    tx.commit().await?;

    Ok(())
}
