mod mcp;
mod ws_server;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "novywave-mcp")]
#[command(about = "MCP server for NovyWave browser automation")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Mcp {
        #[arg(long, default_value = "9225")]
        ws_port: u16,
    },
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Mcp { ws_port } => {
            mcp::run_mcp_server(ws_port).await;
        }
    }
}
