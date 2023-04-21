use axum::{
    extract::{Json, State},
    response,
};
use axum_auth::AuthBasic;
use std::sync::Arc;

use crate::{app_state::DiscoenvState, auth, errors::DiscoError};

pub async fn get_token(
    State(state): State<Arc<DiscoenvState>>,
    AuthBasic((username, password)): AuthBasic,
) -> response::Result<Json<auth::Token>, DiscoError> {
    let a = &state.auth;
    let password = password.unwrap_or_default();
    Ok(Json(a.get_token(&username, &password).await?))
}
