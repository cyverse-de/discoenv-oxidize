use axum::{
    extract::{Json, Path, State},
    response,
};
use std::sync::Arc;

use crate::db::analyses;

use crate::app_state::DiscoenvState;
use crate::errors::DiscoError;
use debuff::analysis;

use super::common;

#[utoipa::path(
    get,
    path = "/analyses/{username}",
    params(
        ("username" = String, Path, description = "A username"),
    ),
    security(
      ("api_key" = []),  
    ),
    responses(
        (status = 200, description = "Lists all of a user's analyses", body = Bags),
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
    tag = "analyses"
)]
pub async fn get_user_analyses(
    State(state): State<Arc<DiscoenvState>>,
    Path(username): Path<String>,
) -> response::Result<Json<Vec<analysis::Analysis>>, DiscoError> {
    let pool = &state.pool;
    let handler_config = &state.handler_config;
    let mut tx = pool.begin().await?;
    let user = common::validate_username(&mut tx, &username, handler_config).await?;
    Ok(Json(analyses::get_user_analyses(&mut tx, &user).await?))
}
