use std::{
    io::{BufRead as _, BufReader},
    process::{Command, Stdio},
};

use indicatif::ProgressBar;
use serde_json::Value;
use tracing::info;

use crate::{
    code_run::run_iface::CodeRunTrait,
    models::{
        args::AppArgs,
        config::{Config, ProjectType},
        feature_test::FeatureTest,
    },
    utils::{self, errors::EmptyResult},
};

#[allow(dead_code)]
pub struct RunFlutter {
    args: AppArgs,
    config: Config,
}

impl CodeRunTrait for RunFlutter {
    fn get_type(&self) -> ProjectType {
        ProjectType::Flutter
    }

    fn run(&self, features: &[FeatureTest]) -> EmptyResult {
        info!(
            "Running Flutter tests with {} feature files",
            features.len()
        );

        let spinner = utils::command::CommandUtils::display_loader("Starting tests...".to_string());

        if let Err(err) = self.execute_tests(&spinner) {
            spinner.finish_and_clear();
            return Err(err);
        }

        Ok(())
    }
}

impl RunFlutter {
    pub fn new(args: AppArgs, config: Config) -> Self {
        Self { args, config }
    }

    fn execute_tests(&self, spinner: &ProgressBar) -> EmptyResult {
        let exec_dir = utils::dir::DirUtils::exec_dir()?;

        let mut child = Command::new("flutter")
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
            .spawn()?;

        let stdout = child.stdout.take().unwrap();
        let reader = BufReader::new(stdout);

        let mut passed = 0;
        let mut failed = 0;
        let mut id_to_name = std::collections::HashMap::new();
        let unknown_str = "unknown".to_string();

        let mut test_spinner = None;

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

                info!("   Test URL: {}", root_url);

                test_spinner = Some(utils::command::CommandUtils::display_loader(format!(
                    "Running: {}",
                    name
                )));

                id_to_name.insert(id, name.to_string());
            } else if json["hidden"].as_bool() == Some(false) && json["type"] == "testDone" {
                spinner.finish_and_clear();

                if let Some(ts) = test_spinner.take() {
                    ts.finish_and_clear();
                }

                let success = json["result"] == "success";
                let id = json["testID"].as_i64().unwrap_or(-1);
                let name = id_to_name.get(&id).unwrap_or(&unknown_str);

                if success {
                    passed += 1;
                    info!("✅ Passed: {}", name);
                } else {
                    failed += 1;
                    info!("❌ Failed: {}", name);
                }
            }
        }

        spinner.finish_and_clear();
        info!("All tests completed");

        let status = child.wait()?;
        info!("✅ Passed: {passed}  ❌ Failed: {failed}");
        if !status.success() {
            info!("Some tests failed");
        }

        Ok(())
    }
}
