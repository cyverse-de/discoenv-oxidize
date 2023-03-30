use axum::{
    extract::{Json, Path, State},
    response,
};
use sqlx::postgres::PgPool;

use crate::db::analyses;

use debuff::analysis;
use crate::errors::DiscoError;

use super::common;
use super::config;

#[utoipa::path(
    get,
    path = "/analyses/{username}",
    params(
        ("username" = String, Path, description = "A username"),
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
    State((conn, cfg)): State<(PgPool, config::HandlerConfiguration)>,
    Path(username): Path<String>,
) -> response::Result<Json<Vec<analysis::Analysis>>, DiscoError> {
    let mut tx = conn.begin().await?;
    let user = common::validate_username(&mut tx, &username, &cfg).await?;
    Ok(Json(analyses::get_user_analyses(&mut tx, &user).await?))
}
