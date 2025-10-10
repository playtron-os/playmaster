use tracing::info;

use crate::{
    code_run,
    hooks::{self, iface::HookListExt as _},
    models::{
        args::{AppArgs, AppMode, Command},
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
}

impl CodeRun {
    pub fn new(args: AppArgs, config: Config) -> Self {
        let hooks = Self::load_hooks(&config);
        Self {
            args,
            config,
            hooks,
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

    fn run_hooks_of_type(&self, hook_type: hooks::iface::HookType) -> EmptyResult {
        let hooks_to_run = self.hooks.hooks_of_type(hook_type);
        for hook in hooks_to_run {
            hook.run(&self.args, &self.config)?;
        }
        Ok(())
    }

    pub fn execute(&self) -> EmptyResult {
        let features = FeatureTest::all_from_curr_dir()?;

        info!(
            "Executing with config for project type: {:?}",
            self.config.project_type
        );

        for hook_type in hooks::iface::HookType::pre_hooks() {
            self.run_hooks_of_type(hook_type)?;
        }

        self.run_tests(features)?;

        for hook_type in hooks::iface::HookType::post_hooks() {
            self.run_hooks_of_type(hook_type)?;
        }

        info!("Execution finished");

        Ok(())
    }

    fn run_tests(&self, features: Vec<FeatureTest>) -> EmptyResult {
        if let Command::Run { setup: true, .. } = self.args.command {
            info!("Setup flag detected, performing only setup tasks without executing tests.");
            return Ok(());
        }

        let runners: Vec<Box<dyn code_run::run_iface::CodeRunTrait>> = vec![Box::new(
            code_run::run_flutter::RunFlutter::new(self.args.clone(), self.config.clone()),
        )];
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

        match &self.args.command {
            Command::Run { mode, .. } => match mode {
                Some(AppMode::Local) | None => self.run_tests_locally(runner, features),
                Some(AppMode::Remote) => self.run_tests_remotely(runner, features),
            },
            _ => unreachable!(),
        }
    }

    fn run_tests_locally(
        &self,
        runner: Box<dyn code_run::run_iface::CodeRunTrait>,
        features: Vec<FeatureTest>,
    ) -> EmptyResult {
        runner.run(&features)
    }

    // TODO: Implement remote test running
    fn run_tests_remotely(
        &self,
        runner: Box<dyn code_run::run_iface::CodeRunTrait>,
        features: Vec<FeatureTest>,
    ) -> EmptyResult {
        runner.run(&features)
    }
}
