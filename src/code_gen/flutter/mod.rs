use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use crate::{
    code_gen::gen_iface::CodeGenTrait,
    hooks::iface::HookContext,
    models::{
        args::AppArgs,
        config::{Config, ProjectType},
        feature_test::{self, FeatureTest, SimpleStep, Step, WaitFor},
        gen_state::GenState,
    },
    utils::{
        dir::DirUtils,
        errors::{EmptyResult, ResultWithError},
    },
};

mod entrypoint;
mod helper;
mod test_driver;
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

    fn run(&self, ctx: &HookContext<'_, GenState>) -> EmptyResult {
        self.generate_helpers()?;

        {
            let features = &ctx.read_state()?.features;
            for feature in features {
                feature.generate_dart(ctx, &self.out_dir)?;
            }
            self.generate_all_entrypoint(features)?;
        }

        self.generate_vars(ctx)?;
        self.generate_test_driver()?;
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
    pub fn generate_dart(&self, ctx: &HookContext<'_, GenState>, out_dir: &Path) -> EmptyResult {
        let normalized_name = self.name.to_lowercase().replace(' ', "_") + "_test";
        let file_name = normalized_name.clone() + ".dart";
        let file_path = out_dir.join(&file_name);
        let mut out = String::new();
        let has_before_each = self.before_each.is_some();

        // Header
        out.push_str("// GENERATED FILE - DO NOT EDIT\n");
        out.push_str("import 'dart:ui';\n");
        out.push_str("import 'package:flutter_test/flutter_test.dart';\n");
        out.push_str("import 'package:flutter/material.dart';\n");
        out.push_str("import 'package:integration_test/integration_test.dart';\n");
        out.push_str("import 'helpers.dart';\n");
        out.push_str("import 'vars.dart';\n\n");
        out.push_str("void main() {\n");
        out.push_str("  IntegrationTestWidgetsFlutterBinding.ensureInitialized();\n\n");

        // Before each
        if let Some(before_each) = self.before_each.as_ref()
            && !before_each.steps.is_empty()
        {
            let mut steps = "".to_owned();
            for step in &before_each.steps {
                steps.push_str(&step.to_dart_code(ctx, &self.step_definitions, &normalized_name));
            }

            out.push_str(&format!(
                r#"
beforeEach(WidgetTester tester) async {{
    {steps}
}}
        "#
            ));
        }

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
            out.push_str(&format!(
                "      await tester.initializeTest('{}');",
                ctx.vars.replace_var_usage(&test.state),
            ));
            if has_before_each {
                out.push_str("      await beforeEach(tester);\n");
            } else {
                out.push('\n');
            }
            out.push('\n');

            for step in &test.steps {
                out.push_str("      //\n");
                out.push_str(&step.to_dart_code(ctx, &self.step_definitions, &normalized_name));
                out.push('\n');
            }

            out.push_str("    });\n\n");
        }

        out.push_str("  });\n}\n");

        fs::write(file_path, out)?;
        Ok(())
    }
}

impl Step {
    pub fn to_dart_code(
        &self,
        ctx: &HookContext<'_, GenState>,
        step_definitions: &HashMap<String, Vec<Step>>,
        file_name: &str,
    ) -> String {
        match self {
            Step::Use { use_step } => {
                if let Some(steps) = step_definitions.get(use_step) {
                    let mut code = String::new();
                    for step in steps {
                        code.push_str(&step.to_dart_code(ctx, step_definitions, file_name));
                    }
                    code
                } else {
                    format!("      // Step definition '{}' not found.\n", use_step)
                }
            }
            Step::Simple(SimpleStep::Settle) => "      await tester.pumpAndSettle();\n".to_owned(),
            Step::NotFound {
                not_found,
                timeout_millis,
            } => format!(
                "      await tester.waitUntilGone({}, timeout: {});\n",
                Self::find_by(ctx, not_found),
                Self::duration(*timeout_millis, 10000)
            ),
            Step::WaitFor { wait_for } => match wait_for {
                WaitFor::Key {
                    key,
                    timeout_millis,
                    settle,
                } => format!(
                    "      await tester.pumpUntilFound(find.byKey(Key('{}')), timeout: {});\n{}",
                    ctx.vars.replace_var_usage(key),
                    Self::duration(*timeout_millis, 5000),
                    Self::settle(*settle),
                ),
                WaitFor::Text {
                    text,
                    timeout_millis,
                    settle,
                } => format!(
                    "      await tester.pumpUntilFound(find.text('{}'), timeout: {});\n{}",
                    ctx.vars.replace_var_usage(text),
                    Self::duration(*timeout_millis, 5000),
                    Self::settle(*settle),
                ),
                WaitFor::Delay { delay, settle } => format!(
                    "      await tester.pump(Duration(milliseconds: {}));\n{}",
                    delay,
                    Self::settle(*settle),
                ),
                WaitFor::Progress {
                    progress,
                    timeout_millis,
                    settle,
                } => match progress {
                    feature_test::ProgressWidgetType::Linear => format!(
                        "      await tester.pumpUntilProgressCompleted(find.byType(LinearProgressIndicator), timeout: {});\n{}",
                        Self::duration(*timeout_millis, 30000),
                        Self::settle(*settle),
                    ),
                    feature_test::ProgressWidgetType::Radial => format!(
                        "      await tester.pumpUntilProgressCompleted(find.byType(CircularProgressIndicator), timeout: {});\n{}",
                        Self::duration(*timeout_millis, 30000),
                        Self::settle(*settle),
                    ),
                },
            },
            Step::Tap { tap } => format!(
                "      await tester.pumpAndSettle();\n      await tester.tap({}, kind: PointerDeviceKind.mouse);\n      await tester.pumpAndSettle();\n",
                Self::find_by(ctx, tap)
            ),
            Step::Type { r#type } => format!(
                "      await tester.type({}, {});\n",
                Self::find_by(ctx, &r#type.by),
                ctx.vars.replace_var_usage(&r#type.value)
            ),
            Step::Match { r#match } => match &r#match.target {
                feature_test::MatchTarget::Key { key } => {
                    format!(
                        "      expect(find.byKey(Key('{}')), findsOneWidget);\n",
                        ctx.vars.replace_var_usage(key)
                    )
                }
                feature_test::MatchTarget::Text { text } => {
                    format!(
                        "      expect(find.text('{}'), findsOneWidget);\n",
                        ctx.vars.replace_var_usage(text)
                    )
                }
                feature_test::MatchTarget::Screenshot { screenshot } => {
                    format!(
                        "      await tester.compareScreenshot('{}', '{}');\n",
                        file_name,
                        ctx.vars.replace_var_usage(screenshot)
                    )
                }
            },
            Step::Scroll { scroll } => format!(
                "      await tester.drag({}, const Offset({}, {}));\n",
                Self::find_by(ctx, &scroll.by),
                scroll.delta.x,
                scroll.delta.y
            ),
            Step::Pointer { pointer } => match pointer {
                feature_test::PointerAction::Move { to, remove } => format!(
                    "      await tester.movePointer(Offset({}, {}), remove: {});\n",
                    to.x, to.y, remove,
                ),
            },
        }
    }

    fn find_by(ctx: &HookContext<'_, GenState>, by: &feature_test::FindBy) -> String {
        match by {
            feature_test::FindBy::Key { key } => {
                format!("find.byKey(Key('{}'))", ctx.vars.replace_var_usage(key))
            }
            feature_test::FindBy::Text { text } => {
                format!("find.text('{}')", ctx.vars.replace_var_usage(text))
            }
            feature_test::FindBy::Placeholder { placeholder } => {
                format!(
                    "find.byPlaceholder('{}')",
                    ctx.vars.replace_var_usage(placeholder)
                )
            }
            feature_test::FindBy::Type { r#type } => {
                format!("find.byType({})", ctx.vars.replace_var_usage(r#type))
            }
        }
    }

    fn duration(duration: Option<u32>, default_ms: u32) -> String {
        format!("Duration(milliseconds: {})", duration.unwrap_or(default_ms))
    }

    fn settle(settle: bool) -> String {
        if settle {
            "      await tester.pumpAndSettle();\n".to_owned()
        } else {
            "".to_owned()
        }
    }
}
