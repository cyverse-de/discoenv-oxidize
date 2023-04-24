use axum::{
    extract::{Json, State},
    response,
};
use axum_auth::AuthBasic;
use std::sync::Arc;

use crate::{
    app_state::DiscoenvState,
    auth::{self, Token},
    errors::DiscoError,
};

#[utoipa::path(
    get,
    path = "/token",
    security(
        ("http" = []) // Use HTTP basic auth here.
    ),
    responses(
        (status = 200, description = "Tokens generated for authentication", body = Token),
        (status = 401, description = "Unauthorized", body = DiscoError,
            example = json!(DiscoError::Unauthenticated("unauthorized".to_owned()).create_service_error())),
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
    tag = "auth"
)]
pub async fn get_token(
    State(state): State<Arc<DiscoenvState>>,
    AuthBasic((username, password)): AuthBasic,
) -> response::Result<Json<auth::Token>, DiscoError> {
    let a = &state.auth;
    let password = password.unwrap_or_default();
    let t: Token = a.get_token(&username, &password).await?;
    Ok(Json(t))
}
