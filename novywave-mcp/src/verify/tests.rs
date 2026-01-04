use super::config::NovyWaveConfig;
use super::polling;
use super::CommandRunner;
use super::TestResult;
use crate::ws_server::{Command, Response};
use tokio::time::{sleep, Duration};

pub async fn test_no_loading_stuck_runner(runner: &CommandRunner, timeout_ms: u64) -> TestResult {
    if let Err(e) = polling::wait_for_app_ready_runner(runner, timeout_ms).await {
        return TestResult::Fail(format!("App not ready: {}", e));
    }

    sleep(Duration::from_millis(500)).await;

    match runner
        .send_command(Command::FindText {
            text: "Loading workspace".into(),
            exact: false,
        })
        .await
    {
        Ok(Response::TextMatches { found, .. }) => {
            if found {
                TestResult::Fail("Header still shows 'Loading workspace...'".into())
            } else {
                TestResult::Pass
            }
        }
        Ok(other) => TestResult::Fail(format!("Unexpected response: {:?}", other)),
        Err(e) => TestResult::Fail(format!("Command failed: {}", e)),
    }
}

pub async fn test_files_restored_runner(
    runner: &CommandRunner,
    config: &NovyWaveConfig,
    timeout_ms: u64,
) -> TestResult {
    if config.workspace.opened_files.is_empty() {
        return TestResult::Skip("No files configured in .novywave".into());
    }

    if let Err(e) = polling::wait_for_app_ready_runner(runner, timeout_ms).await {
        return TestResult::Fail(format!("App not ready: {}", e));
    }

    sleep(Duration::from_millis(300)).await;

    match runner.send_command(Command::GetPageText).await {
        Ok(Response::PageText { text }) => {
            if text.contains("Click 'Load Files' to add waveform files.") {
                return TestResult::Fail(
                    "Files panel shows empty state despite configured files".into(),
                );
            }
        }
        Ok(other) => return TestResult::Fail(format!("Unexpected response: {:?}", other)),
        Err(e) => return TestResult::Fail(format!("Failed to get page text: {}", e)),
    }

    match runner.send_command(Command::GetLoadedFiles).await {
        Ok(Response::LoadedFiles { files }) => {
            if files.is_empty() {
                return TestResult::Fail("No files loaded via test API".into());
            }

            for expected_file in &config.workspace.opened_files {
                let expected_filename = std::path::Path::new(expected_file)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or(expected_file);

                let found = files
                    .iter()
                    .any(|f| f.path.contains(expected_filename) || f.path.ends_with(expected_file));

                if !found {
                    return TestResult::Fail(format!(
                        "Expected file '{}' not found in loaded files: {:?}",
                        expected_file,
                        files.iter().map(|f| &f.path).collect::<Vec<_>>()
                    ));
                }
            }

            TestResult::Pass
        }
        Ok(other) => TestResult::Fail(format!("Unexpected response: {:?}", other)),
        Err(e) => TestResult::Fail(format!("Failed to get loaded files: {}", e)),
    }
}

pub async fn test_variables_restored_runner(
    runner: &CommandRunner,
    config: &NovyWaveConfig,
    timeout_ms: u64,
) -> TestResult {
    if config.workspace.selected_variables.is_empty() {
        return TestResult::Skip("No variables configured in .novywave".into());
    }

    if let Err(e) = polling::wait_for_app_ready_runner(runner, timeout_ms).await {
        return TestResult::Fail(format!("App not ready: {}", e));
    }

    sleep(Duration::from_millis(300)).await;

    match runner.send_command(Command::GetPageText).await {
        Ok(Response::PageText { text }) => {
            if text.contains("Select variables in the Variables panel to show them here.") {
                return TestResult::Fail(
                    "Variables panel shows empty state despite configured variables".into(),
                );
            }
        }
        Ok(other) => return TestResult::Fail(format!("Unexpected response: {:?}", other)),
        Err(e) => return TestResult::Fail(format!("Failed to get page text: {}", e)),
    }

    match runner.send_command(Command::GetSelectedVariables).await {
        Ok(Response::SelectedVariables { variables }) => {
            if variables.is_empty() {
                return TestResult::Fail("No variables selected via test API".into());
            }

            for expected_var in &config.workspace.selected_variables {
                let found = variables.iter().any(|v| v.unique_id == expected_var.unique_id);

                if !found {
                    return TestResult::Fail(format!(
                        "Expected variable '{}' not found in selected variables: {:?}",
                        expected_var.unique_id,
                        variables.iter().map(|v| &v.unique_id).collect::<Vec<_>>()
                    ));
                }
            }

            TestResult::Pass
        }
        Ok(other) => TestResult::Fail(format!("Unexpected response: {:?}", other)),
        Err(e) => TestResult::Fail(format!("Failed to get selected variables: {}", e)),
    }
}
