use serde::{Deserialize, Serialize};
use sqlx::{query, query_as, types::Uuid};
use utoipa::ToSchema;

/// A record containing a user's saved searches.
#[derive(Debug, Default, Serialize, Deserialize, ToSchema, Clone, PartialEq)]
pub struct SavedSearches {
    /// The unique identifier for the record.
    pub id: Uuid,

    /// The UUID of the user that created the saved searches.
    pub user_id: Uuid,

    /// The saved searches serialized as a string of JSON.
    pub saved_searches: String,
}

/// A list of saved searches
#[derive(Debug, Default, Serialize, Deserialize, ToSchema, Clone, PartialEq)]
pub struct SavedSearchesList {
    pub saved_searches: Vec<SavedSearches>,
}

pub async fn has_saved_searches<'a, E>(conn: E, username: &str) -> Result<bool, sqlx::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    let result = query!(
        r#"
            SELECT COUNT(*) > 0 AS has_saved_searches
            FROM user_saved_searches
            JOIN users ON user_saved_searches.user_id = users.id
            WHERE users.username = $1
        "#,
        username,
    )
    .fetch_one(conn)
    .await?;

    Ok(result.has_saved_searches.unwrap_or(false))
}

pub async fn get_saved_searches<'a, E>(
    conn: E,
    username: &str,
) -> Result<SavedSearches, sqlx::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    let saved_searches = query_as!(
        SavedSearches,
        r#"
            SELECT
                s.id,
                s.user_id,
                s.saved_searches
            FROM
                user_saved_searches s
            JOIN users u ON s.user_id = u.id
            WHERE u.username = $1
        "#,
        username,
    )
    .fetch_one(conn)
    .await?;

    Ok(saved_searches)
}

pub async fn add_saved_searches<'a, E>(
    conn: E,
    username: &str,
    saved_searches: &str,
) -> Result<Uuid, sqlx::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    let result = query!(
        r#"
            INSERT INTO user_saved_searches
                (user_id, saved_searches)
            VALUES
                ((SELECT id FROM users WHERE username = $1), $2)
            RETURNING id
        "#,
        username,
        saved_searches,
    )
    .fetch_one(conn)
    .await?;

    Ok(result.id)
}

pub async fn update_saved_searches<'a, E>(
    conn: E,
    username: &str,
    saved_searches: &str,
) -> Result<u64, sqlx::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    let result = query!(
        r#"
            UPDATE ONLY user_saved_searches
            SET saved_searches = $2
            FROM users
            WHERE user_saved_searches.user_id = users.id
            AND users.username = $1
        "#,
        username,
        saved_searches,
    )
    .execute(conn)
    .await?;

    Ok(result.rows_affected())
}

pub async fn delete_saved_searches<'a, E>(conn: E, username: &str) -> Result<u64, sqlx::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    let result = query!(
        r#"
            DELETE FROM user_saved_searches
            WHERE user_id = (SELECT id FROM users WHERE username = $1)
        "#,
        username,
    )
    .execute(conn)
    .await?;

    Ok(result.rows_affected())
}
