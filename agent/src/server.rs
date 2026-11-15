//! API HTTP local.
//!
//! Seguridad:
//! - Solo escucha en 127.0.0.1 (nunca en la red).
//! - CORS restringido a los orígenes de la web (y localhost para desarrollo).
//! - Comprobación de la cabecera Host, para frustrar ataques de DNS rebinding:
//!   una web maliciosa que resuelva su dominio a 127.0.0.1 enviaría su propio Host.
//! - Responde a la preflight de Private Network Access de Chrome.
//! - Las acciones (instalar, arrancar, parar) exigen el token de emparejamiento
//!   (`Authorization: Bearer …`) y solo aceptan modelos del catálogo embebido.

use axum::{
    Json, Router,
    extract::{Request, State},
    http::{HeaderValue, Method, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use std::sync::Arc;
use tower_http::cors::{AllowOrigin, CorsLayer};

use crate::auth;
use crate::detect;
use crate::install::job::{Agent, InstallRequest};
use crate::install::services::Services;

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

pub struct AppState {
    allowed_hosts: Vec<String>,
    token: String,
    agent: Arc<Agent>,
}

pub fn router(config: &Config, token: String, agent: Arc<Agent>) -> Router {
    let origins: Vec<HeaderValue> = config.origins.iter().filter_map(|o| o.parse().ok()).collect();
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::list(origins))
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE])
        .allow_private_network(true);

    let state = Arc::new(AppState {
        allowed_hosts: vec![format!("127.0.0.1:{}", config.port), format!("localhost:{}", config.port)],
        token,
        agent,
    });

    let protected = Router::new()
        .route("/v1/pair/check", get(pair_check))
        .route("/v1/install", post(install))
        .route("/v1/status", get(status))
        .route("/v1/stop", post(stop))
        .route_layer(middleware::from_fn_with_state(state.clone(), require_token));

    Router::new()
        .route("/v1/health", get(health))
        .route("/v1/hardware", get(hardware))
        .merge(protected)
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

async fn require_token(State(state): State<Arc<AppState>>, req: Request, next: Next) -> Response {
    let given = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|h| h.strip_prefix("Bearer "))
        .unwrap_or("");
    if !auth::constant_time_eq(given, &state.token) {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({ "error": "no vinculado" }))).into_response();
    }
    next.run(req).await
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "status": "ok", "version": env!("CARGO_PKG_VERSION"), "install": true }))
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

async fn pair_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "paired": true }))
}

fn error(status: StatusCode, msg: impl ToString) -> Response {
    (status, Json(serde_json::json!({ "error": msg.to_string() }))).into_response()
}

async fn install(State(state): State<Arc<AppState>>, body: Result<Json<InstallRequest>, axum::extract::rejection::JsonRejection>) -> Response {
    let Ok(Json(req)) = body else {
        return error(StatusCode::BAD_REQUEST, "petición no válida");
    };
    if let Err(e) = req.validate() {
        return error(StatusCode::BAD_REQUEST, e);
    }
    if let Err(e) = state.agent.begin(&req) {
        return error(StatusCode::CONFLICT, e);
    }
    tracing::info!(model = %req.model_id, quant = %req.quant, "instalación iniciada");
    tokio::spawn(state.agent.clone().run(req));
    (StatusCode::ACCEPTED, Json(serde_json::json!({ "started": true }))).into_response()
}

async fn status(State(state): State<Arc<AppState>>) -> Response {
    let job = state.agent.job.lock().expect("estado").clone();
    let mut svc = state.agent.services.lock().await;
    let llama = Services::running(&mut svc.llama);
    let webui = Services::running(&mut svc.webui);
    let paths = &state.agent.paths;
    let installed = serde_json::json!({
        "llama": crate::install::llamacpp::installed(paths).is_some(),
        "webui": crate::install::uv::webui_path(paths).exists(),
    });
    Json(serde_json::json!({ "job": job, "services": { "llama": llama, "webui": webui }, "installed": installed })).into_response()
}

async fn stop(State(state): State<Arc<AppState>>) -> Response {
    state.agent.services.lock().await.stop().await;
    Json(serde_json::json!({ "stopped": true })).into_response()
}
