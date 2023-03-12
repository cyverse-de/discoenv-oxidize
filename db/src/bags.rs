use serde::{Deserialize, Serialize};
use serde_json::Map;
use sqlx::{
    query, query_as,
    types::{Json, JsonValue, Uuid},
    PgPool,
};

use anyhow::Result;

#[derive(Serialize, Deserialize)]
pub struct Bag {
    pub id: Uuid,
    pub user_id: Uuid,
    pub contents: Json<Map<String, JsonValue>>,
}

pub async fn list_bags(conn: &PgPool) -> Result<Vec<Bag>> {
    Ok(query_as!(
        Bag,
        r#"select id, user_id, contents as "contents: Json<Map<String, JsonValue>>" from bags"#
    )
    .fetch_all(conn)
    .await?)
}

pub async fn list_user_bags(conn: &PgPool, user_id: Uuid) -> Result<Vec<Bag>> {
    Ok(query_as!(
        Bag,
        r#"select id, user_id, contents as "contents: Json<Map<String, JsonValue>>" from bags where user_id = $1"#,
        user_id
    )
    .fetch_all(conn)
    .await?)
}

pub async fn update_bag_contents(
    conn: &PgPool,
    id: Uuid,
    contents: Map<String, JsonValue>,
) -> Result<u64> {
    let result = query!(
        r#"update bags set contents = $2 where id = $1"#,
        id,
        JsonValue::Object(contents)
    )
    .execute(conn)
    .await?;

    Ok(result.rows_affected())
}

pub async fn add_bag(
    conn: &PgPool,
    user_id: Uuid,
    contents: Map<String, JsonValue>,
) -> Result<Uuid> {
    let result = query!(
        r#"insert into bags (user_id, contents) values ($1, $2) returning id"#,
        user_id,
        JsonValue::Object(contents)
    )
    .fetch_one(conn)
    .await?;

    Ok(result.id)
}
