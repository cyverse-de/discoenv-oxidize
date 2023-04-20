use anyhow::anyhow;
use axum::{
    middleware,
    routing::get,
    Router,
};
use axum_tracing_opentelemetry::{opentelemetry_tracing_layer, response_with_trace_layer};
use clap::Parser;
use discoenv::db::{bags, preferences, searches};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPool;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use anyhow::{Result, Context};
use discoenv::app_state::DiscoenvState;
use discoenv::auth::{self, middleware::{auth_middleware, require_entitlements}};
use discoenv::errors;
use discoenv::handlers;
use discoenv::signals::shutdown_signal;
use utoipa_swagger_ui::oauth;
use std::sync::Arc;

#[derive(Parser)]
#[command(author, version, about, long_about=None)]
struct Cli {
    /// Whether to include the user domain if it's missing from requests.
    #[arg(short, long, default_value_t = true)]
    append_user_domain: bool,

    /// The config file to read settings from.
    #[arg(short, long, default_value_t = String::from("/etc/cyverse/de/configs/service.yml"))]
    config: String
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct ConfigDB {
    uri: String
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct ConfigUsers {
    domain: String
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct ConfigEntitlements {
    admin: String
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct ConfigOauth {
    uri: String,
    realm: String,
    client_id: String,
    client_secret: String,
    entitlements: Option<ConfigEntitlements>
}


#[derive(Debug, Default, Serialize, Deserialize)]
struct Config {
    db: ConfigDB, 
    users: ConfigUsers,
    oauth: Option<ConfigOauth>,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let cfg_file = std::fs::File::open(&cli.config).expect("could not open configuration file");
    let cfg: Config = serde_yaml::from_reader(cfg_file).expect("could not read values from configuration file");

    let pool = Arc::new(PgPool::connect(&cfg.db.uri).await.context("error connecting to db")?);

    let handler_config = handlers::config::HandlerConfiguration{
        append_user_domain: cli.append_user_domain,
        user_domain: cfg.users.domain.clone(),
        do_auth: cfg.oauth.is_some(),
    };

    let mut state = DiscoenvState {
        pool,
        handler_config,
        auth: None,
        admin_entitlements: None,
    };
    
    let mut swagger_ui = SwaggerUi::new("/docs")
        .url("/openapi.json", ApiDoc::openapi());


    if cfg.oauth.is_some() {
        let o = cfg.oauth.unwrap_or_default();
        swagger_ui = swagger_ui
            .oauth(
                oauth::Config::new()
                    .client_id(&o.client_id)
                    .client_secret(&o.client_secret)
                    .realm(&o.realm)
            )
            .config(
                utoipa_swagger_ui::Config::default().oauth2_redirect_url(&o.uri)
            );
        state.auth = Some(auth::Authenticator::setup(
            &o.uri,
            &o.realm,
            &o.client_id,
            &o.client_secret,
        )?);

        if o.entitlements.is_some() {
            let e = o.entitlements.unwrap_or_default();
            let ent_parts: Vec<String> = e.admin.as_str().split(',').map(String::from).collect();
            state.admin_entitlements = Some(Arc::new(ent_parts));
        }
    }

    #[derive(OpenApi)]
    #[openapi(
        paths(
            handlers::analyses::get_user_analyses,
            handlers::bags::get_user_bags,
            handlers::bags::delete_user_bags,
            handlers::bags::add_user_bag,
            handlers::bags::user_has_bags,
            handlers::bags::get_bag,
            handlers::bags::update_bag,
            handlers::bags::delete_bag,
            handlers::bags::get_default_bag,
            handlers::bags::update_default_bag,
            handlers::bags::delete_default_bag,
            handlers::preferences::get_user_preferences,
            handlers::preferences::add_user_preferences,
            handlers::preferences::update_user_preferences,
            handlers::preferences::delete_user_preferences,
            handlers::searches::get_saved_searches,
            handlers::searches::add_saved_searches,
            handlers::searches::update_saved_searches,
            handlers::searches::delete_saved_searches,
        ),
        components(
            schemas(
                handlers::common::ID,
                bags::Bag, 
                bags::Bags, 
                preferences::Preferences,
                searches::SavedSearches,
                errors::DiscoError,
            )
        ),
        security(
            (),
            ("open_id_connect" = [])
        ),
        tags(
            (name = "user-info", description="User information API")
        )
    )]

    struct ApiDoc;
    axum_tracing_opentelemetry::tracing_subscriber_ext::init_subscribers()
       .map_err(|e| anyhow!(format!("{:?}", e)))? ;

    let pref_routes = Router::new()
        .route(
            "/:username",
            get(handlers::preferences::get_user_preferences)
                .put(handlers::preferences::add_user_preferences)
                .post(handlers::preferences::update_user_preferences)
                .delete(handlers::preferences::delete_user_preferences)
        );

    let searches_routes = Router::new()
        .route(
            "/:username",
            get(handlers::searches::get_saved_searches)
                .put(handlers::searches::add_saved_searches)
                .post(handlers::searches::update_saved_searches)
                .delete(handlers::searches::delete_saved_searches)
        );

    let sessions_routes = Router::new()
        .route(
            "/:username",
            get(handlers::sessions::get_user_sessions)
                .put(handlers::sessions::add_user_sessions)
                .post(handlers::sessions::update_user_sessions)
                .delete(handlers::sessions::delete_user_sessions)
        );

    let bag_routes = Router::new()
        .route("/", get(|| async {}))
        .route(
            "/:username",
            get(handlers::bags::get_user_bags)
                .head(handlers::bags::user_has_bags)
                .put(handlers::bags::add_user_bag)
                .delete(handlers::bags::delete_user_bags),
        )
        .route(
            "/:username/default",
            get(handlers::bags::get_default_bag)
                .post(handlers::bags::update_default_bag)
                .delete(handlers::bags::delete_default_bag),
        )
        .route(
            "/:username/:bag_id",
            get(handlers::bags::get_bag)
                .post(handlers::bags::update_bag)
                .delete(handlers::bags::delete_bag),
        );

    let auth_m = middleware::from_fn_with_state(state.clone(), auth_middleware);
    let ent_m = middleware::from_fn_with_state(state.clone(), require_entitlements);

    let analyses_routes = Router::new()
        .route(
            "/:username",
            get(handlers::analyses::get_user_analyses)
        )
        .layer(ent_m)
        .layer(auth_m);

    let app = Router::new()
        .route("/", get(|| async {}))
        .merge(swagger_ui)
        .nest("/analyses", analyses_routes)
        .nest("/bags", bag_routes)
        .nest("/searches", searches_routes)
        .nest("/sessions", sessions_routes)
        .nest("/preferences", pref_routes)
        .route("/otel", get(handlers::otel::report_otel))
        .route("/token", get(handlers::tokens::get_token))
        .layer(response_with_trace_layer())
        .layer(opentelemetry_tracing_layer())
        .with_state(state);

    let addr = "0.0.0.0:60000".parse()?;

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal()).await?;


    Ok(())
}
