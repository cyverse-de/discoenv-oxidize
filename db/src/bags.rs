use serde::{Deserialize, Serialize};
use serde_json::Map;
use sqlx::{
    query, query_as,
    types::{Json, JsonValue, Uuid},
    PgPool,
};

use crate::users;

#[derive(Serialize, Deserialize)]
pub struct Bag {
    pub id: Uuid,
    pub user_id: Uuid,
    pub contents: Json<Map<String, JsonValue>>,
}

#[derive(Serialize, Deserialize)]
pub struct Bags {
    pub bags: Vec<Bag>,
}

pub async fn list_bags(conn: &PgPool) -> Result<Vec<Bag>, sqlx::Error> {
    Ok(query_as!(
        Bag,
        r#"select id, user_id, contents as "contents: Json<Map<String, JsonValue>>" from bags"#
    )
    .fetch_all(conn)
    .await?)
}

pub async fn list_user_bags(conn: &PgPool, username: &str) -> Result<Bags, sqlx::Error> {
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

pub async fn add_user_bag(
    conn: &PgPool,
    username: &str,
    contents: Map<String, JsonValue>,
) -> Result<Uuid, sqlx::Error> {
    let user_id = users::user_id(&conn, username).await?;

    let result = query!(
        r#"insert into bags (user_id, contents) values ($1, $2) returning id"#,
        user_id,
        JsonValue::Object(contents)
    )
    .fetch_one(conn)
    .await?;

    Ok(result.id)
}

pub async fn update_bag_contents(
    conn: &PgPool,
    id: Uuid,
    contents: Map<String, JsonValue>,
) -> Result<u64, sqlx::Error> {
    let result = query!(
        r#"update bags set contents = $2 where id = $1"#,
        id,
        JsonValue::Object(contents)
    )
    .execute(conn)
    .await?;

    Ok(result.rows_affected())
}
