use anyhow::{Context, Result};
use axum::{routing::get, Json, Router};
use axum_tracing_opentelemetry::{opentelemetry_tracing_layer, response_with_trace_layer};
use serde::Serialize;
use service_errors::DiscoError;
use service_signals::shutdown_signal;

#[derive(Debug, Serialize)]
struct OtelReport {
    trace_id: String,
}

async fn report_otel() -> Result<Json<OtelReport>, DiscoError> {
    let trace_id = axum_tracing_opentelemetry::find_current_trace_id()
        .context("failed to get trace id")
        .map_err(|a| DiscoError::Internal(a.to_string()))?;

    Ok(Json(OtelReport { trace_id: trace_id }))
}

#[tokio::main]
async fn main() {
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
            get(|| async {})
                .head(|| async {})
                .put(|| async {})
                .delete(|| async {}),
        )
        .route(
            "/:username/default",
            get(|| async {}).post(|| async {}).delete(|| async {}),
        )
        .route(
            "/:username/:bag_id",
            get(|| async {}).post(|| async {}).delete(|| async {}),
        );

    let app = Router::new()
        .nest("/bags", bag_routes)
        .route("/otel", get(report_otel))
        .layer(response_with_trace_layer())
        .layer(opentelemetry_tracing_layer());

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
