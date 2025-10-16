use std::{
    io::{BufRead as _, BufReader},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
};

use tracing::{error, info};

use crate::{
    code_run::run_iface::CodeRunTrait,
    hooks::iface::HookContext,
    models::{app_state::RemoteInfo, config::ProjectType, feature_test::FeatureTest},
    utils::{
        self,
        errors::{EmptyResult, ResultWithError},
    },
};

#[allow(dead_code)]
pub struct RunFlutter;

impl CodeRunTrait for RunFlutter {
    fn get_type(&self) -> ProjectType {
        ProjectType::Flutter
    }

    fn run(&self, ctx: &HookContext, features: &[FeatureTest]) -> EmptyResult {
        let state = ctx.read_state()?;
        let remote = state.remote.as_ref();

        let exec_dir = if remote.is_some() {
            // Use remote path for command execution context
            utils::dir::DirUtils::root_dir(remote)?.join("flutter_app")
        } else {
            utils::dir::DirUtils::curr_dir()?
        };

        // Prepare environment
        self.prepare_env(remote, &exec_dir)?;

        // Execute either locally or remotely
        if let Some(remote) = remote {
            info!("Running Flutter tests remotely");
            self.execute_remote(remote, &exec_dir, features)
        } else {
            info!("Running Flutter tests locally\n");
            self.execute_local(&exec_dir, features)
        }
    }
}

impl RunFlutter {
    pub fn new() -> Self {
        Self {}
    }

    fn prepare_env(&self, remote: Option<&RemoteInfo>, exec_dir: &Path) -> EmptyResult {
        self.build()?;
        self.sync_build(remote, exec_dir)?;
        self.sync_tests(remote, exec_dir)?;
        self.sync_driver(remote, exec_dir)?;
        self.sync_linux(remote, exec_dir)?;
        self.sync_pubspec(remote, exec_dir)?;

        Ok(())
    }

    fn build(&self) -> EmptyResult {
        info!("Building Flutter app...");

        let mut command = Command::new("sh");
        command
            .current_dir(utils::dir::DirUtils::curr_dir()?)
            .arg("-c")
            .arg("flutter build linux --debug --target=integration_test/generated/all_tests.dart")
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        let status = command.status()?;

        if !status.success() {
            return Err("Flutter build failed".into());
        }

        Ok(())
    }

    fn sync_build(&self, remote: Option<&RemoteInfo>, exec_dir: &Path) -> EmptyResult {
        let local_flutter_dir = utils::dir::DirUtils::curr_dir()?
            .join("build")
            .join("linux")
            .join("x64")
            .join("debug")
            .join("bundle");
        let remote_flutter_dir = exec_dir
            .join("build")
            .join("linux")
            .join("x64")
            .join("debug")
            .join("bundle");

        if let Some(remote) = remote {
            info!("Syncing build to remote...");

            utils::command::CommandUtils::sync_dir_to_remote(
                remote,
                local_flutter_dir.to_string_lossy().as_ref(),
                remote_flutter_dir.to_string_lossy().as_ref(),
            )?;
        }

        Ok(())
    }

    fn sync_tests(&self, remote: Option<&RemoteInfo>, exec_dir: &Path) -> EmptyResult {
        let local_flutter_dir = utils::dir::DirUtils::curr_dir()?.join("integration_test");
        let remote_flutter_dir = exec_dir.join("integration_test");

        if let Some(remote) = remote {
            info!("Syncing integration tests to remote...");

            utils::command::CommandUtils::sync_dir_to_remote(
                remote,
                local_flutter_dir.to_string_lossy().as_ref(),
                remote_flutter_dir.to_string_lossy().as_ref(),
            )?;
        }

        Ok(())
    }

    fn sync_driver(&self, remote: Option<&RemoteInfo>, exec_dir: &Path) -> EmptyResult {
        let local_flutter_dir = utils::dir::DirUtils::curr_dir()?.join("test_driver");
        let remote_flutter_dir = exec_dir.join("test_driver");

        if let Some(remote) = remote {
            info!("Syncing test_driver to remote...");

            utils::command::CommandUtils::sync_dir_to_remote(
                remote,
                local_flutter_dir.to_string_lossy().as_ref(),
                remote_flutter_dir.to_string_lossy().as_ref(),
            )?;
        }

        Ok(())
    }

    fn sync_linux(&self, remote: Option<&RemoteInfo>, exec_dir: &Path) -> EmptyResult {
        let local_flutter_dir = utils::dir::DirUtils::curr_dir()?.join("linux");
        let remote_flutter_dir = exec_dir.join("linux");

        if let Some(remote) = remote {
            info!("Syncing linux to remote...");

            utils::command::CommandUtils::sync_dir_to_remote(
                remote,
                local_flutter_dir.to_string_lossy().as_ref(),
                remote_flutter_dir.to_string_lossy().as_ref(),
            )?;
        }

        Ok(())
    }

    fn sync_pubspec(&self, remote: Option<&RemoteInfo>, exec_dir: &Path) -> EmptyResult {
        let local_pubspec_file = utils::dir::DirUtils::curr_dir()?.join("pubspec.yaml");
        let remote_pubspec_file = exec_dir.join("pubspec.yaml");

        if let Some(remote) = remote {
            info!("Syncing pubspec.yaml to remote...");

            utils::command::CommandUtils::copy_file_to_remote(
                remote,
                local_pubspec_file.to_string_lossy().as_ref(),
                &remote_pubspec_file,
            )?;
        }

        Ok(())
    }

    fn execute_local(&self, exec_dir: &PathBuf, features: &[FeatureTest]) -> EmptyResult {
        let child = self.spawn_flutter_command(exec_dir)?;
        self.process_output(child, features)
    }

    fn execute_remote(
        &self,
        remote: &RemoteInfo,
        exec_dir: &Path,
        features: &[FeatureTest],
    ) -> EmptyResult {
        info!("Executing tests remotely via SSH...\n");

        let binary = "build/linux/x64/debug/bundle/sample_app";
        let binary_arg = format!("--use-application-binary={binary}");
        let args = format!(
            "--driver=test_driver/integration_test.dart --target=integration_test/generated/all_tests.dart {binary_arg} --no-headless"
        );

        let cmd = format!(
            "cd {} && DISPLAY=:0 flutter drive {}",
            exec_dir.display(),
            args
        );

        let output = remote.exec_remote_stream(&cmd)?;
        self.process_remote_output(output, features)
    }

    fn spawn_flutter_command(&self, exec_dir: &PathBuf) -> ResultWithError<Child> {
        let binary = "build/linux/x64/debug/bundle/sample_app";
        let mut command = Command::new("flutter");
        command
            .current_dir(exec_dir)
            .args([
                "drive",
                "--driver=test_driver/integration_test.dart",
                "--target=integration_test/generated/all_tests.dart",
                &format!("--use-application-binary={binary}"),
                "--no-headless",
            ])
            .env("DISPLAY", ":0") // Ensure DISPLAY is set for Linux GUI apps
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit());

        Ok(command.spawn()?)
    }

    fn process_output(&self, mut child: Child, features: &[FeatureTest]) -> EmptyResult {
        let stdout = child.stdout.take().unwrap();
        let reader = BufReader::new(stdout);

        let res = self.process_lines(reader.lines(), features);
        let output = child.wait_with_output()?;
        let status = output.status;
        if res.is_ok() && !status.success() {
            error!(
                "❌ Error when running tests, status:{}, output:{}, error:{}",
                status,
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            return Err("Error during tests".into());
        }
        res
    }

    fn process_remote_output<I: Iterator<Item = String>>(
        &self,
        lines: I,
        features: &[FeatureTest],
    ) -> EmptyResult {
        self.process_lines(lines.map(Ok), features)
    }

    fn process_lines(
        &self,
        lines: impl Iterator<Item = std::io::Result<String>>,
        features: &[FeatureTest],
    ) -> EmptyResult {
        let mut passed = 0;
        let mut failed = 0;

        let mut current_test: Option<String> = None;
        let mut test_spinner: Option<indicatif::ProgressBar> = None;

        // track failure logs per test
        let mut current_test_output = String::new();
        let mut collecting_output = false;

        for line in lines {
            let mut line = line?;
            line = line.trim().to_string();

            if line.is_empty()
                || line.contains("Some tests failed")
                || line.contains("All tests passed")
                || line.contains("VMServiceFlutterDriver")
            {
                continue;
            }

            // strip "flutter: " prefix
            let line = line.strip_prefix("flutter:").unwrap_or(&line).trim();

            // detect test progress lines, e.g. "00:06 +0: TestName" or "+0 -1:"
            if let Some((_, rest)) = line.split_once(' ') {
                if rest.contains("[E]") {
                    continue;
                }

                if rest.contains('+') && rest.contains(':') {
                    // Example: "+0 -1: TestName"
                    // Extract counters and test name
                    let mut parts = rest.splitn(2, ':');
                    let counters = parts.next().unwrap_or_default().trim();
                    let test_name = parts.next().unwrap_or_default().trim();

                    let counts: Vec<i16> = counters
                        .split_whitespace()
                        .map(|s| s.parse::<i16>().unwrap_or(0).abs())
                        .collect();
                    let curr_passed = counts.first().cloned().unwrap_or(0);
                    let curr_failed = counts.get(1).cloned().unwrap_or(0);

                    // finish spinner if previous test was running
                    if let Some(ts) = test_spinner.take() {
                        ts.finish_and_clear();
                    }

                    if curr_failed != failed {
                        failed += 1;
                        if let Some(test_name) = current_test.as_ref() {
                            error!("❌ Failed: {}", test_name);

                            if let Some(desc) =
                                self.find_feature_test_description(features, test_name)
                            {
                                error!("Test Description: {}", desc);
                            }
                        }
                        error!("Test output...");
                        for line in current_test_output.lines() {
                            error!("    {}", line);
                        }
                        error!("End of test output\n");
                    } else if curr_passed != passed {
                        passed += 1;
                        if let Some(test_name) = current_test.as_ref() {
                            info!("✅ Succeeded: {}", test_name);
                        }
                    }

                    if test_name.starts_with("(") {
                        // skip (setUpAll) and similar
                        current_test = None;
                        continue;
                    }

                    current_test = Some(test_name.to_string());
                    test_spinner = Some(utils::command::CommandUtils::display_loader(format!(
                        "Running: {}",
                        test_name
                    )));
                    current_test_output.clear();
                    collecting_output = true;

                    continue;
                }
            }

            if collecting_output && !line.is_empty() {
                current_test_output.push_str(format!("{}\n", line).as_str());
            }
        }

        if let Some(ts) = test_spinner.take() {
            ts.finish_and_clear();
        }

        println!();
        info!("🎉 All tests completed");
        info!(
            "✅ Passed: {passed}  ❌ Failed: {failed}  📋 Total: {}",
            passed + failed
        );

        if failed > 0 {
            return Err("Some tests failed".into());
        }

        Ok(())
    }

    fn find_feature_test_description(
        &self,
        features: &[FeatureTest],
        full_test_name: &str,
    ) -> Option<String> {
        features.iter().find_map(|f| {
            f.tests.iter().find_map(|t| {
                let joined = format!("{} {}", f.name, t.name);
                if full_test_name == joined {
                    Some(t.description.clone())
                } else {
                    None
                }
            })
        })
    }
}
