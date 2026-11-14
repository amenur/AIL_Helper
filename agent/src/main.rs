//! AIL Helper: agente local.
//!
//! Uso:
//!   ail-agent                 Arranca la API en http://127.0.0.1:47800 y abre la web para vincularla
//!   ail-agent --print         Muestra el informe de hardware en JSON y sale
//!   ail-agent --port 47801    Otro puerto
//!   ail-agent --allow-origin https://mi-dominio.com   Origen extra permitido (repetible)
//!   ail-agent --web-url URL   Web con la que vincularse (por defecto la oficial)
//!   ail-agent --no-browser    No abrir el navegador al arrancar

mod auth;
mod catalog;
mod detect;
mod install;
mod paths;
mod report;
mod server;

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;

const DEFAULT_WEB_URL: &str = "https://amenur.github.io/AIL_Helper";

struct Args {
    print: bool,
    open_browser: bool,
    web_url: String,
    config: server::Config,
}

fn usage() -> ! {
    eprintln!("Uso: ail-agent [--print] [--port N] [--allow-origin URL]... [--web-url URL] [--no-browser]");
    std::process::exit(2);
}

fn parse_args() -> Args {
    let mut args = Args {
        print: false,
        open_browser: true,
        web_url: DEFAULT_WEB_URL.to_string(),
        config: server::Config {
            port: server::DEFAULT_PORT,
            origins: server::DEFAULT_ORIGINS.iter().map(|s| s.to_string()).collect(),
        },
    };
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "--print" => args.print = true,
            "--no-browser" => args.open_browser = false,
            "--port" => args.config.port = it.next().and_then(|p| p.parse().ok()).unwrap_or_else(|| usage()),
            "--allow-origin" => args.config.origins.push(it.next().unwrap_or_else(|| usage())),
            "--web-url" => args.web_url = it.next().unwrap_or_else(|| usage()).trim_end_matches('/').to_string(),
            _ => usage(),
        }
    }
    args
}

/// Abre una URL en el navegador por defecto (sin pasar por un intérprete de comandos).
fn open_browser(url: &str) {
    let result = if cfg!(windows) {
        std::process::Command::new("rundll32").args(["url.dll,FileProtocolHandler", url]).spawn()
    } else {
        std::process::Command::new("xdg-open").arg(url).spawn()
    };
    if let Err(e) = result {
        tracing::warn!("no se pudo abrir el navegador: {e}");
    }
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_target(false).init();
    let args = parse_args();

    if args.print {
        let report = detect::hardware_report();
        println!("{}", serde_json::to_string_pretty(&report).expect("serializable"));
        return;
    }

    let paths = paths::Paths::new(paths::Paths::default_root());
    let token = match auth::load_or_create(&paths.token_file()) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("No se pudo crear el token en {}: {e}", paths.token_file().display());
            std::process::exit(1);
        }
    };
    let agent = Arc::new(install::job::Agent::new(paths));

    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, args.config.port));
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("No se pudo abrir {addr}: {e}. ¿Hay otro agente en marcha?");
            std::process::exit(1);
        }
    };

    // El token va en el fragmento (#): el navegador no lo envía a ningún servidor.
    let pair_url = format!("{}/vincular#token={token}", args.web_url);
    tracing::info!("AIL Helper agent {} escuchando en http://{addr}", env!("CARGO_PKG_VERSION"));
    println!("\nPara vincular la web con este agente, abre:\n\n  {pair_url}\n");
    if args.open_browser {
        open_browser(&pair_url);
    }

    let app = server::router(&args.config, token, agent.clone());
    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
        .expect("servidor HTTP");
    // Al salir, se paran llama-server y Open WebUI.
    agent.services.lock().await.stop().await;
}
