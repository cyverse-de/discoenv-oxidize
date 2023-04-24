use crate::auth;
use crate::handlers;
use sqlx::{Pool, Postgres};

#[derive(Debug, Clone)]
pub struct DiscoenvState {
    pub pool: Pool<Postgres>,
    pub handler_config: handlers::config::HandlerConfiguration,
    pub auth: auth::Authenticator,
    pub admin_entitlements: Vec<String>,
}
