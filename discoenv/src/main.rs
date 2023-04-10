use discoenv::errors::DiscoError;
use discoenv::handlers::common;
use discoenv::handlers::config;
use discoenv::handlers::config::HandlerConfiguration;
use futures::stream::StreamExt;
use futures::future::{BoxFuture, FutureExt};
use async_nats::ConnectOptions;
use axum::{
    routing::get,
    Router,
};
use axum_tracing_opentelemetry::{opentelemetry_tracing_layer, response_with_trace_layer};
use clap::Parser;
use discoenv::db::{bags, preferences, searches, analyses};
use serde::{Deserialize, Serialize};
use serde_yaml;
use sqlx::postgres::PgPool;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use anyhow::{Result, Context, anyhow};
use discoenv::errors;
use discoenv::handlers;
use discoenv::signals::shutdown_signal;
use debuff::{self, requests};
use std::sync::Arc;
use std::future::Future;

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

#[derive(Debug, Serialize, Deserialize)]
struct ConfigDB {
    uri: String
}

#[derive(Debug, Serialize, Deserialize)]
struct ConfigUsers {
    domain: String
}

#[derive(Debug, Serialize, Deserialize)]
struct ConfigNatsTls{
    enabled: bool,
    crt: Option<String>,
    key: Option<String>,
    ca: Option<String>
}

#[derive(Debug, Serialize, Deserialize)]
struct ConfigNATS {
    server_urls: String,
    creds: Option<String>,
    max_reconnects: Option<u32>,
    reconnect_wait: Option<u32>,
    tls: ConfigNatsTls
}

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    db: ConfigDB, 
    users: ConfigUsers,
    nats: ConfigNATS, 
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let cfg_file = std::fs::File::open(&cli.config).expect(&format!("could not open file {}", &cli.config));
    let cfg: Config = serde_yaml::from_reader(cfg_file).expect(&format!("could not read values from {}", &cli.config));

    let pool = Arc::new(PgPool::connect(&cfg.db.uri).await.context("error connecting to db")?);

    let handler_config = handlers::config::HandlerConfiguration{
        append_user_domain: cli.append_user_domain,
        user_domain: cfg.users.domain.clone(),
    };

    let nats_handler_config = Arc::new(handler_config.clone());

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

    let analyses_routes = Router::new()
        .route(
            "/:username",
            get(handlers::analyses::get_user_analyses)
        );

    let app = Router::new()
        .route("/", get(|| async {}))
        .merge(SwaggerUi::new("/docs").url("/openapi.json", ApiDoc::openapi()))
        .nest("/analyses", analyses_routes)
        .nest("/bags", bag_routes)
        .nest("/searches", searches_routes)
        .nest("/sessions", sessions_routes)
        .nest("/preferences", pref_routes)
        .route("/otel", get(handlers::otel::report_otel))
        .layer(response_with_trace_layer())
        .layer(opentelemetry_tracing_layer())
        .with_state((pool.clone(), handler_config));

    let addr = "0.0.0.0:60000".parse()?;

    let mut nats_opts: ConnectOptions;

    if cfg.nats.creds.is_none() {
        nats_opts = ConnectOptions::new();
    } else {
        nats_opts = ConnectOptions::with_credentials_file(
            cfg.nats.creds.unwrap_or_default().into()
        ).await?;
    }

    if cfg.nats.tls.enabled {
        nats_opts = nats_opts
            .require_tls(true)
            .add_root_certificates(cfg.nats.tls.ca.unwrap_or_default().into())
            .add_client_certificate(
                cfg.nats.tls.crt.unwrap_or_default().into(),
                cfg.nats.tls.key.unwrap_or_default().into()
            );
    }

    let nats_client = Arc::new(nats_opts
        .connect(cfg.nats.server_urls)
        .await?);

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal()).await?;


    Ok(())
}
