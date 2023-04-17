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
    println!("{:?}", username);
    println!("{:?}", password);
    println!("{:?}", authz_opt);

    if let Some(a) = authz_opt {
        let r = a.get_token(&username, &password).await?;
        println!("{:?}", r);
        Ok(Json(r))
    } else {
        Err(DiscoError::BadRequest("authentication not enabled".into()))
    }
}
