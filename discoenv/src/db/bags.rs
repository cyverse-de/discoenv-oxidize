use serde::{Deserialize, Serialize};
use serde_json::Map;
use sqlx::{
    query, query_as,
    types::{Json, JsonValue, Uuid},
};
use utoipa::ToSchema;

/// A JSON document stored in the database as a Bag.
#[derive(Debug, Default, Serialize, Deserialize, ToSchema, Clone, PartialEq)]
pub struct Bag {
    /// The unique identifier.
    pub id: Uuid,

    /// The UUID of the user that owns the bag.
    pub user_id: Uuid,

    // The JSON contents of the bag.
    #[schema(value_type = Object)]
    pub contents: Json<Map<String, JsonValue>>,
}

/// A vector of Bags.
#[derive(Debug, Default, Serialize, Deserialize, ToSchema, Clone, PartialEq)]
pub struct Bags {
    pub bags: Vec<Bag>,
}

pub async fn list_bags<'a, E>(conn: E) -> Result<Vec<Bag>, sqlx::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    Ok(query_as!(
        Bag,
        r#"select id, user_id, contents as "contents: Json<Map<String, JsonValue>>" from bags"#
    )
    .fetch_all(conn)
    .await?)
}

pub async fn list_user_bags<'a, E>(conn: E, username: &str) -> Result<Bags, sqlx::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    let bags = query_as!(
        Bag,
        r#"
            select 
                bags.id, 
                user_id, 
                contents as "contents: Json<Map<String, JsonValue>>" 
            from bags
            join users on users.id = bags.user_id 
            where users.username = $1
        "#,
        username
    )
    .fetch_all(conn)
    .await?;

    Ok(Bags { bags })
}

pub async fn add_user_bag<'a, E>(
    conn: E,
    username: &str,
    contents: Map<String, JsonValue>,
) -> Result<Uuid, sqlx::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    let r = query!(
        r#"
            insert into bags 
                (user_id, contents) 
            values 
                ((SELECT id from users where username = $1), $2) returning id"#,
        username,
        JsonValue::Object(contents)
    )
    .fetch_one(conn)
    .await?;

    Ok(r.id)
}

pub async fn delete_user_bags<'a, E>(conn: E, username: &str) -> Result<u64, sqlx::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    let result = query!(
        r#"
            delete from bags where user_id = (select id from users where username = $1)
        "#,
        username,
    )
    .execute(conn)
    .await?;

    Ok(result.rows_affected())
}

pub async fn delete_user_bag<'a, E>(
    conn: E,
    username: &str,
    bag_id: &Uuid,
) -> Result<u64, sqlx::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    Ok(
        query!(
            r#"
                delete from bags where id = $2 and user_id = (select id from users where username = $1)
            "#,
            username,
            bag_id,
        )
        .execute(conn)
        .await?
        .rows_affected()
    )
}

struct HasBags {
    has_bags: Option<bool>,
}

pub async fn user_has_bags<'a, E>(conn: E, username: &str) -> Result<bool, sqlx::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    let result = query_as!(
        HasBags,
        r#"
            select COUNT(*) > 0 as has_bags 
            from bags 
            where user_id = (
                select id 
                from users 
                where username = $1
            )
        "#,
        username,
    )
    .fetch_one(conn)
    .await?;

    Ok(result.has_bags.unwrap_or(false))
}

pub async fn bag_exists<'a, E>(conn: E, username: &str, bag_id: &Uuid) -> Result<bool, sqlx::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    let result = query!(
        r#"
            select count(*) > 0 as bag_exists
            from bags
            join users on bags.user_id = users.id
            and users.username = $1
            and bags.id = $2
        "#,
        username,
        bag_id
    )
    .fetch_one(conn)
    .await?;

    Ok(result.bag_exists.unwrap_or(false))
}

pub async fn update_bag_contents<'a, E>(
    conn: E,
    id: &Uuid,
    contents: Map<String, JsonValue>,
) -> Result<u64, sqlx::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    let result = query!(
        r#"update bags set contents = $2 where id = $1"#,
        id,
        JsonValue::Object(contents)
    )
    .execute(conn)
    .await?;

    Ok(result.rows_affected())
}

pub async fn get_default_bag<'a, E>(conn: E, username: &str) -> Result<Bag, sqlx::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    let row = query_as!(
        Bag,
        r#"
            SELECT
                b.id,
                b.user_id,
                b.contents as "contents: Json<Map<String, JsonValue>>"
            FROM bags b
            JOIN default_bags d ON b.id = d.bag_id
            JOIN users u ON d.user_id = u.id
            WHERE
                u.username = $1
        "#,
        username
    )
    .fetch_one(conn)
    .await?;

    Ok(row)
}

pub async fn get_bag<'a, E>(conn: E, username: &str, bag_id: &Uuid) -> Result<Bag, sqlx::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    Ok(query_as!(
        Bag,
        r#"
            SELECT
                b.id,
                b.user_id,
                b.contents as "contents: Json<Map<String, JsonValue>>"
            FROM bags b
            JOIN users u ON b.user_id = u.id
            WHERE
                u.username = $1
            AND
                b.id = $2
        "#,
        username,
        bag_id,
    )
    .fetch_one(conn)
    .await?)
}

pub async fn set_default_bag<'a, E>(
    conn: E,
    username: &str,
    bag_id: &Uuid,
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    query!(
        r#"
            WITH uid AS (
                SELECT id
                FROM users
                WHERE username = $1
            )
            INSERT INTO 
                default_bags 
            VALUES 
                ( (SELECT id FROM uid), $2 ) 
            ON CONFLICT (user_id) 
                DO UPDATE SET bag_id = $2
        "#,
        username,
        bag_id
    )
    .execute(conn)
    .await?;

    Ok(())
}

pub async fn update_default_bag<'a, E>(
    executor: E,
    username: &str,
    contents: Map<String, JsonValue>,
) -> Result<u64, sqlx::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    Ok(query!(
        r#"
            UPDATE bags
            SET contents = $2
            FROM default_bags, users
            WHERE bags.id = default_bags.bag_id
            AND default_bags.user_id = users.id
            AND users.username = $1
        "#,
        username,
        JsonValue::Object(contents)
    )
    .execute(executor)
    .await?
    .rows_affected())
}

pub async fn update_bag<'a, E>(
    executor: E,
    username: &str,
    bag_id: &Uuid,
    contents: Map<String, JsonValue>,
) -> Result<u64, sqlx::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    Ok(query!(
        r#"
            UPDATE bags
            SET contents = $3
            FROM users
            WHERE bags.id = $2
            AND bags.user_id = users.id
            AND users.username = $1
        "#,
        username,
        bag_id,
        JsonValue::Object(contents)
    )
    .execute(executor)
    .await?
    .rows_affected())
}

pub async fn has_default_bag<'a, E>(executor: E, username: &str) -> Result<bool, sqlx::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    let r = query_as!(
        HasBags,
        r#"
            SELECT 
                COUNT(*) > 0 as has_bags
            FROM
                bags b
            JOIN
                default_bags d ON b.id = d.bag_id
            JOIN
                users u ON d.user_id = u.id
            WHERE
                u.username = $1
        "#,
        username
    )
    .fetch_one(executor)
    .await?;

    Ok(r.has_bags.unwrap_or(false))
}

pub async fn delete_default_bag<'a, E>(conn: E, username: &str) -> Result<u64, sqlx::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    let r = query!(
        r#"
            DELETE FROM bags
            WHERE bags.id = (
                SELECT 
                    b.id
                FROM 
                    bags b
                JOIN
                    default_bags d ON b.id = d.bag_id
                JOIN
                    users u ON d.user_id = u.id
                WHERE
                    u.username = $1
            )
        "#,
        username
    )
    .execute(conn)
    .await?;

    Ok(r.rows_affected())
}

pub async fn delete_bag<'a, E>(conn: E, username: &str, bag_id: &Uuid) -> Result<u64, sqlx::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    Ok(query!(
        r#"
            DELETE FROM bags
            WHERE bags.id = $2
            AND bags.user_id = (SELECT id FROM users WHERE username = $1)
        "#,
        username,
        bag_id
    )
    .execute(conn)
    .await?
    .rows_affected())
}
