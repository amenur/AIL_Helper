//! Carpetas del agente. Todo vive bajo ~/.ail-helper (o %USERPROFILE%\.ail-helper).

use std::path::PathBuf;

pub fn home() -> PathBuf {
    let var = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    std::env::var_os(var).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("."))
}

#[derive(Debug, Clone)]
pub struct Paths {
    pub root: PathBuf,
}

impl Paths {
    pub fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn default_root() -> PathBuf {
        home().join(".ail-helper")
    }

    pub fn bin(&self) -> PathBuf {
        self.root.join("bin")
    }
    pub fn llama(&self) -> PathBuf {
        self.root.join("llama.cpp")
    }
    pub fn models(&self) -> PathBuf {
        self.root.join("models")
    }
    pub fn logs(&self) -> PathBuf {
        self.root.join("logs")
    }
    pub fn webui_data(&self) -> PathBuf {
        self.root.join("open-webui")
    }
    /// Entorno de uv aislado dentro de ~/.ail-helper (no toca la instalación del usuario).
    pub fn uv_tools(&self) -> PathBuf {
        self.root.join("uv").join("tools")
    }
    pub fn uv_python(&self) -> PathBuf {
        self.root.join("uv").join("python")
    }
    pub fn uv_cache(&self) -> PathBuf {
        self.root.join("uv").join("cache")
    }
    pub fn token_file(&self) -> PathBuf {
        self.root.join("token")
    }
}

/// Nombre del ejecutable con la extensión de la plataforma.
pub fn exe(name: &str) -> String {
    if cfg!(windows) { format!("{name}.exe") } else { name.to_string() }
}
