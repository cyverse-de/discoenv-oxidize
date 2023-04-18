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
) -> response::Result<Json<auth::Token>, DiscoError> {
    let password = password.unwrap_or_default();
    println!("{:?}", username);
    println!("{:?}", password);
    println!("{:?}", authz_opt);

    if let Some(a) = authz_opt {
        Ok(Json(a.get_token(&username, &password).await?))
    } else {
        Err(DiscoError::BadRequest("authentication not enabled".into()))
    }
}
