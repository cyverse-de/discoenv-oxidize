use serde::{Deserialize, Serialize};
use sqlx::{query, query_as, types::Uuid};
use utoipa::ToSchema;

/// A record containing a user's session.
#[derive(Debug, Default, Serialize, Deserialize, ToSchema, Clone, PartialEq)]
pub struct Session {
    /// The session's UUID
    pub id: Uuid,

    /// The UUID of the user that created the session.
    pub user_id: Uuid,

    /// The session stored as text.
    pub session: String,
}

/// A list of sessions
#[derive(Debug, Default, Serialize, Deserialize, ToSchema, Clone, PartialEq)]
pub struct Sessions {
    pub sessions: Vec<Session>,
}

pub async fn has_session<'a, E>(conn: E, username: &str) -> Result<bool, sqlx::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    let result = query!(
        r#"
            SELECT COUNT(*) > 0 AS has_session
            FROM user_sessions
            JOIN users ON user_sessions.user_id = users.id
            WHERE users.username = $1
        "#,
        username,
    )
    .fetch_one(conn)
    .await?;

    Ok(result.has_session.unwrap_or(false))
}

pub async fn list_sessions<'a, E>(conn: E, username: &str) -> Result<Sessions, sqlx::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    let sessions = query_as!(
        Session,
        r#"
            SELECT
                s.id,
                s.user_id,
                s.session
            FROM
                user_sessions s
            JOIN users u ON s.user_id = u.id
            WHERE u.username = $1
        "#,
        username,
    )
    .fetch_all(conn)
    .await?;

    Ok(Sessions { sessions })
}

pub async fn add_session<'a, E>(conn: E, username: &str, session: &str) -> Result<Uuid, sqlx::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    let result = query!(
        r#"
            INSERT INTO user_sessions
                (user_id, session)
            VALUES
                ((SELECT id FROM users WHERE username = $1), $2)
            RETURNING id
        "#,
        username,
        session,
    )
    .fetch_one(conn)
    .await?;

    Ok(result.id)
}

pub async fn update_session<'a, E>(
    conn: E,
    username: &str,
    session: &str,
) -> Result<u64, sqlx::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    let result = query!(
        r#"
            UPDATE ONLY user_sessions
            SET session = $2
            FROM users
            WHERE user_sessions.user_id = users.id
            AND users.username = $1
        "#,
        username,
        session,
    )
    .execute(conn)
    .await?;

    Ok(result.rows_affected())
}

pub async fn delete_session<'a, E>(
    conn: E,
    username: &str,
    session_id: &Uuid,
) -> Result<u64, sqlx::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    let result = query!(
        r#"
            DELETE FROM user_sessions
            WHERE user_sessions.id = $2
            AND user_sessions.user_id = (SELECT id FROM users WHERE username = $1)
        "#,
        username,
        session_id,
    )
    .execute(conn)
    .await?;

    Ok(result.rows_affected())
}
