use crate::app_state::DiscoenvState;
use axum::{
    extract::{Extension, State},
    headers::{authorization::Bearer, Authorization},
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
    RequestPartsExt, TypedHeader,
};
use std::sync::Arc;

use super::UserInfo;

pub async fn auth_middleware<B>(
    State(state): State<Arc<DiscoenvState>>,
    request: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode>
where
    B: Send,
{
    let authz = &state.auth;
    let (mut parts, body) = request.into_parts();

    let mut req: Request<B>;

    let bearer: TypedHeader<Authorization<Bearer>> = parts
        .extract()
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    if bearer.token().is_empty() {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let user_info = authz.validate_token(bearer.token()).await?;

    if !user_info.active {
        return Err(StatusCode::UNAUTHORIZED);
    }

    req = Request::from_parts(parts, body);
    req.extensions_mut().insert(user_info);

    Ok(next.run(req).await)
}

pub async fn require_entitlements<B>(
    State(state): State<Arc<DiscoenvState>>,
    request: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode>
where
    B: Send,
{
    let check_ents = &state.admin_entitlements;
    let (mut parts, body) = request.into_parts();

    // This check requires that the user info be passed in,
    // so if it's not there, they're unauthed.
    let user_info: Extension<UserInfo> = parts
        .extract()
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // If they have no entitlements in the user info,
    // they're not supposed to access this call.
    if user_info.entitlement.is_none() {
        return Err(StatusCode::FORBIDDEN);
    }
    let user_ents = user_info.entitlement.to_owned().unwrap_or(vec![]);
    let mut found_ent = false;

    // See if any of the user's entitlments are in the
    // list of admin entitlments listed in the configuration.
    for req_ent in check_ents.iter() {
        for user_ent in user_ents.iter() {
            if user_ent == req_ent {
                found_ent = true
            }
        }

        if found_ent {
            break;
        }
    }

    // If none of the user's entitlements are admin entitlements,
    // then they're not allowed to make the call.
    if !found_ent {
        return Err(StatusCode::FORBIDDEN);
    }

    // If the user didn't configure any admin entitlements, then let the call through.
    // This is useful for development, but could very well change. If it does, add an
    // else block and return an Err(StatusCode::FORBIDDEN) from it.

    let req = Request::from_parts(parts, body);
    Ok(next.run(req).await)
}
