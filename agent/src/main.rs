//! AIL Helper: agente local.
//!
//! Uso:
//!   ail-agent                 Arranca la API en http://127.0.0.1:47800
//!   ail-agent --print         Muestra el informe de hardware en JSON y sale
//!   ail-agent --port 47801    Otro puerto
//!   ail-agent --allow-origin https://mi-dominio.com   Origen extra permitido (repetible)

mod detect;
mod report;
mod server;

use std::net::{Ipv4Addr, SocketAddr};

fn usage() -> ! {
    eprintln!("Uso: ail-agent [--print] [--port N] [--allow-origin URL]...");
    std::process::exit(2);
}

fn parse_args() -> (bool, server::Config) {
    let mut print = false;
    let mut config = server::Config {
        port: server::DEFAULT_PORT,
        origins: server::DEFAULT_ORIGINS.iter().map(|s| s.to_string()).collect(),
    };
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--print" => print = true,
            "--port" => config.port = args.next().and_then(|p| p.parse().ok()).unwrap_or_else(|| usage()),
            "--allow-origin" => config.origins.push(args.next().unwrap_or_else(|| usage())),
            "-h" | "--help" => usage(),
            _ => usage(),
        }
    }
    (print, config)
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt().with_target(false).init();
    let (print, config) = parse_args();

    if print {
        let report = detect::hardware_report();
        println!("{}", serde_json::to_string_pretty(&report).expect("serializable"));
        return;
    }

    let addr = SocketAddr::from((Ipv4Addr::LOCALHOST, config.port));
    let listener = match tokio::net::TcpListener::bind(addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("No se pudo abrir {addr}: {e}. ¿Hay otro agente en marcha?");
            std::process::exit(1);
        }
    };
    tracing::info!("AIL Helper agent {} escuchando en http://{addr}", env!("CARGO_PKG_VERSION"));
    tracing::info!("Orígenes permitidos: {}", config.origins.join(", "));

    axum::serve(listener, server::router(&config))
        .with_graceful_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
        .expect("servidor HTTP");
}
