use std::collections;

use debuff::groups::{GroupInfo, Permission};
use sqlx::query;
use sqlx::Row;

pub async fn groups_for_subject<'a, E>(
    conn: E,
    subject_id: &str,
    group_name: &str,
) -> Result<Vec<GroupInfo>, sqlx::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    let groups = query(
        r#"
            SELECT
                group_id,
                group_name
            FROM
                grouper_memberships_v
            WHERE subject_id = $1
            AND group_name LIKE $2
            AND list_name = 'members';
        "#,
    )
    .bind(subject_id)
    .bind(group_name)
    .fetch_all(conn)
    .await?
    .into_iter()
    .map(|rec| GroupInfo {
        id: rec.get("group_id"),
        name: rec.get("group_name"),
    })
    .collect();

    Ok(groups)
}

pub async fn add_source_id<'a, E>(
    conn: E,
    permissions: &mut [Permission],
) -> Result<(), sqlx::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    // Get the list of subject ids that we need the source id's for.
    let subject_ids: Vec<String> = permissions
        .iter_mut()
        .map(|p| match p.subject.as_ref() {
            Some(s) => s.id.clone(),
            None => String::new(),
        })
        .filter(|s| !s.is_empty())
        .collect();

    // Put together a HashMap of the subject_id and subject_source.
    let subjects = query(
        r#"
            SELECT
                subject_id,
                subject_source
            FROM grouper_members
            WHERE subject_id = ANY($1)
        "#,
    )
    .bind(&subject_ids)
    .fetch_all(conn)
    .await?
    .into_iter()
    .fold(
        collections::HashMap::<String, String>::new(),
        |mut acc, r| {
            acc.insert(r.get("subject_id"), r.get("subject_source"));
            acc
        },
    );

    for p in permissions.iter_mut() {
        if let Some(mut s) = p.subject.as_mut() {
            if let Some(source_id) = subjects.get(&s.subject_id) {
                s.subject_source_id = source_id.clone();
            }
        }
    }

    Ok(())
}
