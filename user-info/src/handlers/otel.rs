use axum::{extract::Json, response};
use serde::Serialize;
use utoipa::ToSchema;

use service_errors::DiscoError;

#[derive(Debug, Serialize, ToSchema)]
pub struct OtelReport {
    trace_id: String,
}

/// Returns an open telementry trace ID back to the caller.
///
/// Just useful to make sure that the open telemetry middleware is working.
pub async fn report_otel() -> response::Result<Json<OtelReport>, DiscoError> {
    let trace_id = axum_tracing_opentelemetry::find_current_trace_id()
        .ok_or(DiscoError::NotFound(String::from("trace not found")))?;

    Ok(Json(OtelReport { trace_id: trace_id }))
}
