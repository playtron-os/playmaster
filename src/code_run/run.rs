use std::sync::{Arc, RwLock};

use tracing::{error, info};

use crate::{
    code_run,
    hooks::{
        self,
        iface::{HookContext, HookListExt as _},
    },
    models::{
        app_state::AppState,
        args::{AppArgs, Command},
        config::Config,
        feature_test::FeatureTest,
        vars::Vars,
    },
    utils::{
        command::CommandUtils,
        errors::{EmptyResult, OptionResultTrait},
        execution::ExecutionUtils,
    },
};

/// Main controller to run the tests.
pub struct CodeRun {
    args: AppArgs,
    config: Config,
    vars: Vars,
    hooks: Vec<Box<dyn hooks::iface::Hook>>,
    state: Arc<RwLock<AppState>>,
}

impl CodeRun {
    pub fn new(args: AppArgs, config: Config, vars: Vars) -> Self {
        let hooks = Self::load_hooks(&config);
        let state = Arc::new(RwLock::new(AppState::default()));
        Self {
            args,
            config,
            vars,
            hooks,
            state,
        }
    }

    fn load_hooks(config: &Config) -> Vec<Box<dyn hooks::iface::Hook>> {
        let mut hooks: Vec<Box<dyn hooks::iface::Hook>> = vec![
            Box::new(hooks::check_dependency::HookCheckDependency::new()),
            Box::new(hooks::connect::HookConnect::new()),
        ];

        hooks.extend(config.hooks.iter().map(|hook| {
            Box::new(hooks::custom::HookCustom::new(hook.clone())) as Box<dyn hooks::iface::Hook>
        }));

        hooks
    }

    fn run_hooks_of_type(
        &self,
        ctx: &HookContext<'_, AppState>,
        hook_type: hooks::iface::HookType,
        has_error: bool,
    ) -> EmptyResult {
        let hooks_to_run = self.hooks.hooks_of_type(hook_type);
        for hook in hooks_to_run {
            if hook.continue_on_error() || !has_error {
                hook.run(ctx)?;
            }
        }
        Ok(())
    }

    pub fn execute(&self) -> EmptyResult {
        let features = FeatureTest::all_from_curr_dir()?;

        info!(
            "Executing with config for project type: {:?}",
            self.config.project_type
        );

        let ctx = HookContext {
            args: &self.args,
            config: &self.config,
            vars: &self.vars,
            state: Arc::clone(&self.state),
        };

        let mut has_error = false;

        for hook_type in hooks::iface::HookType::pre_hooks() {
            if let Err(err) = self.run_hooks_of_type(&ctx, hook_type, has_error) {
                error!("Pre-hook error {:?} failed: {}", hook_type, err);
                has_error = true;

                if !ExecutionUtils::is_running() {
                    break;
                }
            }
        }

        let res = if !has_error && ExecutionUtils::is_running() {
            self.run_tests(&ctx, features)
        } else {
            Err("Pre-hook failed".into())
        };

        if let Err(err) = CommandUtils::terminate_all_cmds() {
            error!("Failed to terminate running commands: {}", err);
        }

        for hook_type in hooks::iface::HookType::post_hooks() {
            if let Err(err) = self.run_hooks_of_type(&ctx, hook_type, has_error) {
                error!("Post-hook {:?} failed: {}", hook_type, err);
                has_error = true;
            }
        }

        if let Err(err) = CommandUtils::terminate_all_cmds() {
            error!("Failed to terminate running commands: {}", err);
        }

        info!("Execution finished");

        res
    }

    fn run_tests(
        &self,
        ctx: &HookContext<'_, AppState>,
        features: Vec<FeatureTest>,
    ) -> EmptyResult {
        if let Command::Run { setup: true, .. } = self.args.command {
            info!("Setup flag detected, performing only setup tasks without executing tests.");
            return Ok(());
        }

        let runners: Vec<Box<dyn code_run::run_iface::CodeRunTrait>> =
            vec![Box::new(code_run::run_flutter::RunFlutter::new())];
        let runner = runners
            .into_iter()
            .find(|r| r.get_type() == self.config.project_type)
            .auto_err(
                format!(
                    "No test runner found for project type: {:?}",
                    self.config.project_type,
                )
                .as_str(),
            )?;

        runner.run(ctx, &features)
    }
}
