use axum::{
    middleware,
    routing::get,
    Router,
};
use axum_server::tls_rustls::RustlsConfig;
use clap::Parser;
use discoenv::db::{bags, preferences, searches};
use sqlx::postgres::PgPool;
use utoipa::{
    openapi::security::{SecurityScheme, ApiKey, ApiKeyValue, Http, HttpAuthScheme}, 
    OpenApi, Modify,
};
use utoipa_swagger_ui::SwaggerUi;
use discoenv::app_state::DiscoenvState;
use discoenv::auth::{self, middleware::{auth_middleware, require_entitlements}};
use discoenv::errors;
use discoenv::handlers;
use discoenv::signals::shutdown_signal;
use utoipa_swagger_ui::oauth;
use std::sync::Arc;
use std::process;
use std::path::PathBuf;
use tracing::{info, debug, error};
use discoenv::config;

#[tokio::main]
async fn main() {
    // Set up tracing/logging using the RUST_LOG environment variable.
    tracing_subscriber::fmt::init();

    let cli = config::Cli::parse();

    debug!("reading config {}", cli.config);
    let cfg_file = std::fs::File::open(&cli.config).unwrap_or_else(|err| {
        eprintln!("error opening configuration file: {err}");
        process::exit(exitcode::IOERR);
    });
    
    let cfg: config::Config = serde_yaml::from_reader(cfg_file).unwrap_or_else(|err| {
        eprintln!("error reading values from configuration file: {err}");
        process::exit(exitcode::CONFIG);
    });


    debug!("connecting to database");
    let pool = PgPool::connect(&cfg.db.uri).await.unwrap_or_else(|err| {
        eprintln!("error connecting to the db: {err}");
        process::exit(exitcode::IOERR);
    });



    if cfg.oauth.is_none() {
        eprintln!("missing oauth configuration");
        process::exit(exitcode::CONFIG);
    }

    debug!("append_user_domain: {}", cli.append_user_domain);
    debug!("user_domain: {}", cfg.users.domain);
    debug!("do_auth: {}", cfg.oauth.is_some());
    
    let handler_config = handlers::config::HandlerConfiguration{
        append_user_domain: cli.append_user_domain,
        user_domain: cfg.users.domain.clone(),
        do_auth: cfg.oauth.is_some(),
    };

    let mut state = DiscoenvState {
        pool,
        handler_config,
        auth: auth::Authenticator::default(),
        admin_entitlements: vec![],
    };

    debug!("setting up swagger ui");
    let mut swagger_ui = SwaggerUi::new("/docs")
        .url("/openapi.json", ApiDoc::openapi());

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

    state.auth = auth::Authenticator::setup(
        &o.uri,
        &o.realm,
        &o.client_id,
        &o.client_secret,
    )
        .unwrap_or_else(|e| {
            eprintln!("error setting up authentication: {e}");
            process::exit(exitcode::SOFTWARE);
        });

    if o.entitlements.is_some() {
        let e = o.entitlements.unwrap_or_default();
        let ent_parts: Vec<String> = e.admin.as_str().split(',').map(String::from).collect();
        state.admin_entitlements = ent_parts;
    }

    #[derive(OpenApi)]
    #[openapi(
        paths(
            handlers::tokens::get_token,
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
                auth::Token,
            )
        ),
        modifiers(&SecurityAddon),
    )]

    struct ApiDoc;
    
    struct SecurityAddon;

    impl Modify for SecurityAddon {
        fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
            if let Some(components) = openapi.components.as_mut() {
                components.add_security_scheme(
                    "api_key",
                    SecurityScheme::ApiKey(
                        ApiKey::Header(ApiKeyValue::new("Authorization")),
                    ),
                );

                components.add_security_scheme(
                    "http",
                    SecurityScheme::Http(
                        Http::new(
                            HttpAuthScheme::Basic,
                        )
                    )
                );
            }
        }
    }


    debug!("setting up routes");
    let service_state = Arc::new(state);

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

    let auth_m = |s| middleware::from_fn_with_state(s, auth_middleware);
    let ent_m = |s| middleware::from_fn_with_state(s, require_entitlements);

    let analyses_routes = Router::new()
        .route(
            "/:username",
            get(handlers::analyses::get_user_analyses)
        )
        .layer(ent_m(service_state.clone()))
        .layer(auth_m(service_state.clone()));

    let app = Router::new()
        .route("/", get(|| async {}))
        .merge(swagger_ui)
        .nest("/analyses", analyses_routes)
        .nest("/bags", bag_routes)
        .nest("/searches", searches_routes)
        .nest("/sessions", sessions_routes)
        .nest("/preferences", pref_routes)
        .route("/token", get(handlers::tokens::get_token))
        .with_state(service_state);

    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], cli.port));
    
    debug!("no_tls is {}", cli.no_tls);
    if !cli.no_tls {
        let tls_config = RustlsConfig::from_pem_file(
            PathBuf::from(&cli.cert),
            PathBuf::from(&cli.key),
        )
        .await
        .unwrap_or_else(|err| {
            error!("error setting up TLS: {}", err);
            process::exit(exitcode::CONFIG);
        });

        info!("listening on {}", addr);
        axum_server::bind_rustls(addr, tls_config)
            .serve(app.into_make_service())
            .await
            .unwrap();
    } else {
        info!("listening on {}", addr);
        axum::Server::bind(&addr)
            .serve(app.into_make_service())
            .with_graceful_shutdown(shutdown_signal()).await.unwrap_or_else(|e| {
                error!("error shutting down: {e}");
                process::exit(exitcode::SOFTWARE);
            });
    }

    process::exit(exitcode::OK);
}
