use crate::db::groups;
use debuff::groups::{Permission, PermissionList, ResourceIn, ResourceOut, SubjectOut};
use sqlx::query_as;

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

// There are some queries that result in the fields returned being optional, so
// this struct is included for use with those queries in the query_as! macro.
struct PermissionsOptionsRecord {
    id: Option<uuid::Uuid>,
    internal_subject_id: Option<uuid::Uuid>,
    subject_id: Option<String>,
    subject_type: Option<String>,
    resource_id: uuid::Uuid,
    resource_name: Option<String>,
    resource_type: Option<String>,
    permission_level: Option<i32>,
}

impl From<PermissionsOptionsRecord> for Permission {
    fn from(item: PermissionsOptionsRecord) -> Self {
        Permission {
            id: item.id.unwrap_or_default().to_string(),

            permission_level: item.permission_level.unwrap_or_default(),

            subject: Some(SubjectOut {
                id: item.internal_subject_id.unwrap_or_default().to_string(),
                subject_id: item.subject_id.unwrap_or_default(),
                subject_type: item.subject_type.unwrap_or_default(),
                subject_source_id: String::new(),
            }),

            resource: Some(ResourceOut {
                id: item.resource_id.to_string(),
                name: item.resource_name.unwrap_or_default(),
                resource_type: item.resource_type.unwrap_or_default(),
            }),
        }
    }
}

// List the permissions for the provided resource.
pub async fn resource_perms<'a, E, F>(
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

// List all of the permissions, and we do mean all of them. You probably don't want to call this.
pub async fn list_perms<'a, E, F>(conn: E, groups_conn: F) -> Result<PermissionList, sqlx::Error>
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

// Lists permissions for 1 or more subjects.
pub async fn subject_perms<'a, E, F>(
    conn: E,
    groups_conn: F,
    subject_ids: &[String],
) -> Result<PermissionList, sqlx::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
    F: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    let mut permissions: Vec<Permission> = query_as!(
        PermissionsOptionsRecord,
        r#"
            SELECT DISTINCT ON (r.id)
	            first_value(p.id) OVER w AS id,
	            first_value(s.id) OVER w AS internal_subject_id,
	            first_value(s.subject_id) OVER w AS subject_id,
	            first_value(s.subject_type) OVER w AS "subject_type: String",
	            r.id AS resource_id,
	            first_value(r.name) OVER w AS resource_name,
	            first_value(rt.name) OVER w AS resource_type,
	            first_value(pl.precedence) OVER w AS permission_level
	        FROM permissions.permissions p
	        JOIN permissions.permission_levels pl ON p.permission_level_id = pl.id
	        JOIN permissions.subjects s ON p.subject_id = s.id
	        JOIN permissions.resources r ON p.resource_id = r.id
	        JOIN permissions.resource_types rt ON r.resource_type_id = rt.id
	        WHERE s.subject_id = any($1)
	        WINDOW w AS (PARTITION BY r.id ORDER BY pl.precedence)
            ORDER BY r.id
        "#,
        subject_ids,
    )
    .fetch_all(conn)
    .await?
    .into_iter()
    .map(|r| r.into())
    .collect();

    groups::add_source_id(groups_conn, &mut permissions).await?;

    Ok(PermissionList { permissions })
}

// List permissions for subjects, optionally filtering the results by resource
// type name and min level.
pub async fn filtered_subject_perms<'a, E, F>(
    conn: E,
    groups_conn: F,
    subject_ids: &[String],
    resource_name: Option<String>,
    rt_name: Option<String>,
    min_level: Option<i32>,
) -> Result<PermissionList, sqlx::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
    F: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    let mut l = subject_perms(conn, groups_conn, subject_ids).await?;

    l.permissions.retain(|p| {
        let mut should_retain: bool = false;

        if let Some(r) = p.resource.as_ref() {
            if let Some(rt) = rt_name.as_ref() {
                should_retain = should_retain && (r.resource_type == *rt);
            }

            if let Some(rname) = resource_name.as_ref() {
                should_retain = should_retain && (r.name == *rname);
            }
        }

        if let Some(min) = min_level {
            should_retain = should_retain && (p.permission_level <= min);
        }

        should_retain
    });

    Ok(l)
}
