use sqlx::{query, types::Uuid, PgPool};

pub async fn user_id(conn: &PgPool, username: &str) -> Result<Uuid, sqlx::Error> {
    let result = query!(r#"select id from users where username = $1"#, username,)
        .fetch_one(conn)
        .await?;

    Ok(result.id)
}
