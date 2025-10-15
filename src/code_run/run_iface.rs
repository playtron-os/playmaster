use crate::{
    hooks::iface::HookContext,
    models::{config::ProjectType, feature_test::FeatureTest},
    utils::errors::EmptyResult,
};

/// Trait that all code run implementations must adhere to.
pub trait CodeRunTrait {
    fn get_type(&self) -> ProjectType;
    fn run(&self, ctx: &HookContext, features: &[FeatureTest]) -> EmptyResult;
}
