use crate::db::groups;
use debuff::groups::{Permission, PermissionList, ResourceIn, ResourceOut, SubjectOut};
use sqlx::{query, query_as};

struct PermissionsRecord {
    id: uuid::Uuid,
    internal_subject_id: uuid::Uuid,
    subject_id: String,
    subject_type: String,
    resource_id: uuid::Uuid,
    resource_name: String,
    resource_type: String,
    permission_level: i32,
}

impl From<PermissionsRecord> for Permission {
    fn from(item: PermissionsRecord) -> Self {
        Permission {
            id: item.id.to_string(),

            permission_level: item.permission_level,

            subject: Some(SubjectOut {
                id: item.internal_subject_id.to_string(),
                subject_id: item.subject_id,
                subject_type: item.subject_type,
                subject_source_id: String::new(),
            }),

            resource: Some(ResourceOut {
                id: item.resource_id.to_string(),
                name: item.resource_name,
                resource_type: item.resource_type,
            }),
        }
    }
}

pub async fn list_resource_permissions<'a, E, F>(
    conn: E,
    groups_conn: F,
    resource: &ResourceIn,
) -> Result<PermissionList, sqlx::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
    F: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    let mut permissions: Vec<Permission> = query_as!(
        PermissionsRecord,
        r#"
            SELECT 
                p.id AS id,
	            s.id AS internal_subject_id,
	            s.subject_id AS subject_id,
	            s.subject_type AS "subject_type: String",
	            r.id AS resource_id,
	            r.name AS resource_name,
	            rt.name AS resource_type,
	            pl.precedence AS permission_level
	        FROM permissions.permissions p
	        JOIN permissions.permission_levels pl ON p.permission_level_id = pl.id
	        JOIN permissions.subjects s ON p.subject_id = s.id
	        JOIN permissions.resources r ON p.resource_id = r.id
	        JOIN permissions.resource_types rt ON r.resource_type_id = rt.id
            WHERE rt.name = $1 
            AND r.name = $2
	        ORDER BY s.subject_id
        "#,
        resource.name,
        resource.resource_type,
    )
    .fetch_all(conn)
    .await?
    .into_iter()
    .map(|r| r.into())
    .collect();

    groups::add_source_id(groups_conn, &mut permissions).await?;

    Ok(PermissionList { permissions })
}

pub async fn list_all_permissions<'a, E, F>(
    conn: E,
    groups_conn: F,
) -> Result<PermissionList, sqlx::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
    F: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    let mut permissions: Vec<Permission> = query_as!(
        PermissionsRecord,
        r#"
            SELECT
                p.id,
                s.id AS internal_subject_id,
                s.subject_id,
                s.subject_type AS "subject_type: String",
                r.id as resource_id,
                r.name as resource_name,
                rt.name as resource_type,
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
    .map(|r| r.into())
    .collect();

    groups::add_source_id(groups_conn, &mut permissions).await?;

    Ok(PermissionList { permissions })
}
