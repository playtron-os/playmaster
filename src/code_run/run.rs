use std::sync::{Arc, RwLock};

use tracing::info;

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
    },
    utils::errors::{EmptyResult, OptionResultTrait},
};

/// Main controller to run the tests.
pub struct CodeRun {
    args: AppArgs,
    config: Config,
    hooks: Vec<Box<dyn hooks::iface::Hook>>,
    state: Arc<RwLock<AppState>>,
}

impl CodeRun {
    pub fn new(args: AppArgs, config: Config) -> Self {
        let hooks = Self::load_hooks(&config);
        let state = Arc::new(RwLock::new(AppState::default()));
        Self {
            args,
            config,
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
        ctx: &HookContext,
        hook_type: hooks::iface::HookType,
    ) -> EmptyResult {
        let hooks_to_run = self.hooks.hooks_of_type(hook_type);
        for hook in hooks_to_run {
            hook.run(ctx)?;
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
            state: Arc::clone(&self.state),
        };

        for hook_type in hooks::iface::HookType::pre_hooks() {
            self.run_hooks_of_type(&ctx, hook_type)?;
        }

        self.run_tests(&ctx, features)?;

        for hook_type in hooks::iface::HookType::post_hooks() {
            self.run_hooks_of_type(&ctx, hook_type)?;
        }

        info!("Execution finished");

        Ok(())
    }

    fn run_tests(&self, ctx: &HookContext, features: Vec<FeatureTest>) -> EmptyResult {
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
