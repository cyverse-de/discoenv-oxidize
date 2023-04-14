use axum::{
    extract::{Json, State},
    response,
};
use axum_auth::AuthBasic;

use crate::{
    auth::{self, Authenticator},
    errors::DiscoError,
};

pub async fn get_token(
    AuthBasic((username, password)): AuthBasic,
    State(authz_opt): State<Option<Authenticator>>,
) -> response::Result<Json<auth::OIDCToken>, DiscoError> {
    let password = password.unwrap_or_default();

    if let Some(auth) = authz_opt {
        Ok(Json(
            auth.get_token(&username, &password)
                .await
                .map_err(|re| DiscoError::BadRequest(re.to_string()))?,
        ))
    } else {
        Err(DiscoError::BadRequest("authentication not enabled".into()))
    }
}
