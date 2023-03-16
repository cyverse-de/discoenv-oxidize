use axum::{
    routing::get,
    Router,
};
use axum_tracing_opentelemetry::{opentelemetry_tracing_layer, response_with_trace_layer};
use clap::Parser;
use db::bags;
use service_signals::shutdown_signal;
use sqlx::{
    postgres::PgPool,
};
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use user_info::handlers;

#[derive(Parser)]
#[command(author, version, about, long_about=None)]
struct Cli {
    #[arg(short, long)]
    /// The connection string for the database in the format postgres:://user:password@host:port/database
    database_url: String,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    let pool = match PgPool::connect(&cli.database_url).await {
        Ok(pool) => pool,
        Err(e) => {
            println!("error connecting to the database: {}", e);
            return;
        }
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
                handlers::bags::ID,
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
        .merge(SwaggerUi::new("/docs").url("/openapi.json", ApiDoc::openapi()))
        .nest("/bags", bag_routes)
        .route("/otel", get(handlers::otel::report_otel))
        .layer(response_with_trace_layer())
        .layer(opentelemetry_tracing_layer())
        .with_state(pool);

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
