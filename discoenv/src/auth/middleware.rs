use std::task::{Context, Poll};

use axum::{
    body::Body,
    extract::State,
    headers::{authorization::Bearer, Authorization},
    http::{Request, StatusCode},
    middleware::Next,
    response::Response,
    RequestPartsExt, TypedHeader,
};
use futures::future::BoxFuture;
use tower::{Layer, Service};

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

#[derive(Debug, Clone, Default)]
pub struct RequireEntitlementsLayer {
    required: Vec<String>,
}

impl RequireEntitlementsLayer {
    pub fn new(ent: String) -> Self {
        Self {
            required: vec![ent],
        }
    }

    pub fn new_multi(ents: Vec<String>) -> Self {
        Self { required: ents }
    }

    pub fn add(&mut self, ent: String) -> &mut RequireEntitlementsLayer {
        self.required.push(ent);
        self
    }

    pub fn add_multi(&mut self, ents: Vec<String>) -> &mut RequireEntitlementsLayer {
        self.required.extend(ents);
        self
    }
}

impl<S> Layer<S> for RequireEntitlementsLayer {
    type Service = RequireEntitlements<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RequireEntitlements {
            inner,
            required: self.required.clone(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct RequireEntitlements<S> {
    inner: S,
    required: Vec<String>,
}

impl<S> Service<Request<Body>> for RequireEntitlements<S>
where
    S: Service<Request<Body>, Response = Response> + Send + 'static,
    S::Future: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, ctx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(ctx)
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        // Do checks here

        let future = self.inner.call(request);
        Box::pin(async move {
            let response: Response = future.await?;
            Ok(response)
        })
    }
}
