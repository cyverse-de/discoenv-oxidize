use super::config;
use crate::db::users;
use crate::errors::DiscoError;
use serde::{Deserialize, Serialize};
use sqlx::types::Uuid;
use utoipa::ToSchema;

#[derive(Serialize, Deserialize, ToSchema)]
pub struct ID {
    pub id: Uuid,
}

pub fn fix_username(username: &str, cfg: &config::HandlerConfiguration) -> String {
    let mut retval: String = username.into();

    if cfg.append_user_domain && !cfg.user_domain.is_empty() && !retval.ends_with(&cfg.user_domain)
    {
        if !cfg.user_domain.starts_with('@') {
            retval = format!("{}@", retval);
        }
        retval = format!("{}{}", retval, cfg.user_domain);
    }

    retval
}

pub async fn validate_username<'a, E>(
    conn: E,
    username: &str,
    cfg: &config::HandlerConfiguration,
) -> Result<String, DiscoError>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    let user = fix_username(username, cfg);

    if !users::username_exists(conn, &user).await? {
        return Err(DiscoError::NotFound(format!("user {} was not found", user)));
    }

    Ok(user)
}
