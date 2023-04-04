use futures::stream::StreamExt;
use futures::future::Future;
use async_nats::{Client, message::Message, ConnectOptions};
use axum::{
    routing::get,
    Router,
};
use axum_tracing_opentelemetry::{opentelemetry_tracing_layer, response_with_trace_layer};
use clap::Parser;
use discoenv::db::{bags, preferences, searches};
use serde::{Deserialize, Serialize};
use serde_yaml;
use sqlx::postgres::PgPool;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use anyhow::{Result, Context, anyhow};
use discoenv::errors;
use discoenv::handlers;
use discoenv::signals::shutdown_signal;
use debuff::user;
use tokio::task::spawn;

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
    crt: String,
    key: String,
    ca: String
}

#[derive(Debug, Serialize, Deserialize)]
struct ConfigNATS {
    server_urls: String,
    creds: String,
    max_reconnects: u32,
    reconnect_wait: u32,
    tls: ConfigNatsTls
}

#[derive(Debug, Serialize, Deserialize)]
struct Config {
    db: ConfigDB, 
    users: ConfigUsers,
    nats: ConfigNATS, 
}

async fn subscribe<F, Ft>(client: Client, subject: &str, handler: F) -> Result<(), anyhow::Error> 
where
    F: Fn(Client, Message) -> Ft,
    Ft: Future<Output = Result<(), anyhow::Error>> + Send,
{
    let mut subscriber = client.subscribe(subject.into()).await?;

    while let Some(msg) = subscriber.next().await {
        let cl = client.clone();
        handler(cl, msg).await?;
    }

    Ok::<(), anyhow::Error>(())
}

async fn handle_analysis_request(client: Client, msg: Message) -> Result<(), anyhow::Error> {
    spawn({
        let client = client.clone();

        async move {
            if let Ok(req) = serde_json::from_slice::<user::User>(&msg.payload) {
                println!("received user from={}", req.username);
                
                let outgoing = serde_json::json!(&req).as_str().unwrap_or_default().to_owned();
                client.publish(msg.reply.unwrap_or_default(), outgoing.into()).await?;
            }

            Ok::<(), anyhow::Error>(())

        }
    }).await?
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let cfg_file = std::fs::File::open(&cli.config).expect(&format!("could not open file {}", &cli.config));
    let cfg: Config = serde_yaml::from_reader(cfg_file).expect(&format!("could not read values from {}", &cli.config));

    let pool = PgPool::connect(&cfg.db.uri).await.context("error connecting to db")?;

    let handler_config = handlers::config::HandlerConfiguration{
        append_user_domain: cli.append_user_domain,
        user_domain: cfg.users.domain.clone(),
    };

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
        .with_state((pool, handler_config));

    let addr = "0.0.0.0:60000".parse()?;

    let nats_client = ConnectOptions::with_credentials_file(cfg.nats.creds.into()).await?
        .require_tls(cfg.nats.tls.enabled)
        .add_client_certificate(
            cfg.nats.tls.crt.into(),
            cfg.nats.tls.key.into()
        )
        .connect(cfg.nats.server_urls)
        .await?;

    tokio::spawn({
        let client = nats_client.clone();

        async move {
            subscribe(client, "cyverse.discoenv.analyses.get".into(), handle_analysis_request).await?;
            Ok::<(), anyhow::Error>(())
        }
    });

    axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}
