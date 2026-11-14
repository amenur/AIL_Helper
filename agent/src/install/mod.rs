//! Instalación con un clic: llama.cpp + modelo + uv + Open WebUI, y arranque.

pub mod archive;
pub mod download;
pub mod github;
pub mod hf;
pub mod job;
pub mod llamacpp;
pub mod services;
pub mod uv;

/// En Windows, que los procesos hijos no abran una ventana de consola.
pub fn hide_window(cmd: &mut tokio::process::Command) {
    #[cfg(windows)]
    {
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        cmd.creation_flags(CREATE_NO_WINDOW);
    }
    #[cfg(not(windows))]
    let _ = cmd;
}
