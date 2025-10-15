use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::{
    code_gen::gen_iface::CodeGenTrait,
    models::{
        args::AppArgs,
        config::{Config, ProjectType},
        feature_test::{self, FeatureTest, Step, WaitFor},
    },
    utils::{
        dir::DirUtils,
        errors::{EmptyResult, ResultWithError},
    },
};

mod entrypoint;
mod helper;
mod utils;
mod vars;

#[allow(dead_code)]
pub struct GenFlutter {
    args: AppArgs,
    config: Config,
    out_dir: PathBuf,
}

impl CodeGenTrait for GenFlutter {
    fn get_type(&self) -> ProjectType {
        ProjectType::Flutter
    }

    fn run(&self, features: &[FeatureTest]) -> EmptyResult {
        self.generate_helpers()?;

        for feature in features {
            feature.generate_dart(&self.out_dir)?;
        }

        self.generate_vars()?;
        self.generate_all_entrypoint(features)?;
        self.run_dart_format();
        self.run_dart_fix();

        Ok(())
    }
}

impl GenFlutter {
    pub fn from_exec_dir(args: AppArgs, config: Config) -> ResultWithError<Self> {
        let cwd = DirUtils::curr_dir()?;
        let out_dir = cwd.join("integration_test/generated");
        fs::create_dir_all(&out_dir)?;

        Ok(Self {
            args,
            config,
            out_dir,
        })
    }
}

impl FeatureTest {
    pub fn generate_dart(&self, out_dir: &Path) -> EmptyResult {
        let normalized_name = self.name.to_lowercase().replace(' ', "_") + "_test";
        let file_name = normalized_name.clone() + ".dart";
        let file_path = out_dir.join(&file_name);
        let mut out = String::new();

        // Header
        out.push_str("// GENERATED FILE - DO NOT EDIT\n");
        out.push_str("import 'package:flutter_test/flutter_test.dart';\n");
        out.push_str("import 'package:flutter/material.dart';\n");
        out.push_str("import 'package:integration_test/integration_test.dart';\n");
        out.push_str("import 'package:sample_app/main.dart' as app;\n\n");
        out.push_str("import 'helpers.dart';\n");
        out.push_str("import 'vars.dart';\n\n");
        out.push_str("void main() {\n");
        out.push_str("  IntegrationTestWidgetsFlutterBinding.ensureInitialized();\n\n");

        // Vars
        if !self.vars.is_empty() {
            let mut vars: Vec<_> = self.vars.iter().collect();
            vars.sort_by_key(|(key, _)| *key);
            for (key, value) in vars {
                out.push_str(&format!("  const {} = '{}';\n", key, value));
            }
            out.push_str("\n\n");
        }

        out.push_str(&format!("  group('{}', () {{\n", self.name));

        // Test cases
        for test in &self.tests {
            out.push_str(&format!(
                "    testWidgets('{}', (tester) async {{\n",
                test.name
            ));
            out.push_str("      await tester.setTestResolution();\n\n");
            out.push_str("      app.main();\n");
            out.push_str("      await tester.pumpAndSettle();\n\n");

            for step in &test.steps {
                out.push_str(&step.to_dart_code(&normalized_name));
            }

            out.push_str("    });\n\n");
        }

        out.push_str("  });\n}\n");

        fs::write(file_path, out)?;
        Ok(())
    }
}

impl Step {
    pub fn to_dart_code(&self, file_name: &str) -> String {
        match self {
            Step::WaitFor { wait_for } => match wait_for {
                WaitFor::Text { text } => format!(
                    "      await tester.pumpUntilFound(find.text('{}'));\n",
                    text
                ),
                WaitFor::Delay { delay } => format!(
                    "      await tester.pump(Duration(milliseconds: {}));\n",
                    delay
                ),
                WaitFor::Progress { progress } => match progress {
                    feature_test::ProgressWidgetType::Linear => "      await tester.pumpUntilProgressCompleted(find.byType(LinearProgressIndicator));\n".to_string(),
                    feature_test::ProgressWidgetType::Radial => "      await tester.pumpUntilProgressCompleted(find.byType(CircularProgressIndicator));\n".to_string(),
                },
            },
            Step::Tap { tap } => match &tap.target {
                feature_test::Target::Placeholder { placeholder } => format!(
                    "      await tester.tap(find.byPlaceholder('{}'));\n",
                    placeholder
                ),
                feature_test::Target::Text { text } => {
                    format!("      await tester.tap(find.text('{}'));\n", text)
                }
            },
            Step::Type { r#type } => match &r#type.by {
                feature_test::Target::Placeholder { placeholder } => format!(
                    "      await tester.enterText(find.byPlaceholder('{}'), '{}');\n",
                    placeholder, r#type.value
                ),
                feature_test::Target::Text { text } => format!(
                    "      await tester.enterText(find.text('{}'), '{}');\n",
                    text, r#type.value
                ),
            },
            Step::Match { r#match } => match &r#match.target {
                feature_test::MatchTarget::Text { text } => {
                    format!("      expect(find.text('{}'), findsOneWidget);\n", text)
                }
                feature_test::MatchTarget::Screenshot { screenshot } => {
                    format!("      await tester.compareScreenshot('{}', '{}');\n", file_name, screenshot)
                }
            },
        }
    }
}
