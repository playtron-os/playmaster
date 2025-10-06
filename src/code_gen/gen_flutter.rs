use crate::{
    code_gen::gen_iface::CodeGenTrait,
    models::{
        args::AppArgs,
        config::{Config, ProjectType},
        feature_test::FeatureTest,
    },
    utils::errors::EmptyResult,
};

pub struct GenFlutter {}

impl CodeGenTrait for GenFlutter {
    fn get_type(&self) -> ProjectType {
        ProjectType::Flutter
    }

    fn run(&self, args: &AppArgs, config: &Config, features: &[FeatureTest]) -> EmptyResult {
        Ok(())
    }
}
