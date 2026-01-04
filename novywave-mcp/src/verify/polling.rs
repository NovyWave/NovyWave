use super::CommandRunner;
use crate::ws_server::{Command, Response};
use anyhow::{bail, Result};
use tokio::time::{sleep, Duration, Instant};

pub async fn wait_for_app_ready_runner(runner: &CommandRunner, timeout_ms: u64) -> Result<()> {
    let start = Instant::now();
    let timeout = Duration::from_millis(timeout_ms);
    let poll_interval = Duration::from_millis(200);
    let stability_delay = Duration::from_millis(200);

    while start.elapsed() < timeout {
        match runner.send_command(Command::GetStatus).await {
            Ok(Response::Status { app_ready, .. }) if app_ready => {
                sleep(stability_delay).await;

                match runner.send_command(Command::GetStatus).await {
                    Ok(Response::Status { app_ready, .. }) if app_ready => {
                        return Ok(());
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        sleep(poll_interval).await;
    }

    bail!("Timeout waiting for app ready after {}ms", timeout_ms)
}
