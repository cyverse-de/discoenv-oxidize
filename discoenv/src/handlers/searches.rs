use axum::{
    extract::{Json, Extension, State},
    http::StatusCode,
    response,
};
use serde_json::Map;
use sqlx::types::JsonValue;
use std::sync::Arc;

use crate::db::searches::{self, SavedSearches};
use crate::db::users;
use crate::errors::DiscoError;
use crate::app_state::DiscoenvState;
use crate::auth::UserInfo;

use super::common;

/// Get the saved searches for a user.
///
/// Returns the JSON document containing the saved searches for a user.
#[utoipa::path(
    get,
    path = "/searches",
    responses(
        (status = 200, description = "Returned the user's saved searches", body = SavedSearches),
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
    tag = "searches"
)]
pub async fn get_saved_searches(
    State(state): State<Arc<DiscoenvState>>,
    Extension(user_info): Extension<UserInfo>,
) -> response::Result<Json<SavedSearches>, DiscoError> {
    let user = common::fix_username(&user_info.preferred_username.unwrap_or_default(), &state.handler_config);
    let mut tx = state.pool.begin().await?;
    if !users::username_exists(&mut tx, &user).await? {
        return Err(DiscoError::NotFound(format!("user {} was not found", user)));
    }
    let retval = searches::get_saved_searches(&mut tx, &user).await?;
    Ok(Json(retval))
}

/// Whether the user has saved searches.
///
/// Returns a 200 status if the user has saved searches.
#[utoipa::path(
    head,
    path = "/searches",
    responses(
        (status = 200, description = "The user has saved searches"),
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
    tag = "searches"
)]
pub async fn has_saved_searches(
    State(state): State<Arc<DiscoenvState>>,
    Extension(user_info): Extension<UserInfo>,
) -> response::Result<StatusCode, DiscoError> {
    let user = common::fix_username(&user_info.preferred_username.unwrap_or_default(), &state.handler_config);
    let mut status_code = StatusCode::OK;
    let has_saved_searches = searches::has_saved_searches(&state.pool, &user).await?;
    if !has_saved_searches {
        status_code = StatusCode::NOT_FOUND;
    }
    Ok(status_code)
}

/// Adds a saved searches document for a user.
///
/// Adds a new saved searches document for a user. Only really useful when setting up a new user.
#[utoipa::path(
    put,
    path = "/searches",
    request_body = JsonValue::Object<Searches>,
    responses(
        (status = 200, description = "The saved searches document was added", body = common::ID),
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
    tag = "searches"
)]
pub async fn add_saved_searches(
    State(state): State<Arc<DiscoenvState>>,
    Extension(user_info): Extension<UserInfo>,
    Json(saved_searches): Json<Map<String, JsonValue>>,
) -> response::Result<Json<common::ID>, DiscoError> {
    let user = common::fix_username(&user_info.preferred_username.unwrap_or_default(), &state.handler_config);
    let mut tx = state.pool.begin().await?;
    if !users::username_exists(&mut tx, &user).await? {
        return Err(DiscoError::NotFound(format!("user {} was not found", user)));
    }
    let searches_str = serde_json::to_string(&saved_searches)
        .map_err(|e| DiscoError::BadRequest(e.to_string()))?;
    let id = searches::add_saved_searches(&mut tx, &user, &searches_str).await?;
    tx.commit().await?;
    Ok(Json(common::ID { id }))
}

/// Updates the saved searches document stored for a user.
///
/// Returns the updated searches document.
#[utoipa::path(
    post,
    path = "/searches",
    request_body = JsonValue::Object<Searches>,
    responses(
        (status = 200, description = "The saved searches document was updated", body = JsonValue::Object<Searches>),
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
    tag = "searches"
)]
pub async fn update_saved_searches(
    State(state): State<Arc<DiscoenvState>>,
    Extension(user_info): Extension<UserInfo>,
    Json(saved_searches): Json<Map<String, JsonValue>>,
) -> response::Result<Json<SavedSearches>, DiscoError> {
    let user = common::fix_username(&user_info.preferred_username.unwrap_or_default(), &state.handler_config);
    let mut tx = state.pool.begin().await?;
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

/// Deletes the saved searches document for a user.
///
/// Returns a 200 status code on success.
#[utoipa::path(
    delete,
    path = "/searches",
    responses(
        (status = 200, description = "The saved searches document was deleted"),
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
    tag = "searches"
)]
pub async fn delete_saved_searches(
    State(state): State<Arc<DiscoenvState>>,
    Extension(user_info): Extension<UserInfo>,
) -> Result<(), DiscoError> {
    let user = common::fix_username(&user_info.preferred_username.unwrap_or_default(), &state.handler_config);
    let mut tx = state.pool.begin().await?;
    if !users::username_exists(&mut tx, &user).await? {
        return Err(DiscoError::NotFound(format!("user {} was not found", user)));
    }
    searches::delete_saved_searches(&mut tx, &user).await?;
    tx.commit().await?;
    Ok(())
}
