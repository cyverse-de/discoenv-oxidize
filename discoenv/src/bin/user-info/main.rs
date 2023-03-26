extern crate db;
extern crate service_signals;

use axum::{
    routing::get,
    Router,
};
use axum_tracing_opentelemetry::{opentelemetry_tracing_layer, response_with_trace_layer};
use clap::Parser;
use db::bags;
use serde::{Deserialize, Serialize};
use serde_yaml::{self};
use service_signals::shutdown_signal;
use sqlx::{
    postgres::PgPool,
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use discoenv::handlers;

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
struct Config {
    db: ConfigDB, 
    users: ConfigUsers,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    let cfg_file = std::fs::File::open(&cli.config).expect(&format!("could not open file {}", &cli.config));
    let cfg: Config = serde_yaml::from_reader(cfg_file).expect(&format!("could not read values from {}", &cli.config));

    println!("database URL: {}", cfg.db.uri);

    let pool = match PgPool::connect(&cfg.db.uri).await {
        Ok(pool) => pool,
        Err(e) => {
            println!("error connecting to the database: {}", e);
            return;
        }
    };

    let cfg = handlers::config::HandlerConfiguration{
        append_user_domain: cli.append_user_domain,
        user_domain: cfg.users.domain.clone(),
    };

    #[derive(OpenApi)]
    #[openapi(
        paths(
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
        ),
        components(
            schemas(
                handlers::common::ID,
                bags::Bag, 
                bags::Bags, 
                service_errors::DiscoError,
            )
        ),
        tags(
            (name = "user-info", description="User information API")
        )
    )]

    struct ApiDoc;
    match axum_tracing_opentelemetry::tracing_subscriber_ext::init_subscribers() {
        Ok(_) => {}
        Err(e) => {
            println!("error setting up opentelemetry: {}", e);
            return;
        }
    };

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

    let app = Router::new()
        .route("/", get(|| async {}))
        .merge(SwaggerUi::new("/docs").url("/openapi.json", ApiDoc::openapi()))
        .nest("/bags", bag_routes)
        .nest("/searches", searches_routes)
        .nest("/sessions", sessions_routes)
        .nest("/preferenaces", pref_routes)
        .route("/otel", get(handlers::otel::report_otel))
        .layer(response_with_trace_layer())
        .layer(opentelemetry_tracing_layer())
        .with_state((pool, cfg));

    let addr = match "0.0.0.0:60000".parse() {
        Ok(v) => v,
        Err(e) => {
            println!("error parsing address: {:?}", e);
            return;
        }
    };

    match axum::Server::bind(&addr)
        .serve(app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
    {
        Ok(_) => {}
        Err(e) => {
            println!("{:?}", e);
            return;
        }
    };
}
