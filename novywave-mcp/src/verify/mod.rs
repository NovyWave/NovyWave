pub mod config;
mod polling;
pub mod tests;

use crate::ws_server::{self, Command, Response};
use anyhow::{bail, Result};
use std::path::PathBuf;

pub use config::NovyWaveConfig;

#[derive(Debug)]
pub enum TestResult {
    Pass,
    Fail(String),
    Skip(String),
}

impl TestResult {
    pub fn is_pass(&self) -> bool {
        matches!(self, TestResult::Pass)
    }

    pub fn is_fail(&self) -> bool {
        matches!(self, TestResult::Fail(_))
    }
}

pub struct VerifyOptions {
    pub workspace: PathBuf,
    pub timeout_ms: u64,
    pub verbose: bool,
}

/// Command runner that connects to WS server as client
pub enum CommandRunner {
    Remote { port: u16 },
}

impl CommandRunner {
    pub async fn send_command(&self, command: Command) -> Result<Response> {
        match self {
            CommandRunner::Remote { port } => ws_server::send_command_to_server(*port, command).await,
        }
    }

    pub async fn is_connected(&self) -> bool {
        match self {
            CommandRunner::Remote { port } => {
                ws_server::send_command_to_server(*port, Command::GetStatus)
                    .await
                    .is_ok()
            }
        }
    }
}

pub async fn run_verify(options: VerifyOptions) -> Result<bool> {
    println!("🧪 NovyWave Verification Tests");
    println!("================================");
    println!("Workspace: {}", options.workspace.display());
    println!();

    let config_path = options.workspace.join(".novywave");
    let config = if config_path.exists() {
        println!("📄 Loading config from {}", config_path.display());
        Some(config::load_config(&config_path)?)
    } else {
        println!("⚠️  No .novywave config found, running basic tests only");
        None
    };

    let runner = CommandRunner::Remote { port: 9225 };

    println!("🔗 Connecting to WebSocket server on port 9225...");
    if !runner.is_connected().await {
        bail!(
            "WebSocket server not running or extension not connected.\n\
             Start the server with: makers start\n\
             Or manually: novywave-mcp server"
        );
    }
    println!("✅ Extension connected");
    println!();

    polling::wait_for_app_ready_runner(&runner, options.timeout_ms).await?;
    println!("✅ App ready");
    println!();

    println!("Running tests...");
    println!("─────────────────");

    let mut passed = 0;
    let mut failed = 0;
    let mut skipped = 0;

    let result = tests::test_no_loading_stuck_runner(&runner, options.timeout_ms).await;
    print_result("No stuck 'Loading workspace...'", &result);
    match result {
        TestResult::Pass => passed += 1,
        TestResult::Fail(_) => failed += 1,
        TestResult::Skip(_) => skipped += 1,
    }

    if let Some(ref cfg) = config {
        let result = tests::test_files_restored_runner(&runner, cfg, options.timeout_ms).await;
        print_result("Files restored in Files & Scopes", &result);
        match result {
            TestResult::Pass => passed += 1,
            TestResult::Fail(_) => failed += 1,
            TestResult::Skip(_) => skipped += 1,
        }

        let result = tests::test_variables_restored_runner(&runner, cfg, options.timeout_ms).await;
        print_result("Selected variables restored", &result);
        match result {
            TestResult::Pass => passed += 1,
            TestResult::Fail(_) => failed += 1,
            TestResult::Skip(_) => skipped += 1,
        }
    }

    println!();
    println!("═══════════════════════════════");
    println!(
        "Results: {} passed, {} failed, {} skipped",
        passed, failed, skipped
    );

    if failed > 0 {
        println!("❌ VERIFICATION FAILED");
        Ok(false)
    } else {
        println!("✅ VERIFICATION PASSED");
        Ok(true)
    }
}

fn print_result(name: &str, result: &TestResult) {
    match result {
        TestResult::Pass => println!("  ✅ {}", name),
        TestResult::Fail(msg) => println!("  ❌ {}: {}", name, msg),
        TestResult::Skip(msg) => println!("  ⏭️  {} (skipped: {})", name, msg),
    }
}
