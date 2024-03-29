use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    response, Extension,
};
use serde_json::Map;
use sqlx::types::{JsonValue, Uuid};
use std::sync::Arc;


use crate::{db::{bags::{self, Bag}, users}, auth::UserInfo};
use crate::db::bags::{list_user_bags, Bags};
use crate::errors::DiscoError;
use crate::app_state::DiscoenvState;
use super::common;

/// Get all of a user's bags.
/// 
/// Returns a complete listing of all of the user's bags. Mostly useful for administrative purposes.
#[utoipa::path(
    get,
    path = "/bags",
    responses(
        (status = 200, description = "Lists all of a user's bags", body = Bags),
        (status = 400, description = "Bad request.", 
            body = DiscoError,
            example = json!(DiscoError::BadRequest("bad request".to_owned()).create_service_error())),
        (status = 404, description = "User didn't exist.", 
            body = DiscoError,
            example = json!(DiscoError::NotFound("user wasn't found".to_owned()).create_service_error())),
        (status = 500, description = "Internal error.", 
            body = DiscoError,
            example = json!(DiscoError::Internal("internal error".to_owned()).create_service_error())),
    ),
    tag = "bag"
)]
pub async fn get_user_bags(
    State(state): State<Arc<DiscoenvState>>,
    Extension(user_info): Extension<UserInfo>,
) -> response::Result<Json<Bags>, DiscoError> {
    let user = common::fix_username(&user_info.preferred_username.unwrap_or_default(), &state.handler_config);
    let mut tx = state.pool.begin().await?;

    if !users::username_exists(&mut tx, &user).await? {
        return Err(DiscoError::NotFound(format!("user {} was not found", user)));
    }

    Ok(Json(list_user_bags(&mut tx, &user).await?))
}

/// Deletes all of a user's bags.
///
/// You probably don't want to actually call this.
#[utoipa::path(
    delete,
    path = "/bags",
    responses(
        (status = 200, description="Deleted all of the user's bags."),
        (status = 400, description = "Bad request.", 
            body = DiscoError, 
            example = json!(DiscoError::BadRequest("bad request".to_owned()).create_service_error())),
        (status = 404, description = "User didn't exist.", 
            body = DiscoError, 
            example = json!(DiscoError::NotFound("user doesn't exist".to_owned()).create_service_error())),
        (status = 500, description = "Internal error.", 
            body = DiscoError, 
            example = json!(DiscoError::Internal("internal error".to_owned()).create_service_error())),
    ),
    tag = "bag"
)]
pub async fn delete_user_bags(
    State(state): State<Arc<DiscoenvState>>,
    Extension(user_info): Extension<UserInfo>,
) -> response::Result<(), DiscoError> {
    let user = common::fix_username(&user_info.preferred_username.unwrap_or_default(), &state.handler_config);
    let mut tx = state.pool.begin().await?;

    if !users::username_exists(&mut tx, &user).await? {
        return Err(DiscoError::NotFound(format!("user {} was not found", user)));
    }

    bags::delete_user_bags(&mut tx, &user).await?;

    tx.commit().await?;

    Ok(())
}

/// Add a bag for a user.
/// 
/// Adds a new bag for a user. It is not set to the default.
#[utoipa::path(
    put,
    path = "/bags",
    request_body = JsonValue::Object,
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
pub  async fn add_user_bag(
    State(state): State<Arc<DiscoenvState>>,
    Extension(user_info): Extension<UserInfo>,
    Json(bag): Json<Map<String, JsonValue>>,
) -> response::Result<Json<common::ID>, DiscoError> {
    let user = common::fix_username(&user_info.preferred_username.unwrap_or_default(), &state.handler_config);
    let mut tx = state.pool.begin().await?;

    if !users::username_exists(&mut tx, &user).await? {
        return Err(DiscoError::NotFound(format!("user {} was not found", user)));
    }

    let u = bags::add_user_bag(&mut tx, &user, bag).await?;

    tx.commit().await?;

    let b = common::ID { id: u };
    
    Ok(Json(b))
}

/// Returns whether the user has bags.
/// 
/// Check the status code to tell whether the user has any bags defined.
#[utoipa::path(
    head,
    path = "/bags",
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
pub async fn user_has_bags(
    State(state): State<Arc<DiscoenvState>>,
    Extension(user_info): Extension<UserInfo>,
) -> response::Result<StatusCode, DiscoError> {
    let user = common::fix_username(&user_info.preferred_username.unwrap_or_default(), &state.handler_config);
    let mut status_code = StatusCode::OK;
    let has_bag = bags::user_has_bags(&state.pool, &user).await?;
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
    path = "/bags/{bag_id}",
    params(
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
pub async fn get_bag(
    State(state): State<Arc<DiscoenvState>>,
    Extension(user_info): Extension<UserInfo>,
    Path(bag_id): Path<Uuid>,
) -> response::Result<Json<Bag>, DiscoError> {
    let user = common::fix_username(&user_info.preferred_username.unwrap_or_default(), &state.handler_config);
    let mut tx = state.pool.begin().await?;

    if !users::username_exists(&mut tx, &user).await? {
        return Err(DiscoError::NotFound(format!("user {} was not found", user)));
    }

    if !bags::bag_exists(&mut tx, &user, &bag_id).await? {
        return Err(DiscoError::NotFound(format!("bag {} was not found", bag_id)));
    }

    Ok(Json(bags::get_bag(&mut tx, &user, &bag_id).await?))
}

/// Updates a particular bag for a user.
#[utoipa::path(
    post,
    path = "/bags/{bag_id}",
    params(
        ("bag_id" = String, Path, description = "A bag's UUID"),
    ),
    request_body = JsonValue::Object,
    responses(
        (status = 200, description = "The user's default bag.", body = Bag),
        (status = 404, description = "The user was not found.", 
            body = DiscoError, 
            example = json!(DiscoError::NotFound("user doesn't exist".to_owned()).create_service_error())),
        (status = 500, description = "Internal error.", 
            body = DiscoError, 
            example = json!(DiscoError::Internal("internal error".to_owned()).create_service_error())),
    ),
    tag = "bag"
)]
pub async fn update_bag(
    State(state): State<Arc<DiscoenvState>>,
    Extension(user_info): Extension<UserInfo>,
    Path(bag_id): Path<Uuid>,
    Json(bag): Json<Map<String, JsonValue>>,
) -> response::Result<Json<Bag>, DiscoError> {
    let user = common::fix_username(&user_info.preferred_username.unwrap_or_default(), &state.handler_config);
    let mut tx = state.pool.begin().await?;

    if !users::username_exists(&mut tx, &user).await? {
        return Err(DiscoError::NotFound(format!("user {} was not found", user)));
    }

    if !bags::bag_exists(&mut tx, &user, &bag_id).await? {
        return Err(DiscoError::NotFound(format!("bag {} was not found", bag_id)));
    }

    bags::update_bag(&mut tx, &user, &bag_id, bag).await?;

    let retval = bags::get_bag(&mut tx, &user, &bag_id).await?;
    
    tx.commit().await?;
    
    Ok(Json(retval))
}

/// Deletes a particular bag for a user.
#[utoipa::path(
    delete,
    path = "/bags/{bag_id}",
    params(
        ("bag_id" = String, Path, description = "A bag's UUID"),
    ),
    responses(
        (status = 200, description = "The user's bag was deleted."),
        (status = 404, description = "The user was not found.", 
            body = DiscoError, 
            example = json!(DiscoError::NotFound("user doesn't exist".to_owned()).create_service_error())),
        (status = 500, description = "Internal error.", 
            body = DiscoError, 
            example = json!(DiscoError::Internal("internal error".to_owned()).create_service_error())),
    ),
    tag = "bag"
)]
pub async fn delete_bag(
    State(state): State<Arc<DiscoenvState>>,
    Extension(user_info): Extension<UserInfo>,
    Path(bag_id): Path<Uuid>,
) -> response::Result<(), DiscoError> {
    let user = common::fix_username(&user_info.preferred_username.unwrap_or_default(), &state.handler_config);
    let mut tx = state.pool.begin().await?;

    if !users::username_exists(&mut tx, &user).await? {
        return Err(DiscoError::NotFound(format!("user {} was not found", user)));
    }

    bags::delete_bag(&mut tx, &user, &bag_id).await?;

    tx.commit().await?;

    Ok(())
}

/// Returns a user's default bag.
///
/// Creates the default bag first if it doesn't exist.
#[utoipa::path(
    get,
    path = "/bags/default",
    responses(
        (status = 200, description = "The user's default bag.", body = Bag),
        (status = 404, description = "The user was not found.", 
            body = DiscoError, 
            example = json!(DiscoError::NotFound("user doesn't exist".to_owned()).create_service_error())),
        (status = 500, description = "Internal error,", 
            body = DiscoError, 
            example = json!(DiscoError::Internal("internal error".to_owned()).create_service_error())),
    ),
    tag = "bag"
)]
pub async fn get_default_bag(
    State(state): State<Arc<DiscoenvState>>,
    Extension(user_info): Extension<UserInfo>,
) -> response::Result<Json<Bag>, DiscoError> {
    let user = common::fix_username(&user_info.preferred_username.unwrap_or_default(), &state.handler_config);
    let mut tx = state.pool.begin().await?;

    if !users::username_exists(&mut tx, &user).await? {
        return Err(DiscoError::NotFound(format!("user {} was not found", user)));
    }

    if !bags::has_default_bag(&mut tx, &user).await? {
        let new_bag: Map<String, JsonValue> = Map::new();
        let new_bag_uuid = bags::add_user_bag(&mut tx, &user, new_bag).await?;
        bags::set_default_bag(&mut tx, &user, &new_bag_uuid).await?;
    }

    Ok(Json(bags::get_default_bag(&mut tx, &user).await?))
}



/// Updates the default bag owned by a user.
///
/// This will create the default bag if it doesn't exist,.
#[utoipa::path(
    post,
    path = "/bags/default",
    request_body = JsonValue::Object,
    responses(
        (status = 200, description = "The user's default bag.", body = Bag),
        (status = 404, description = "The user was not found.", 
            body = DiscoError, 
            example = json!(DiscoError::NotFound("user doesn't exist".to_owned()).create_service_error())),
        (status = 500, description = "Internal error.", 
            body = DiscoError, 
            example = json!(DiscoError::Internal("internal error".to_owned()).create_service_error())),
    ),
    tag = "bag"
)]
pub async fn update_default_bag(
    State(state): State<Arc<DiscoenvState>>,
    Extension(user_info): Extension<UserInfo>,
    Json(bag): Json<Map<String, JsonValue>>,
) -> response::Result<Json<Bag>, DiscoError> {
    let user = common::fix_username(&user_info.preferred_username.unwrap_or_default(), &state.handler_config);
    let mut tx = state.pool.begin().await?;

    if !users::username_exists(&mut tx, &user).await? {
        return Err(DiscoError::NotFound(format!("user {} was not found", user)));
    }

    if !bags::has_default_bag(&mut tx, &user).await? {
        let new_bag_uuid = bags::add_user_bag(&mut tx, &user, bag).await?;
        bags::set_default_bag(&mut tx, &user, &new_bag_uuid).await?;
    } else {
        bags::update_default_bag(&mut tx, &user, bag).await?;
    }

    let retval = bags::get_default_bag(&mut tx, &user).await?;

    tx.commit().await?;

    Ok(Json(retval))
}

/// Deletes a user's default bag.
#[utoipa::path(
    delete,
    path = "/bags/default",
    responses(
        (status = 200, description = "The user's default bag was deleted."),
        (status = 404, description = "The user was not found.", 
            body = DiscoError, 
            example = json!(DiscoError::NotFound("user doesn't exist".to_owned()).create_service_error())),
        (status = 500, description = "Internal error.", 
            body = DiscoError, 
            example = json!(DiscoError::Internal("internal error".to_owned()).create_service_error())),
    ),
    tag = "bag"
)]
pub async fn delete_default_bag(
    State(state): State<Arc<DiscoenvState>>,
    Extension(user_info): Extension<UserInfo>,
) -> response::Result<(), DiscoError> {
    let user = common::fix_username(&user_info.preferred_username.unwrap_or_default(), &state.handler_config);
    let mut tx = state.pool.begin().await?;

    if !users::username_exists(&mut tx, &user).await? {
        return Err(DiscoError::NotFound(format!("user {} was not found", user)));
    }

    bags::delete_default_bag(&mut tx, &user).await?;

    tx.commit().await?;

    Ok(())
}
