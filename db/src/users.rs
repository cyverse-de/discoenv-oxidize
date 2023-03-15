use sqlx::{query, types::Uuid, PgPool};

pub async fn user_id(conn: &PgPool, username: &str) -> Result<Uuid, sqlx::Error> {
    let result = query!(r#"select id from users where username = $1"#, username,)
        .fetch_one(conn)
        .await?;

    Ok(result.id)
}

pub async fn user_id_exists<'a, E>(conn: E, user_id: Uuid) -> Result<bool, sqlx::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    let result = query!(
        r#"select count(*) > 0 as has_user from users where id = $1"#,
        user_id
    )
    .fetch_one(conn)
    .await?;

    Ok(result.has_user.unwrap_or(false))
}

pub async fn username_exists<'a, E>(conn: E, username: &str) -> Result<bool, sqlx::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    let result = query!(
        r#"select count(*) > 0 as has_user from users where username = $1"#,
        username
    )
    .fetch_one(conn)
    .await?;

    Ok(result.has_user.unwrap_or(false))
}
