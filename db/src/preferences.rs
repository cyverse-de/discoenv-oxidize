use serde::{Deserialize, Serialize};
use serde_json::Map;
use sqlx::{
    query, query_as,
    types::{Json, JsonValue, Uuid},
};
use utoipa::ToSchema;

/// A Json document stored in the database as the user's preferences.
#[derive(Debug, Default, Serialize, Deserialize, ToSchema, Clone, PartialEq)]
pub struct Preferences {
    /// The unique identifier for the preferences document.
    pub id: Uuid,

    /// The UUID of the user that owns the document.
    pub user_id: Uuid,

    /// The JSON preferences.
    #[schema(value_type = Object)]
    pub preferences: Json<Map<String, JsonValue>>,
}

/// A list of preferences.
#[derive(Debug, Default, Serialize, Deserialize, ToSchema, Clone, PartialEq)]
pub struct PreferencesList {
    pub preferences: Vec<Preferences>,
}

pub async fn has_preferences<'a, E>(conn: E, username: &str) -> Result<bool, sqlx::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    let result = query!(
        r#"
            SELECT COUNT(*) > 0 AS has_preferences
            FROM user_preferences
            JOIN users ON user_preferences.user_id = users.id
            AND users.username = $1
        "#,
        username,
    )
    .fetch_one(conn)
    .await?;

    Ok(result.has_preferences.unwrap_or(false))
}

pub async fn user_preferences<'a, E>(
    conn: E,
    username: &str,
) -> Result<PreferencesList, sqlx::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    let preferences = query_as!(
        Preferences,
        r#"
            SELECT
                p.id,
                p.user_id,
                p.preferences as "preferences: Json<Map<String, JsonValue>>"
            FROM user_preferences p
            JOIN users u ON p.user_id = u.id
            WHERE u.username = $1
        "#,
        username,
    )
    .fetch_all(conn)
    .await?;

    Ok(PreferencesList { preferences })
}

pub async fn add_user_preferences<'a, E>(
    conn: E,
    username: &str,
    preferences: &str,
) -> Result<Uuid, sqlx::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    let r = query!(
        r#"
            INSERT INTO user_preferences
                (user_id, preferences)
            VALUES
                ((SELECT id from users where username = $1), $2) 
            RETURNING id
        "#,
        username,
        preferences,
    )
    .fetch_one(conn)
    .await?;

    Ok(r.id)
}

pub async fn update_user_preferences<'a, E>(
    conn: E,
    username: &str,
    preferences: &str,
) -> Result<u64, sqlx::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    let result = query!(
        r#"
            UPDATE user_preferences 
            SET preferences = $2
            FROM users
            WHERE user_preferences.user_id = users.id
            AND users.username = $1
        "#,
        username,
        preferences,
    )
    .execute(conn)
    .await?;

    Ok(result.rows_affected())
}

pub async fn delete_user_preferences<'a, E>(
    conn: E,
    username: &str,
    prefs_id: &Uuid,
) -> Result<u64, sqlx::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    Ok(query!(
        r#"
            DELETE FROM user_preferences 
            WHERE id = $2 
            AND user_id = (
                SELECT id 
                FROM users 
                WHERE username = $1
            )
        "#,
        username,
        prefs_id,
    )
    .execute(conn)
    .await?
    .rows_affected())
}
