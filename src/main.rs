use std::sync::Arc;

use clap::Parser;
use rmcp::ServiceExt;
use tracing::{error, info};

use figma_mcp::election::Election;
use figma_mcp::node::Node;
use figma_mcp::server::FigmaMcpServer;

#[derive(Parser, Debug)]
#[command(name = "figma-mcp")]
struct Cli {
    /// IP address to listen on (use 0.0.0.0 to accept remote connections)
    #[arg(long, default_value = "127.0.0.1")]
    ip: String,

    /// Port to listen on
    #[arg(long, default_value_t = 1994)]
    port: u16,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("figma_mcp=info")),
        )
        .init();

    let cli = Cli::parse();

    let parsed_ip: std::net::IpAddr = cli.ip.parse().unwrap_or_else(|_| {
        error!("invalid IP address: {:?}", cli.ip);
        std::process::exit(1);
    });

    if !parsed_ip.is_loopback() {
        info!(
            "WARNING: binding to {} — server will be reachable from the network with no authentication",
            cli.ip
        );
    }

    let version = env!("CARGO_PKG_VERSION").to_string();
    let node = Arc::new(Node::new(&cli.ip, cli.port, &version));
    let mut election = Election::new(&cli.ip, cli.port, node.clone());

    if let Err(e) = election.start().await {
        error!("election start: {}", e);
        std::process::exit(1);
    }

    info!(
        "Starting figma-mcp {} (role: {})",
        version,
        node.role_name().await
    );

    let server = FigmaMcpServer::new(node.clone());

    // Signal handling
    let node_clone = node.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.unwrap();
        info!("Shutting down...");
        election.stop();
        node_clone.stop().await;
    });

    // Serve over stdio
    let transport = rmcp::transport::stdio();
    match server.serve(transport).await {
        Ok(running_server) => {
            let _ = running_server.waiting().await;
        }
        Err(e) => {
            error!("mcp serve: {}", e);
            std::process::exit(1);
        }
    }
}
