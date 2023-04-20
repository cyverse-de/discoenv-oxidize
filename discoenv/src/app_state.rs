use crate::auth;
use crate::handlers;
use axum::extract::FromRef;
use sqlx::{Pool, Postgres};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct DiscoenvState {
    pub pool: Arc<Pool<Postgres>>,
    pub handler_config: handlers::config::HandlerConfiguration,
    pub auth: Option<auth::Authenticator>,
    pub admin_entitlements: Option<Arc<Vec<String>>>,
}

impl FromRef<DiscoenvState> for Arc<Pool<Postgres>> {
    fn from_ref(state: &DiscoenvState) -> Arc<Pool<Postgres>> {
        state.pool.clone()
    }
}

impl FromRef<DiscoenvState> for handlers::config::HandlerConfiguration {
    fn from_ref(state: &DiscoenvState) -> handlers::config::HandlerConfiguration {
        state.handler_config.clone()
    }
}

impl FromRef<DiscoenvState> for Option<auth::Authenticator> {
    fn from_ref(state: &DiscoenvState) -> Option<auth::Authenticator> {
        state.auth.clone()
    }
}

impl FromRef<DiscoenvState> for Option<Arc<Vec<String>>> {
    fn from_ref(state: &DiscoenvState) -> Option<Arc<Vec<String>>> {
        state.admin_entitlements.clone()
    }
}
