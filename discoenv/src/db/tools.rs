use debuff::{apps::IntegrationData, containers::Image, tools::Tool, user::User};
use sqlx::{self, query};

/// Returns a listing of tools added by the user.
pub async fn get_user_tools<'a, E>(conn: E, username: &str) -> Result<Vec<Tool>, sqlx::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    let records = query!(
        r#"
            SELECT
                t.id,
                t.name,
                t.location,
                t.description,
                t.version,
                t.attribution,
                t.time_limit_seconds,
                t.restricted,
                t.interactive,
                t.gpu_enabled,
                u.id AS user_id,
                u.username AS username,
                t.integration_data_id,
                i.integrator_name,
                i.integrator_email,
                c.id AS container_image_id,
                c.name AS container_image_name,
                c.tag AS container_image_tag,
                c.url as container_image_url,
                c.deprecated as container_image_deprecated,
                c.osg_image_path as container_image_osg_image_path                
            FROM tools t
            JOIN container_images c ON t.container_images_id = c.id
            JOIN integration_data i ON t.integration_data_id = i.id
            JOIN users u ON i.user_id = u.id
            WHERE
                u.username = $1
        "#,
        username
    )
    .fetch_all(conn)
    .await?;

    Ok(records
        .into_iter()
        .map(|r| Tool {
            uuid: r.id.to_string(),
            name: r.name,
            version: r.version,
            attribution: r.attribution.unwrap_or_default(),
            description: r.description.unwrap_or_default(),
            time_limit_seconds: r.time_limit_seconds,
            restricted: r.restricted,
            interactive: r.interactive,
            gpu_enabled: r.gpu_enabled,
            container_image: Some(Image {
                id: r.container_image_id.to_string(),
                name: r.container_image_name,
                tag: r.container_image_tag,
                url: r.container_image_url.unwrap_or_default(),
                osg_image_path: r.container_image_osg_image_path.unwrap_or_default(),
            }),
            integration_data: Some(IntegrationData {
                id: r.integration_data_id.to_string(),
                integrator_name: r.integrator_name,
                integrator_email: r.integrator_email,
                user: Some(User {
                    uuid: r.user_id.to_string(),
                    username: r.username,
                }),
            }),
        })
        .collect())
}

/// Returns a single tool owned by a user.
pub async fn get_user_tool<'a, E>(
    conn: E,
    username: &str,
    tool_id: &uuid::Uuid,
) -> Result<Tool, sqlx::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    let record = query!(
        r#"
            
            SELECT
                t.id,
                t.name,
                t.location,
                t.description,
                t.version,
                t.attribution,
                t.time_limit_seconds,
                t.restricted,
                t.interactive,
                t.gpu_enabled,
                u.id AS user_id,
                u.username AS username,
                t.integration_data_id,
                i.integrator_name,
                i.integrator_email,
                c.id AS container_image_id,
                c.name AS container_image_name,
                c.tag AS container_image_tag,
                c.url as container_image_url,
                c.deprecated as container_image_deprecated,
                c.osg_image_path as container_image_osg_image_path                
            FROM tools t
            JOIN container_images c ON t.container_images_id = c.id
            JOIN integration_data i ON t.integration_data_id = i.id
            JOIN users u ON i.user_id = u.id
            WHERE u.username = $1
            AND t.id = $2
        "#,
        username,
        tool_id
    )
    .fetch_one(conn)
    .await?;

    Ok(Tool {
        uuid: record.id.to_string(),
        name: record.name,
        version: record.version,
        attribution: record.attribution.unwrap_or_default(),
        description: record.description.unwrap_or_default(),
        time_limit_seconds: record.time_limit_seconds,
        restricted: record.restricted,
        interactive: record.interactive,
        gpu_enabled: record.gpu_enabled,
        integration_data: Some(IntegrationData {
            id: record.integration_data_id.to_string(),
            integrator_name: record.integrator_name,
            integrator_email: record.integrator_email,
            user: Some(User {
                uuid: record.user_id.to_string(),
                username: record.username,
            }),
        }),
        container_image: Some(Image {
            id: record.container_image_id.to_string(),
            name: record.container_image_name,
            tag: record.container_image_tag,
            url: record.container_image_url.unwrap_or_default(),
            osg_image_path: record.container_image_osg_image_path.unwrap_or_default(),
        }),
    })
}
