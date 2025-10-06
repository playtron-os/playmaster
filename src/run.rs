use tracing::info;

use crate::{
    hooks::{self, iface::HookListExt as _},
    models::{args::AppArgs, config::Config, feature_test::FeatureTest},
    utils::errors::EmptyResult,
};

/// Main controller to run the application logic.
pub struct Run {
    args: AppArgs,
    config: Config,
    hooks: Vec<Box<dyn hooks::iface::Hook>>,
}

impl Run {
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

        self.run_tests(features);

        for hook_type in hooks::iface::HookType::post_hooks() {
            self.run_hooks_of_type(hook_type)?;
        }

        info!("Execution finished");

        Ok(())
    }

    fn run_tests(&self, features: Vec<FeatureTest>) {
        println!("#### {features:?}");
    }
}
