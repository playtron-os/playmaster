use std::{
    io::{BufRead as _, BufReader},
    path::PathBuf,
    process::{Child, Command, Stdio},
};

use indicatif::ProgressBar;
use serde_json::Value;
use tracing::info;

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
pub struct RunFlutter {}

impl CodeRunTrait for RunFlutter {
    fn get_type(&self) -> ProjectType {
        ProjectType::Flutter
    }

    fn run(&self, ctx: &HookContext, features: &[FeatureTest]) -> EmptyResult {
        let state = ctx.read_state()?;
        let remote = state.remote.as_ref();
        self.prepare_env(remote)?;

        info!(
            "Running Flutter tests with {} feature files",
            features.len()
        );

        let spinner = utils::command::CommandUtils::display_loader("Starting tests...".to_string());

        if let Err(err) = self.execute_tests(&spinner, features) {
            spinner.finish_and_clear();
            return Err(err);
        }

        Ok(())
    }
}

impl RunFlutter {
    pub fn new() -> Self {
        Self {}
    }

    fn prepare_env(&self, remote: Option<&RemoteInfo>) -> EmptyResult {
        if let Some(remote) = remote {
            info!("Preparing remote environment...");

            // Copy current dir to remote
            let curr_dir = utils::dir::DirUtils::curr_dir()?;
            let remote_dir = utils::dir::DirUtils::root_dir(Some(remote))?.join("flutter_app");
            utils::command::CommandUtils::copy_dir_to_remote(remote, &curr_dir, &remote_dir)?;
        }

        Ok(())
    }

    fn execute_tests(&self, spinner: &ProgressBar, features: &[FeatureTest]) -> EmptyResult {
        let exec_dir = utils::dir::DirUtils::curr_dir()?;
        let mut child = self.spawn_flutter_command(exec_dir)?;

        let stdout = child.stdout.take().unwrap();
        let reader = BufReader::new(stdout);

        let mut passed = 0;
        let mut failed = 0;
        let mut id_to_info = std::collections::HashMap::new();
        let unknown_str = "unknown".to_string();

        let mut test_spinner = None;
        let mut print_lines = vec![];

        for line in reader.lines() {
            let line = line?;
            let Ok(json) = serde_json::from_str::<Value>(&line) else {
                continue;
            };

            if json["type"] == "testStart" {
                let Some(test) = json["test"].as_object() else {
                    continue;
                };

                let Some(root_url) = test.get("root_url") else {
                    continue;
                };

                spinner.finish_and_clear();

                let name = test["name"].as_str().unwrap_or(&unknown_str);
                let id = test["id"].as_i64().unwrap_or(-1);

                println!();
                print_lines.clear();
                test_spinner = Some(utils::command::CommandUtils::display_loader(format!(
                    "Running: {}",
                    name
                )));

                id_to_info.insert(id, (name.to_string(), root_url.to_string()));
            } else if json["hidden"].as_bool() == Some(false) && json["type"] == "testDone" {
                spinner.finish_and_clear();

                if let Some(ts) = test_spinner.take() {
                    ts.finish_and_clear();
                }

                let success = json["result"] == "success";
                let id = json["testID"].as_i64().unwrap_or(-1);
                let (name, root_url) = id_to_info
                    .remove(&id)
                    .unwrap_or((unknown_str.clone(), unknown_str.clone()));

                if success {
                    passed += 1;
                    info!("✅ Passed: {}", name);
                } else {
                    let test_desc = self
                        .find_feature_test_description(features, &name)
                        .unwrap_or("".to_string());

                    failed += 1;
                    info!("❌ Failed: {name}");
                    info!("   File: {root_url}");
                    info!("   Test Description: {test_desc}");
                    for line in &print_lines {
                        info!("{}", line);
                    }
                }
            } else if json["type"] == "print" {
                let message = json["message"].as_str().unwrap_or("");

                if message.trim().is_empty() {
                    continue;
                }

                spinner.finish_and_clear();
                if let Some(ts) = test_spinner.as_ref() {
                    ts.finish_and_clear();
                }

                print_lines.push(format!("   {}", message));
            }
        }

        spinner.finish_and_clear();
        println!();
        info!("🎉 All tests completed");

        let status = child.wait()?;
        info!("✅ Passed: {passed}  ❌ Failed: {failed}");
        if !status.success() {
            info!("Some tests failed");
        }

        Ok(())
    }

    fn spawn_flutter_command(&self, exec_dir: PathBuf) -> ResultWithError<Child> {
        Ok(Command::new("flutter")
            .current_dir(exec_dir)
            .args([
                "test",
                "integration_test/generated",
                "--machine",
                "-d",
                "linux",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()?)
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
