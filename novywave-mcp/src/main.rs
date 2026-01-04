use clap::{Parser, Subcommand};
use novywave_mcp::{mcp, verify, ws_server};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "novywave-mcp")]
#[command(about = "MCP server for NovyWave browser automation")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the WebSocket server daemon (for browser extension connection)
    Server {
        #[arg(long, default_value = "9225")]
        port: u16,
    },
    /// Run the MCP protocol server (connects to WS server daemon)
    Mcp {
        #[arg(long, default_value = "9225")]
        ws_port: u16,
    },
    /// Run verification tests against NovyWave app
    Verify {
        #[arg(long, short)]
        workspace: PathBuf,

        #[arg(long, default_value = "15000")]
        timeout: u64,

        #[arg(long, short)]
        verbose: bool,
    },
}

#[tokio::main]
async fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .format_timestamp(None)
        .init();

    let cli = Cli::parse();

    match cli.command {
        Commands::Server { port } => {
            ws_server::run_server_daemon(port).await;
        }
        Commands::Mcp { ws_port } => {
            mcp::run_mcp_server(ws_port).await;
        }
        Commands::Verify {
            workspace,
            timeout,
            verbose,
        } => {
            let options = verify::VerifyOptions {
                workspace,
                timeout_ms: timeout,
                verbose,
            };

            match verify::run_verify(options).await {
                Ok(true) => std::process::exit(0),
                Ok(false) => std::process::exit(1),
                Err(e) => {
                    eprintln!("Error: {:#}", e);
                    std::process::exit(2);
                }
            }
        }
    }
}
