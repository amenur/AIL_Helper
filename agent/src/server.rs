//! API HTTP local.
//!
//! Seguridad:
//! - Solo escucha en 127.0.0.1 (nunca en la red).
//! - CORS restringido a los orígenes de la web (y localhost para desarrollo).
//! - Comprobación de la cabecera Host, para frustrar ataques de DNS rebinding:
//!   una web maliciosa que resuelva su dominio a 127.0.0.1 enviaría su propio Host.
//! - Responde a la preflight de Private Network Access de Chrome.
//! - De momento solo hay endpoints de lectura. Las acciones (descargas, arranque)
//!   llegarán con un token de emparejamiento.

use axum::{
    Json, Router,
    extract::{Request, State},
    http::{HeaderValue, Method, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::get,
};
use std::sync::Arc;
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::detect;

pub const DEFAULT_PORT: u16 = 47800;

pub const DEFAULT_ORIGINS: &[&str] = &[
    "https://amenur.github.io",
    "http://localhost:4321",
    "http://127.0.0.1:4321",
];

#[derive(Clone)]
pub struct Config {
    pub port: u16,
    pub origins: Vec<String>,
}

struct AppState {
    allowed_hosts: Vec<String>,
}

pub fn router(config: &Config) -> Router {
    let origins: Vec<HeaderValue> = config.origins.iter().filter_map(|o| o.parse().ok()).collect();
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([Method::GET])
        .allow_private_network(true);

    let state = Arc::new(AppState {
        allowed_hosts: vec![format!("127.0.0.1:{}", config.port), format!("localhost:{}", config.port)],
    });

    Router::new()
        .route("/v1/health", get(health))
        .route("/v1/hardware", get(hardware))
        .layer(cors)
        .layer(middleware::from_fn_with_state(state.clone(), check_host))
        .with_state(state)
}

async fn check_host(State(state): State<Arc<AppState>>, req: Request, next: Next) -> Response {
    let host = req.headers().get(header::HOST).and_then(|h| h.to_str().ok()).unwrap_or("");
    if !state.allowed_hosts.iter().any(|h| h.eq_ignore_ascii_case(host)) {
        tracing::warn!(host, "petición rechazada: Host no permitido");
        return (StatusCode::FORBIDDEN, "Host no permitido").into_response();
    }
    next.run(req).await
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION") }))
}

async fn hardware() -> Response {
    // La detección hace E/S bloqueante (NVML, sysfs, procesos).
    match tokio::task::spawn_blocking(detect::hardware_report).await {
        Ok(report) => Json(report).into_response(),
        Err(e) => {
            tracing::error!("fallo en la detección: {e}");
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}
