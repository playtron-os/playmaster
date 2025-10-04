use crate::{
    config::Config,
    hooks::{self, iface::HookListExt as _},
    utils::errors::EmptyResult,
};

/// Main controller to run the application logic.
pub struct Run {
    config: Config,
    hooks: Vec<Box<dyn hooks::iface::Hook>>,
}

impl Run {
    pub fn new(config: Config) -> Self {
        let hooks = Self::load_hooks();
        Self { config, hooks }
    }

    fn load_hooks() -> Vec<Box<dyn hooks::iface::Hook>> {
        vec![
            Box::new(hooks::check_dependency::HookCheckDependency::new()),
            Box::new(hooks::connect::HookConnect::new()),
        ]
    }

    fn run_hooks_of_type(&self, hook_type: hooks::iface::HookType) -> EmptyResult {
        let hooks_to_run = self.hooks.hooks_of_type(hook_type);
        for hook in hooks_to_run {
            hook.run(&self.config)?;
        }
        Ok(())
    }

    pub fn execute(&self) -> EmptyResult {
        println!(
            "Executing with config for project type: {:?}",
            self.config.project_type
        );

        for hook_type in hooks::iface::HookType::pre_hooks() {
            self.run_hooks_of_type(hook_type)?;
        }

        for hook_type in hooks::iface::HookType::post_hooks() {
            self.run_hooks_of_type(hook_type)?;
        }

        Ok(())
    }

    fn run_tests(config: &Config, remote: bool) {
        // if let Some(hooks) = &config.hooks {
        //     if let Some(pre_hooks) = &hooks.pre_run {
        //         for hook in pre_hooks {
        //             println!("⚙️ Running pre-run hook: {}", hook.command);
        //             let mut cmd = Command::new(&hook.command);
        //             if let Some(args) = &hook.args {
        //                 cmd.args(args);
        //             }
        //             cmd.spawn().expect("Failed to start pre-run hook");
        //         }
        //     }
        // }

        // if remote {
        //     println!("🌐 Running tests on remote device...");
        //     if let Some(remote_cfg) = &config.remote {
        //         println!("Target device: {:?}", remote_cfg.device_host);
        //     }
        // } else {
        //     println!("🖥️ Running tests locally...");
        //     // TODO: Execute test runner logic here
        // }

        // if let Some(hooks) = &config.hooks {
        //     if let Some(post_hooks) = &hooks.post_run {
        //         for hook in post_hooks {
        //             println!("⚙️ Running post-run hook: {}", hook.command);
        //             let mut cmd = Command::new(&hook.command);
        //             if let Some(args) = &hook.args {
        //                 cmd.args(args);
        //             }
        //             cmd.spawn().expect("Failed to start post-run hook");
        //         }
        //     }
        // }
    }
}
