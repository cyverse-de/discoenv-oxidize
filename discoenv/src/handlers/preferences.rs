use axum::{
    extract::{Json, Extension, State},
    response,
};
use serde_json::Map;
use sqlx::types::JsonValue;
use std::sync::Arc;

use crate::{db::preferences::{self, Preferences}, app_state::DiscoenvState};
use crate::db::users;
use crate::errors::DiscoError;
use crate::auth::UserInfo;

use super::common;

/// Get the user's preferences.
///
/// Returns the preferences as a JSON document.
#[utoipa::path(
    get,
    path = "/preferences",
    responses(
        (status = 200, description = "Body contains the user's preferences", body = Preferences),
        (status = 400, description = "Bad request", body = DiscoError,
            example = json!(DiscoError::BadRequest("bad request".to_owned()).create_service_error())),
        (status = 404, description = "User didn't exist.", 
            body = DiscoError,
            example = json!(DiscoError::NotFound("user wasn't found".to_owned()).create_service_error())),
        (status = 500, description = "Internal error.", 
            body = DiscoError,
            example = json!(DiscoError::Internal("internal error".to_owned()).create_service_error())),
    ),
    tag = "preferences"
)]
pub async fn get_user_preferences(
    State(state): State<Arc<DiscoenvState>>,
    Extension(user_info): Extension<UserInfo>,
) -> response::Result<Json<Preferences>, DiscoError> {
    let user = common::fix_username(&user_info.preferred_username.unwrap_or_default(), &state.handler_config);
    let mut tx = state.pool.begin().await?;
    if !users::username_exists(&mut tx, &user).await? {
        return Err(DiscoError::NotFound(format!("user {} was not found", user)));
    }
    Ok(Json(preferences::user_preferences(&mut tx, &user).await?))
}


/// Adds a new set of user preferences.
///
/// Returns the UUID of the new record containing the preferences. This call is mostly
/// just useful for setting up new users.
#[utoipa::path(
    put,
    path = "/preferences",
    request_body = JsonValue::Object<Preferences>,
    responses(
        (status = 200, description = "Adds a new set of user preferences", body = common::ID),
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
    tag = "preferences"

)]
pub async fn add_user_preferences(
    State(state): State<Arc<DiscoenvState>>,
    Extension(user_info): Extension<UserInfo>,
    Json(preferences): Json<Map<String, JsonValue>>,
) -> response::Result<Json<common::ID>, DiscoError> {
    let user = common::fix_username(&user_info.preferred_username.unwrap_or_default(), &state.handler_config);
    let mut tx = state.pool.begin().await?;

    if !users::username_exists(&mut tx, &user).await? {
        return Err(DiscoError::NotFound(format!("user {} was not found", user)));
    }

    let pref_str =
        serde_json::to_string(&preferences).map_err(|e| DiscoError::BadRequest(e.to_string()))?;

    let id = preferences::add_user_preferences(&mut tx, &user, &pref_str).await?;

    tx.commit().await?;

    let p = common::ID { id };

    Ok(Json(p))
}

/// Updates the user's preferences
///
/// Returns the updated preferences for the user.
#[utoipa::path(
    post,
    path = "/preferences",
    request_body = JsonValue::Object<Preferences>,
    responses(
        (status = 200, description = "Returned the updated user preferences", body = Preferences),
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
    tag = "preferences"
)]
pub async fn update_user_preferences(
    State(state): State<Arc<DiscoenvState>>,
    Extension(user_info): Extension<UserInfo>,
    Json(preferences): Json<Map<String, JsonValue>>,
) -> response::Result<Json<Preferences>, DiscoError> {
    let user = common::fix_username(&user_info.preferred_username.unwrap_or_default(), &state.handler_config);
    let mut tx = state.pool.begin().await?;

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

/// Deletes a user's preferences.
///
/// Returns a 200 status code on success.
#[utoipa::path(
    delete,
    path = "/preferences",
    responses(
        (status = 200, description = "The preferences were successfully deleted"),
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
    tag = "preferences"

)]
pub async fn delete_user_preferences(
    State(state): State<Arc<DiscoenvState>>,
    Extension(user_info): Extension<UserInfo>,
) -> response::Result<(), DiscoError> {
    let user = common::fix_username(&user_info.preferred_username.unwrap_or_default(), &state.handler_config);
    let mut tx = state.pool.begin().await?;

    if !users::username_exists(&mut tx, &user).await? {
        return Err(DiscoError::NotFound(format!("user {} was not found", user)));
    }

    preferences::delete_user_preferences(&mut tx, &user).await?;

    tx.commit().await?;

    Ok(())
}
