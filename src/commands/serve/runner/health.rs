//! Kubernetes-style probes.
//!
//! - `/health/live` answers as long as the process can serve HTTP. It never
//!   touches a dependency: a database outage must not make the orchestrator
//!   restart every pod at once, which would only add a restart storm to it.
//! - `/health/ready` tells whether this instance can serve traffic right now:
//!   not shutting down, and able to run a query on the database within a short
//!   deadline. A failure only takes the pod out of the load balancer.
//!
//! These routes are mounted outside the Sentry and tracing layers, so probes
//! polled every few seconds do not flood the transactions.

use super::common::Context;
use axum::{
    Json,
    extract::State,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
};
use diesel_async::RunQueryDsl;
use serde::Serialize;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tracing::warn;
use utoipa::ToSchema;
use utoipa_axum::router::OpenApiRouter;
use utoipa_axum::routes;

/// Kept below the probe `timeoutSeconds`, so the answer is an explicit 503
/// rather than a probe timeout.
const DATABASE_CHECK_TIMEOUT: Duration = Duration::from_secs(2);

#[derive(ToSchema, Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    Ok,
    Error,
    Skipped,
}

#[derive(ToSchema, Serialize, Clone, Copy, PartialEq, Eq, Debug)]
#[serde(rename_all = "snake_case")]
pub enum HealthStatus {
    Ok,
    Unavailable,
}

#[derive(ToSchema, Serialize)]
pub struct LivenessResponse {
    pub status: HealthStatus,
}

#[derive(ToSchema, Serialize)]
pub struct ReadinessChecks {
    pub database: CheckStatus,
    pub shutting_down: bool,
}

#[derive(ToSchema, Serialize)]
pub struct ReadinessResponse {
    pub status: HealthStatus,
    pub checks: ReadinessChecks,
}

/// Probe answers must never be served from a cache.
fn no_store<T: Serialize>(status: StatusCode, body: T) -> Response {
    (status, [(header::CACHE_CONTROL, "no-store")], Json(body)).into_response()
}

/// Liveness probe
#[utoipa::path(
    get,
    path = "/health/live",
    responses(
        (status = 200, description = "The process is able to serve HTTP", body = LivenessResponse)
    ),
    tag = super::common::PUBLIC_TAG
)]
async fn get_liveness() -> Response {
    no_store(
        StatusCode::OK,
        LivenessResponse {
            status: HealthStatus::Ok,
        },
    )
}

/// Readiness probe
#[utoipa::path(
    get,
    path = "/health/ready",
    responses(
        (status = 200, description = "Ready to serve traffic", body = ReadinessResponse),
        (status = 503, description = "Database unreachable or instance shutting down", body = ReadinessResponse)
    ),
    tag = super::common::PUBLIC_TAG
)]
async fn get_readiness(State(context): State<Arc<Context>>) -> Response {
    // Fully qualified: `RunQueryDsl` brings a `load` method into scope as well.
    let shutting_down = AtomicBool::load(&context.shutting_down, Ordering::Relaxed);

    // No point in querying the database once the instance is draining.
    let database = if shutting_down {
        CheckStatus::Skipped
    } else {
        check_database(&context).await
    };

    let ready = !shutting_down && database == CheckStatus::Ok;

    no_store(
        if ready {
            StatusCode::OK
        } else {
            StatusCode::SERVICE_UNAVAILABLE
        },
        ReadinessResponse {
            status: if ready {
                HealthStatus::Ok
            } else {
                HealthStatus::Unavailable
            },
            checks: ReadinessChecks {
                database,
                shutting_down,
            },
        },
    )
}

/// The timeout also covers the wait for a pooled connection: a saturated pool
/// means this instance cannot take more work either.
///
/// Failures are logged, not sent to Sentry: a probe fails every few seconds on
/// every pod during an outage, and the body stays generic because the route is
/// publicly reachable.
async fn check_database(context: &Context) -> CheckStatus {
    let connectors = context.builders.create();

    let check = async {
        let mut connection = connectors
            .local
            .pool
            .get()
            .await
            .map_err(|error| error.to_string())?;
        diesel::sql_query("SELECT 1")
            .execute(&mut connection)
            .await
            .map_err(|error| error.to_string())
    };

    match tokio::time::timeout(DATABASE_CHECK_TIMEOUT, check).await {
        Ok(Ok(_)) => CheckStatus::Ok,
        Ok(Err(error)) => {
            warn!("[Readiness] Database check failed: {}", error);
            CheckStatus::Error
        }
        Err(_) => {
            warn!(
                "[Readiness] Database check timed out after {:?}",
                DATABASE_CHECK_TIMEOUT
            );
            CheckStatus::Error
        }
    }
}

pub fn router() -> OpenApiRouter<Arc<Context>> {
    OpenApiRouter::new()
        .routes(routes!(get_liveness))
        .routes(routes!(get_readiness))
}
