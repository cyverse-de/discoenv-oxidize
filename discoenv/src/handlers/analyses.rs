use axum::{
    extract::{Json, Path, State},
    response,
};
use sqlx::postgres::PgPool;

use db::{analyses, users};

use debuff::analysis;
use service_errors::DiscoError;

use super::common;
use super::config;

pub async fn get_user_analyses(
    State((conn, cfg)): State<(PgPool, config::HandlerConfiguration)>,
    Path(username): Path<String>,
) -> response::Result<Json<Vec<analysis::Analysis>>, DiscoError> {
    let user = common::fix_username(&username, &cfg);
    let mut tx = conn.begin().await?;

    if !users::username_exists(&mut tx, &user).await? {
        return Err(DiscoError::NotFound(format!("user {} was not found", user)));
    }

    Ok(Json(analyses::get_user_analyses(&mut tx, &user).await?))
}
