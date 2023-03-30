use debuff::{
    analysis::{Analysis, AnalysisType},
    apps::{App, AppVersion, IntegrationData},
};
use sqlx::{query, types::chrono::NaiveDateTime};

use pbjson_types::Timestamp;

fn convert_ts(from: NaiveDateTime) -> Timestamp {
    let ts = from.timestamp_nanos();
    Timestamp {
        seconds: ts / 1_000_000_000,
        nanos: (ts % 1_000_000_000) as i32,
    }
}

/// Return a user's analyses.
///
/// Note that that analyses are stored in the jobs table.
pub async fn get_user_analyses<'a, E>(conn: E, username: &str) -> Result<Vec<Analysis>, sqlx::Error>
where
    E: sqlx::Executor<'a, Database = sqlx::Postgres>,
{
    let records = query!(
        r#"
            SELECT 
                j.id,
                j.job_name,
                j.job_description,
                j.result_folder_path,
                j.start_date,
                j.end_date,
                j.planned_end_date,
                j.status,
                j.deleted,
                j.notify,
                j.subdomain,
                j.parent_id,
                j.millicores_reserved,
                u.id                  as users_id,
                u.username            as users_username,
                a.id                  as apps_id,
                a.name                as apps_name,
                a.description         as apps_description,
                a.wiki_url            as apps_wiki_url,
                t.id                  as job_types_id,
                t.name                as job_types_name,
                t.system_id           as job_types_system_id,
                av.id                 as av_id,
                av.app_id             as av_app_id,
                av.version            as av_version,
                av.version_order      as av_version_order,
                av.deleted            as av_deleted,
                av.disabled           as av_disabled,
                av.integration_date   as av_integration_date,
                av.edited_date        as av_edited_date,
                intd.id               as integration_data_id,
                intd.integrator_name  as integrator_name,
                intd.integrator_email as integrator_email
            FROM jobs j
            JOIN users u ON u.id = j.user_id
            JOIN job_types t ON j.job_type_id = t.id
            JOIN app_versions av ON j.app_version_id = av.id
            JOIN apps a ON av.app_id = a.id
            JOIN integration_data intd on av.integration_data_id = intd.id
            WHERE u.username = $1
        "#,
        username
    )
    .fetch_all(conn)
    .await?;

    Ok(records
        .into_iter()
        .map(|r| {
            let mut analysis = Analysis::default();
            let mut app = App::default();
            let mut intd = IntegrationData::default();
            let mut av = AppVersion::default();
            let mut at = AnalysisType::default();
            let mut user = debuff::user::User::default();

            analysis.id = r.id.to_string();
            analysis.name = r.job_name;
            analysis.description = r.job_description.unwrap_or("".to_owned());
            analysis.deleted = r.deleted;
            analysis.notify = r.notify;
            analysis.result_folder_path = r.result_folder_path.unwrap_or_default();

            user.uuid = r.users_id.to_string();
            user.username = r.users_username;
            analysis.user = Some(user);

            at.id = r.job_types_id.to_string();
            at.name = r.job_types_name;
            at.system_id = r.job_types_system_id;
            analysis.kind = Some(at);

            app.id = r.apps_id.to_string();
            app.name = r.apps_name.unwrap_or("".to_owned());
            app.description = r.apps_description.unwrap_or("".to_owned());
            analysis.app = Some(app);

            intd.id = r.integration_data_id.to_string();
            intd.integrator_name = r.integrator_name;
            intd.integrator_email = r.integrator_email;

            av.id = r.av_id.to_string();
            av.app_id = r.apps_id.to_string();
            av.version = r.av_version;
            av.version_order = r.av_version_order.into();
            av.deleted = r.av_deleted;
            av.disabled = r.av_disabled;
            av.integration_date = Some(convert_ts(r.av_integration_date.unwrap_or_default()));
            av.edited_date = Some(convert_ts(r.av_edited_date.unwrap_or_default()));
            av.integration = Some(intd);
            analysis.app_version = Some(av);

            analysis
        })
        .collect())
}
