use crate::{
    models::{config::ProjectType, feature_test::FeatureTest},
    utils::errors::EmptyResult,
};

/// Trait that all code generation implementations must adhere to.
pub trait CodeGenTrait {
    fn get_type(&self) -> ProjectType;
    fn run(&self, features: &[FeatureTest]) -> EmptyResult;
}
