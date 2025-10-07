use std::fs;

use tracing::info;

use crate::{
    code_gen::{gen_flutter::GenFlutter, gen_iface::CodeGenTrait},
    models::{args::AppArgs, config::Config, feature_test::FeatureTest},
    utils::{
        dir::DirUtils,
        errors::{EmptyResult, ResultWithError},
    },
};

/// Main controller to run the code generation logic.
pub struct CodeGen {
    args: AppArgs,
    config: Config,
}

impl CodeGen {
    pub fn new(args: AppArgs, config: Config) -> Self {
        Self { args, config }
    }

    pub fn execute(&self) -> EmptyResult {
        info!(
            "Code generation started with config for project type: {:?}",
            self.config.project_type
        );

        self.generate_code()?;

        info!("Code generation finished");

        Ok(())
    }

    fn generate_code(&self) -> EmptyResult {
        let features = FeatureTest::all_from_curr_dir()?;
        if features.is_empty() {
            info!("No feature test files found. Nothing to generate.");
            return Ok(());
        }

        let cwd = DirUtils::exec_dir()?;
        let out_dir = cwd.join("integration_test/generated");

        _ = fs::remove_dir_all(&out_dir);
        fs::create_dir_all(out_dir)?;

        let generators = self.get_generators()?;
        for generator in generators {
            generator.run(&features)?;
        }

        Ok(())
    }

    fn get_generators(&self) -> ResultWithError<Vec<Box<dyn CodeGenTrait>>> {
        let all_generators: Vec<Box<dyn CodeGenTrait>> = vec![Box::new(GenFlutter::from_exec_dir(
            self.args.clone(),
            self.config.clone(),
        )?)];

        let generators = all_generators
            .into_iter()
            .filter(|generator| generator.get_type() == self.config.project_type)
            .collect::<Vec<_>>();

        if generators.is_empty() {
            return Err(format!(
                "No code generator found for project type: {:?}",
                self.config.project_type
            )
            .into());
        }

        Ok(generators)
    }
}
