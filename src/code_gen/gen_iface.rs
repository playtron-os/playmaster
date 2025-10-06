use crate::{
    models::{
        args::AppArgs,
        config::{Config, ProjectType},
        feature_test::FeatureTest,
    },
    utils::errors::EmptyResult,
};

/// Trait that all code generation implementations must adhere to.
pub trait CodeGenTrait {
    fn get_type(&self) -> ProjectType;
    fn run(&self, args: &AppArgs, config: &Config, features: &[FeatureTest]) -> EmptyResult;
}
