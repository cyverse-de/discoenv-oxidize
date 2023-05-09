use crate::db::groups;
use debuff::groups::{Permission, PermissionList, ResourceOut, SubjectOut};
use sqlx::query;

pub async fn list_all_permissions<'a, E, F>(
    conn: E,
    groups_conn: F,
) -> Result<PermissionList, sqlx::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
    F: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    let mut permissions: Vec<Permission> = query!(
        r#"
            SELECT
                p.id,
                s.id AS internal_subject_id,
                s.subject_id,
                s.subject_type AS "subject_type: String",
                r.id as resource_id,
                r.name as resource_name,
                rt.id as resource_type_id,
                rt.name as resource_type_name,
                pl.precedence as permission_level
            FROM
                permissions.permissions p
            JOIN permissions.permission_levels pl ON p.permission_level_id = pl.id
            JOIN permissions.subjects s ON p.subject_id = s.id
            JOIN permissions.resources r ON p.resource_id = r.id
            JOIN permissions.resource_types rt ON r.resource_type_id = rt.id
            ORDER BY s.subject_id, r.name, pl.precedence
        "#,
    )
    .fetch_all(conn)
    .await?
    .into_iter()
    .map(|r| Permission {
        id: r.id.to_string(),

        permission_level: r.permission_level,

        subject: Some(SubjectOut {
            id: r.internal_subject_id.to_string(),
            subject_id: r.subject_id,
            subject_type: r.subject_type,
            subject_source_id: String::default(), // Weird, but it doesn't appear to get set in the permissions service code.
        }),

        resource: Some(ResourceOut {
            id: r.resource_id.to_string(),
            name: r.resource_name,
            resource_type: r.resource_type_name,
        }),
    })
    .collect();

    groups::add_source_id(groups_conn, &mut permissions).await?;

    Ok(PermissionList { permissions })
}
