use axum::{
    extract::State,
    headers::{authorization::Bearer, Authorization},
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
    RequestPartsExt, TypedHeader,
};

use super::{Authenticator, UserInfo};

pub async fn auth_middleware<B>(
    State(authz_opt): State<Option<Authenticator>>,
    request: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode>
where
    B: Send,
{
    let (mut parts, body) = request.into_parts();

    let mut req: Request<B>;
    let user_info: UserInfo;

    if let Some(authz) = authz_opt {
        let bearer: TypedHeader<Authorization<Bearer>> = parts
            .extract()
            .await
            .map_err(|_| StatusCode::UNAUTHORIZED)?;

        if bearer.token().is_empty() {
            return Err(StatusCode::UNAUTHORIZED);
        }

        user_info = authz.validate_token(bearer.token()).await?;

        if !user_info.active {
            return Err(StatusCode::UNAUTHORIZED);
        }
    } else {
        user_info = UserInfo::default();
    }

    req = Request::from_parts(parts, body);
    req.extensions_mut().insert(user_info);

    Ok(next.run(req).await)
}
