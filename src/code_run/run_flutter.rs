use std::{
    collections::HashSet,
    fs,
    io::{BufRead as _, BufReader},
    path::{Path, PathBuf},
    pin::Pin,
    process::{Child, Command, Stdio},
    str::FromStr,
    time::Duration,
};

use indicatif::ProgressBar;
use regex::Regex;
use scopeguard::defer;
use serde_yaml::{Mapping, Value};
use tempfile::NamedTempFile;
use tracing::{debug, error, info};

use crate::{
    code_run::run_iface::CodeRunTrait,
    gmail::client::GmailClient,
    hooks::iface::HookContext,
    models::{
        app_state::{AppState, RemoteInfo},
        config::ProjectType,
        feature_test::{FeatureTest, UserInputGmail},
    },
    utils::{
        self,
        command::CommandUtils,
        dbus::DbusUtils,
        errors::{EmptyResult, ResultTrait, ResultWithError},
        flutter::FlutterUtils,
        os::OsUtils,
    },
};

#[allow(dead_code)]
pub struct RunFlutter;

impl CodeRunTrait for RunFlutter {
    fn get_type(&self) -> ProjectType {
        ProjectType::Flutter
    }

    fn run<'a>(
        &'a self,
        ctx: &'a HookContext<'a, AppState>,
        features: &'a [FeatureTest],
    ) -> Pin<Box<dyn Future<Output = EmptyResult> + Send + 'a>> {
        Box::pin(async move {
            let remote = ctx.get_remote_info()?;
            let remote = remote.as_ref();
            let root_dir = ctx.get_root_dir()?;

            let exec_dir = if remote.is_some() {
                PathBuf::from_str(&root_dir)?.join("flutter_app")
            } else {
                utils::dir::DirUtils::curr_dir()?
            };

            self.prepare_env(remote, &exec_dir, &root_dir)?;

            if let Some(remote) = remote {
                info!("Running Flutter tests remotely");
                self.execute_remote(ctx, remote, &exec_dir, &root_dir, features)
                    .await
            } else {
                info!("Running Flutter tests locally\n");
                self.execute_local(ctx, &exec_dir, &root_dir, features)
                    .await
            }
        })
    }
}

impl RunFlutter {
    pub fn new() -> Self {
        Self {}
    }

    fn prepare_env(
        &self,
        remote: Option<&RemoteInfo>,
        exec_dir: &Path,
        root_dir: &str,
    ) -> EmptyResult {
        self.build()?;

        if let Some(remote) = remote {
            self.sync_build(remote, root_dir, exec_dir)?;
            self.sync_tests(remote, root_dir, exec_dir)?;
            self.sync_driver(remote, root_dir, exec_dir)?;
            self.sync_linux(remote, root_dir, exec_dir)?;
            self.sync_pubspec(remote, root_dir, exec_dir)?;
        }

        Ok(())
    }

    fn build(&self) -> EmptyResult {
        info!("Building Flutter app...");

        let mut command = Command::new("bash");
        command
            .current_dir(utils::dir::DirUtils::curr_dir()?)
            .arg("-c")
            .arg("flutter pub get && flutter build linux --debug --target=integration_test/generated/all_tests.dart")
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit());
        let status = command.status()?;

        if !status.success() {
            return Err("Flutter build failed".into());
        }

        Ok(())
    }

    fn sync_build(&self, remote: &RemoteInfo, root_dir: &str, exec_dir: &Path) -> EmptyResult {
        info!("Syncing build to remote...");

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

        utils::command::CommandUtils::sync_dir_to_remote(
            remote,
            root_dir,
            local_flutter_dir.to_string_lossy().as_ref(),
            remote_flutter_dir.to_string_lossy().as_ref(),
        )?;

        Ok(())
    }

    fn sync_tests(&self, remote: &RemoteInfo, root_dir: &str, exec_dir: &Path) -> EmptyResult {
        info!("Syncing integration tests to remote...");

        let local_flutter_dir = utils::dir::DirUtils::curr_dir()?.join("integration_test");
        let remote_flutter_dir = exec_dir.join("integration_test");

        utils::command::CommandUtils::sync_dir_to_remote(
            remote,
            root_dir,
            local_flutter_dir.to_string_lossy().as_ref(),
            remote_flutter_dir.to_string_lossy().as_ref(),
        )?;

        Ok(())
    }

    fn sync_driver(&self, remote: &RemoteInfo, root_dir: &str, exec_dir: &Path) -> EmptyResult {
        info!("Syncing test_driver to remote...");

        let local_flutter_dir = utils::dir::DirUtils::curr_dir()?.join("test_driver");
        let remote_flutter_dir = exec_dir.join("test_driver");

        utils::command::CommandUtils::sync_dir_to_remote(
            remote,
            root_dir,
            local_flutter_dir.to_string_lossy().as_ref(),
            remote_flutter_dir.to_string_lossy().as_ref(),
        )?;

        Ok(())
    }

    fn sync_linux(&self, remote: &RemoteInfo, root_dir: &str, exec_dir: &Path) -> EmptyResult {
        info!("Syncing linux to remote...");

        let local_flutter_dir = utils::dir::DirUtils::curr_dir()?.join("linux");
        let remote_flutter_dir = exec_dir.join("linux");

        utils::command::CommandUtils::sync_dir_to_remote(
            remote,
            root_dir,
            local_flutter_dir.to_string_lossy().as_ref(),
            remote_flutter_dir.to_string_lossy().as_ref(),
        )?;

        Ok(())
    }

    fn cleaned_pubspec(&self, content: &str) -> ResultWithError<String> {
        // Parse pubspec.yaml into a YAML value tree
        let mut root: Value = serde_yaml::from_str(content)?;
        let map = root
            .as_mapping_mut()
            .ok_or("pubspec.yaml root is not a mapping")?;

        // Build: dependencies: { flutter: { sdk: flutter } }
        let mut deps = Mapping::new();
        let mut flutter = Mapping::new();
        flutter.insert(Value::from("sdk"), Value::from("flutter"));
        deps.insert(Value::from("flutter"), Value::Mapping(flutter));
        map.insert(Value::from("dependencies"), Value::Mapping(deps));

        // Build: dev_dependencies: { flutter_test: { sdk: flutter }, integration_test: { sdk: flutter } }
        let mut dev_deps = Mapping::new();

        let mut flutter_test = Mapping::new();
        flutter_test.insert(Value::from("sdk"), Value::from("flutter"));
        dev_deps.insert(Value::from("flutter_test"), Value::Mapping(flutter_test));

        let mut integration_test = Mapping::new();
        integration_test.insert(Value::from("sdk"), Value::from("flutter"));
        dev_deps.insert(
            Value::from("integration_test"),
            Value::Mapping(integration_test),
        );

        map.insert(Value::from("dev_dependencies"), Value::Mapping(dev_deps));

        map.remove(Value::from("dependency_overrides"));

        let out = serde_yaml::to_string(&root)?;
        Ok(out)
    }

    fn sync_pubspec(&self, remote: &RemoteInfo, root_dir: &str, exec_dir: &Path) -> EmptyResult {
        info!("Syncing cleaned pubspec.yaml to remote…");

        let local_pubspec_file = utils::dir::DirUtils::curr_dir()?.join("pubspec.yaml");
        let remote_pubspec_file = exec_dir.join("pubspec.yaml");

        // Read local pubspec.yaml
        let original = fs::read_to_string(&local_pubspec_file)?;

        // Sanitize dependencies/dev_dependencies
        let cleaned = self.cleaned_pubspec(&original)?;

        // Write to temp file
        let temp = NamedTempFile::new()?;
        fs::write(temp.path(), &cleaned)?;

        // Send to remote (or write locally as fallback)
        utils::command::CommandUtils::copy_file_to_remote(
            remote,
            root_dir,
            temp.path().to_string_lossy().as_ref(),
            &remote_pubspec_file,
        )?;

        Ok(())
    }

    async fn execute_local(
        &self,
        ctx: &HookContext<'_, AppState>,
        exec_dir: &PathBuf,
        root_dir: &str,
        features: &[FeatureTest],
    ) -> EmptyResult {
        let child = self.spawn_flutter_command(exec_dir, root_dir)?;
        self.process_output(ctx, child, features).await
    }

    async fn execute_remote(
        &self,
        ctx: &HookContext<'_, AppState>,
        remote: &RemoteInfo,
        exec_dir: &Path,
        root_dir: &str,
        features: &[FeatureTest],
    ) -> EmptyResult {
        info!("Executing tests remotely via SSH...\n");

        let cmd = format!(
            "cd {} && {}",
            exec_dir.display(),
            self.get_flutter_drive_command_str(root_dir)?,
        );
        info!("Remote command: {}\n", cmd);

        let output = remote.exec_remote_stream(&cmd)?;
        self.process_remote_output(ctx, output, features).await
    }

    fn spawn_flutter_command(&self, exec_dir: &PathBuf, root_dir: &str) -> ResultWithError<Child> {
        let mut command = Command::new("sh");
        command
            .current_dir(exec_dir)
            .args(["-c", &self.get_flutter_drive_command_str(root_dir)?])
            .env("DISPLAY", OsUtils::get_display())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        info!("Spawning local command: {:?}\n", command);

        Ok(command.spawn()?)
    }

    fn get_flutter_drive_command_str(&self, root_dir: &str) -> ResultWithError<String> {
        let binary_name = FlutterUtils::get_name()?;

        let binary = format!("build/linux/x64/debug/bundle/{binary_name}");
        let binary_arg = format!("--use-application-binary={binary}");
        let args = format!(
            "--driver=test_driver/integration_test.dart --target=integration_test/generated/all_tests.dart {binary_arg} --no-headless -d linux"
        );

        CommandUtils::with_env_source(root_dir, &format!("flutter drive {args}"))
    }

    async fn process_output(
        &self,
        ctx: &HookContext<'_, AppState>,
        mut child: Child,
        features: &[FeatureTest],
    ) -> EmptyResult {
        let stdout = child.stdout.take().unwrap();
        let reader = BufReader::new(stdout);

        let res = self.process_lines(ctx, reader.lines(), features).await;
        let output = child
            .wait_with_output()
            .auto_err("Failed to wait for child process when running flutter tests")?;
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

    async fn process_remote_output<I: Iterator<Item = String>>(
        &self,
        ctx: &HookContext<'_, AppState>,
        lines: I,
        features: &[FeatureTest],
    ) -> EmptyResult {
        self.process_lines(ctx, lines.map(Ok), features).await
    }

    async fn process_lines(
        &self,
        ctx: &HookContext<'_, AppState>,
        lines: impl Iterator<Item = std::io::Result<String>>,
        features: &[FeatureTest],
    ) -> EmptyResult {
        let start_time = chrono::Utc::now();
        let start_timestamp = start_time.timestamp();
        let mut passed = 0;
        let mut failed = 0;

        let mut current_test: Option<String> = None;
        let mut test_spinner: Option<indicatif::ProgressBar> = None;

        // track failure logs per test
        let mut full_test_output = String::new();
        let mut current_test_output = String::new();
        let mut collecting_output = false;
        let mut prev_test_names = HashSet::new();

        for line in lines {
            let mut line = line?;
            line = line.trim().to_string();
            full_test_output.push_str(format!("{}\n", line).as_str());

            if line.is_empty()
                || line.contains("Some tests failed")
                || line.contains("All tests passed")
                || line.contains("VMServiceFlutterDriver")
            {
                continue;
            }

            if let Some(input_name) = DbusUtils::identify_continue_request(&line)
                && let Some(curr_test) = current_test.as_ref()
            {
                if let Err(err) = self
                    .process_user_input(
                        ctx,
                        features,
                        curr_test,
                        &input_name,
                        &mut test_spinner,
                        start_timestamp,
                    )
                    .await
                {
                    error!(
                        "Error processing user input for test '{}', input '{}', err:{:?}",
                        curr_test, input_name, err
                    );
                }
                continue;
            }

            // strip "flutter: " prefix
            let line = line.strip_prefix("flutter:").unwrap_or(&line).trim();

            // detect test progress lines, e.g. "00:06 +0: TestName" or "+0 -1:"
            if let Some((_, rest)) = line.split_once(' ') {
                if rest.contains("[E]") {
                    continue;
                }

                if rest.starts_with('+') && rest.contains(':') {
                    // Example: "+0 -1: TestName"
                    // Extract counters and test name
                    let mut parts = rest.splitn(2, ':');
                    let counters = parts.next().unwrap_or_default().trim();
                    let test_name = parts.next().unwrap_or_default().trim();

                    if prev_test_names.contains(test_name) {
                        continue;
                    }
                    prev_test_names.insert(test_name.to_owned());

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

                    // Failed
                    if curr_failed != failed
                        && let Some(test_name) = current_test.as_ref()
                    {
                        failed += 1;
                        self.handle_test_failed(
                            ctx,
                            test_name,
                            self.find_feature_test_description(features, test_name),
                            &current_test_output,
                        )?;
                    }

                    // Passed
                    if curr_passed != passed
                        && let Some(test_name) = current_test.as_ref()
                    {
                        passed += 1;
                        self.handle_test_passed(ctx, test_name)?;
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

        let total = passed + failed;
        ctx.set_results_total(total)?;

        let end_time = chrono::Utc::now();
        ctx.set_results_time(start_time, end_time)?;

        ctx.set_results_full_log(full_test_output)?;

        println!();
        info!("🎉 All tests completed");
        info!("✅ Passed: {passed}  ❌ Failed: {failed}  📋 Total: {total}");

        if failed > 0 {
            return Err("Some tests failed".into());
        }

        Ok(())
    }

    async fn process_user_input(
        &self,
        ctx: &HookContext<'_, AppState>,
        features: &[FeatureTest],
        curr_test: &str,
        input_name: &str,
        test_spinner: &mut Option<ProgressBar>,
        start_timestamp: i64,
    ) -> EmptyResult {
        defer! {
            if let Some(spinner) = test_spinner.as_ref() {
                spinner.enable_steady_tick(Duration::from_millis(80));
            }
        }

        if let Some(spinner) = test_spinner.as_ref() {
            spinner.disable_steady_tick();
        }

        let user_input = if let Some(gmail_client) = self.get_gmail_client(ctx)
            && let Some(gmail_config) =
                self.find_feature_test_gmail_config(features, curr_test, input_name)
        {
            info!("Fetching user input via Gmail for input: {}", input_name);

            let email_from = ctx.vars.replace_var(&gmail_config.from, None);
            let email_subject_contains = ctx.vars.replace_var(&gmail_config.subject_contains, None);

            let regex_pattern = match &gmail_config.regex {
                crate::models::feature_test::UserInputGmailRegexType::Custom { pattern } => {
                    pattern.clone()
                }
                crate::models::feature_test::UserInputGmailRegexType::Mfa => {
                    r"\b(\d{6})\b".to_string()
                }
            };

            let re = Regex::new(&regex_pattern).auto_err("Error compiling user input regex")?;

            gmail_client
                .fetch_latest_email_matching_regex(
                    &email_from,
                    &email_subject_contains,
                    &re,
                    Some(start_timestamp),
                    30,
                    3,
                )
                .await
                .unwrap_or_default()
        } else {
            debug!(
                "Prompting for user input via DBus for input: {}",
                input_name
            );

            OsUtils::ask(&format!("User input requested for {}:", input_name))
                .map_err(|err| {
                    error!("Error during prompting for user input, err:{err:?}");
                    err
                })
                .unwrap_or_default()
        };

        debug!("User input: {}", user_input);
        let dbus_cmd = DbusUtils::dbus_method_continue_cmd(&user_input);

        let remote = ctx.get_remote_info()?;
        let root_dir = ctx.get_root_dir()?;

        CommandUtils::run_command_str(&dbus_cmd, remote.as_ref(), &root_dir)?;
        debug!("Sent DBus continue command");

        Ok(())
    }

    fn get_gmail_client(&self, ctx: &HookContext<'_, AppState>) -> Option<GmailClient> {
        if !ctx.config.gmail.enabled {
            return None;
        }

        debug!("Creating Gmail client for user input retrieval");

        Some(GmailClient::new(
            ctx.config
                .gmail
                .credentials
                .s3
                .as_ref()
                .map(|s| s.bucket.clone()),
            ctx.config
                .gmail
                .credentials
                .s3
                .as_ref()
                .map(|s| s.key_prefix.clone()),
        ))
    }

    fn handle_test_passed(&self, ctx: &HookContext<'_, AppState>, test_name: &str) -> EmptyResult {
        info!("✅ Succeeded: {}", test_name);
        ctx.increment_results_passed()?;
        Ok(())
    }

    fn handle_test_failed(
        &self,
        ctx: &HookContext<'_, AppState>,
        test_name: &str,
        description: Option<String>,
        current_test_output: &str,
    ) -> EmptyResult {
        info!("❌ Failed: {}", test_name);
        if let Some(desc) = description {
            error!("Test Description: {}", desc);
        }

        error!("Test output...");
        for line in current_test_output.lines() {
            error!("    {}", CommandUtils::unescape_ansi(line.to_owned()));
        }
        error!("End of test output\n");

        ctx.increment_results_failed()?;
        Ok(())
    }

    fn find_feature_test_description(
        &self,
        features: &[FeatureTest],
        full_test_name: &str,
    ) -> Option<String> {
        features.iter().find_map(|f| {
            f.tests.iter().find_map(|t| {
                let joined = format!("{} - {}", f.name, t.name);
                if full_test_name == joined {
                    Some(t.description.clone())
                } else {
                    None
                }
            })
        })
    }

    fn find_feature_test_gmail_config(
        &self,
        features: &[FeatureTest],
        full_test_name: &str,
        user_input_name: &str,
    ) -> Option<UserInputGmail> {
        features
            .iter()
            .find_map(|f| {
                debug!("Searching in feature test: {}", f.name);
                f.tests.iter().find_map(|t| {
                    let joined = format!("{} - {}", f.name, t.name);
                    debug!("Comparing with test name: {}", joined);
                    if full_test_name == joined {
                        t.steps.iter().find_map(|s| {
                            if let crate::models::feature_test::Step::UserInput { user_input } = s {
                                if user_input.name == user_input_name {
                                    debug!("Found user input: {}", user_input.name);
                                    Some(user_input.gmail.clone())
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        })
                    } else {
                        None
                    }
                })
            })
            .flatten()
    }
}
