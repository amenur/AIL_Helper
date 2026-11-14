//! Trabajo de instalación: estado observable por la web y ejecución de los pasos.

use super::{hf, llamacpp, services, uv};
use crate::catalog;
use crate::paths::Paths;
use anyhow::{Result, bail};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct InstallRequest {
    pub model_id: String,
    pub quant: String,
    pub context: u32,
    pub gpu_layers: u32,
}

impl InstallRequest {
    /// Solo modelos y cuantizaciones del catálogo embebido, y valores en rango.
    pub fn validate(&self) -> Result<&'static catalog::Model> {
        let Some(model) = catalog::find(&self.model_id) else {
            bail!("modelo desconocido: {}", self.model_id);
        };
        if !model.quants.contains(&self.quant) {
            bail!("{} no está disponible en {}", self.quant, model.name);
        }
        if self.context < 512 || self.context > model.max_context {
            bail!("contexto fuera de rango (512–{})", model.max_context);
        }
        if self.gpu_layers > 999 {
            bail!("número de capas en GPU fuera de rango");
        }
        Ok(model)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StepState {
    Pending,
    Running,
    Done,
    Skipped,
    Error,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    pub id: &'static str,
    pub label: &'static str,
    pub state: StepState,
    pub detail: String,
    pub done_bytes: u64,
    pub total_bytes: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Phase {
    Idle,
    Running,
    Ready,
    Error,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct JobState {
    pub phase: Phase,
    pub model_id: Option<String>,
    pub quant: Option<String>,
    pub steps: Vec<Step>,
    pub error: Option<String>,
    pub chat_url: Option<String>,
    pub llama_backend: Option<llamacpp::LlamaBackend>,
}

impl Default for JobState {
    fn default() -> Self {
        Self { phase: Phase::Idle, model_id: None, quant: None, steps: Vec::new(), error: None, chat_url: None, llama_backend: None }
    }
}

fn fresh_steps() -> Vec<Step> {
    [
        ("llama", "Descargar llama.cpp"),
        ("model", "Descargar el modelo"),
        ("uv", "Instalar uv"),
        ("webui", "Instalar Open WebUI"),
        ("start", "Arrancar el chat"),
    ]
    .into_iter()
    .map(|(id, label)| Step { id, label, state: StepState::Pending, detail: String::new(), done_bytes: 0, total_bytes: None })
    .collect()
}

pub struct Agent {
    pub paths: Paths,
    pub client: reqwest::Client,
    pub job: Mutex<JobState>,
    pub services: tokio::sync::Mutex<services::Services>,
}

impl Agent {
    pub fn new(paths: Paths) -> Self {
        Self {
            paths,
            client: super::download::client(),
            job: Mutex::new(JobState::default()),
            services: tokio::sync::Mutex::new(services::Services::default()),
        }
    }

    fn update(&self, id: &str, f: impl FnOnce(&mut Step)) {
        let mut job = self.job.lock().expect("estado");
        if let Some(step) = job.steps.iter_mut().find(|s| s.id == id) {
            f(step);
        }
    }

    fn set(&self, id: &str, state: StepState, detail: impl Into<String>) {
        let detail = detail.into();
        self.update(id, |s| {
            s.state = state;
            s.detail = detail;
        });
    }

    fn progress(&self, id: &'static str) -> impl Fn(u64, Option<u64>, &str) + Sync + '_ {
        move |done, total, label| {
            let label = label.to_string();
            self.update(id, |s| {
                s.done_bytes = done;
                s.total_bytes = total;
                s.detail = label;
            });
        }
    }

    /// Marca el trabajo como iniciado. Falla si ya hay uno en marcha.
    pub fn begin(&self, req: &InstallRequest) -> Result<()> {
        let mut job = self.job.lock().expect("estado");
        if job.phase == Phase::Running {
            bail!("ya hay una instalación en marcha");
        }
        *job = JobState {
            phase: Phase::Running,
            model_id: Some(req.model_id.clone()),
            quant: Some(req.quant.clone()),
            steps: fresh_steps(),
            ..JobState::default()
        };
        Ok(())
    }

    pub async fn run(self: Arc<Self>, req: InstallRequest) {
        let result = self.clone().pipeline(&req).await;
        let mut job = self.job.lock().expect("estado");
        match result {
            Ok(()) => {
                job.phase = Phase::Ready;
                job.chat_url = Some(format!("http://localhost:{}", services::WEBUI_PORT));
            }
            Err(e) => {
                tracing::error!("instalación fallida: {e:#}");
                job.phase = Phase::Error;
                job.error = Some(format!("{e:#}"));
                if let Some(step) = job.steps.iter_mut().find(|s| s.state == StepState::Running) {
                    step.state = StepState::Error;
                }
            }
        }
    }

    async fn pipeline(self: Arc<Self>, req: &InstallRequest) -> Result<()> {
        let model = req.validate()?;
        let paths = &self.paths;

        // 1. llama.cpp
        let llama_server = match llamacpp::installed(paths) {
            Some((key, server)) => {
                self.set("llama", StepState::Skipped, format!("Ya instalado ({key})"));
                server
            }
            None => {
                self.set("llama", StepState::Running, "Consultando la última versión…");
                let gpus = tokio::task::spawn_blocking(crate::detect::gpus).await?;
                let main = gpus.first();
                let prefs = llamacpp::preferences(llamacpp::Os::current(), main, llamacpp::vulkan_available());
                let cuda = main.and_then(|g| g.cuda_version.as_deref()).and_then(|v| {
                    let (a, b) = v.split_once('.')?;
                    Some((a.parse().ok()?, b.parse().ok()?))
                });
                let installed = llamacpp::install(&self.client, paths, &prefs, cuda, &self.progress("llama")).await?;
                self.job.lock().expect("estado").llama_backend = Some(installed.backend);
                self.set("llama", StepState::Done, format!("{} ({:?})", installed.tag, installed.backend));
                llamacpp::installed(paths).map(|(_, s)| s).ok_or_else(|| anyhow::anyhow!("llama-server no encontrado"))?
            }
        };

        // 2. Modelo
        let model_file = match hf::installed(paths, model, &req.quant) {
            Some(path) => {
                self.set("model", StepState::Skipped, "Ya descargado");
                path
            }
            None => {
                self.set("model", StepState::Running, format!("{} {}", model.name, req.quant));
                let path = hf::download_model(&self.client, paths, model, &req.quant, &self.progress("model")).await?;
                self.set("model", StepState::Done, format!("{} {} verificado", model.name, req.quant));
                path
            }
        };

        // 3. uv
        if uv::uv_path(paths).exists() {
            self.set("uv", StepState::Skipped, "Ya instalado");
        } else {
            self.set("uv", StepState::Running, "Descargando uv…");
            uv::install_uv(&self.client, paths, &self.progress("uv")).await?;
            self.set("uv", StepState::Done, "Instalado");
        }

        // 4. Open WebUI
        if uv::webui_path(paths).exists() {
            self.set("webui", StepState::Skipped, "Ya instalado");
        } else {
            self.set("webui", StepState::Running, "Instalando Python y Open WebUI (varios minutos)…");
            let log = |line: &str| {
                let line = line.trim().to_string();
                if !line.is_empty() {
                    self.update("webui", |s| s.detail = line);
                }
            };
            uv::install_webui(paths, &log).await?;
            self.set("webui", StepState::Done, "Instalado");
        }

        // 5. Arranque
        self.set("start", StepState::Running, "Cargando el modelo en llama.cpp…");
        // El cerrojo se toma solo para parar y guardar procesos: arrancar puede tardar
        // minutos y /v1/status debe seguir respondiendo mientras tanto.
        self.services.lock().await.stop().await;
        let llama = services::start_llama(paths, &llama_server, &model_file, req.context, req.gpu_layers).await?;
        self.services.lock().await.llama = Some(llama);
        self.set("start", StepState::Running, "Arrancando Open WebUI (el primer arranque tarda más)…");
        let webui = services::start_webui(paths).await?;
        self.services.lock().await.webui = Some(webui);
        self.set("start", StepState::Done, format!("Chat listo en http://localhost:{}", services::WEBUI_PORT));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(model: &str, quant: &str, ctx: u32) -> InstallRequest {
        InstallRequest { model_id: model.into(), quant: quant.into(), context: ctx, gpu_layers: 999 }
    }

    #[test]
    fn validation_only_accepts_catalog_values() {
        assert!(req("qwen3-8b", "Q4_K_M", 8192).validate().is_ok());
        assert!(req("evil/repo", "Q4_K_M", 8192).validate().is_err());
        assert!(req("qwen3-8b", "Q2_K", 8192).validate().is_err());
        assert!(req("qwen3-8b", "Q4_K_M", 10_000_000).validate().is_err());
        let mut r = req("qwen3-8b", "Q4_K_M", 8192);
        r.gpu_layers = 5000;
        assert!(r.validate().is_err());
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let json = r#"{"modelId":"qwen3-8b","quant":"Q4_K_M","context":8192,"gpuLayers":99,"hfRepo":"evil/x"}"#;
        assert!(serde_json::from_str::<InstallRequest>(json).is_err());
    }

    #[test]
    fn only_one_job_at_a_time() {
        let agent = Agent::new(Paths::new(std::env::temp_dir().join("ail-test")));
        let r = req("qwen3-8b", "Q4_K_M", 8192);
        agent.begin(&r).unwrap();
        assert!(agent.begin(&r).is_err());
    }
}
