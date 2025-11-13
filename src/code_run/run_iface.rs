use std::pin::Pin;

use crate::{
    hooks::iface::HookContext,
    models::{app_state::AppState, config::ProjectType, feature_test::FeatureTest},
    utils::errors::EmptyResult,
};

/// Trait that all code run implementations must adhere to.
pub trait CodeRunTrait: Send + Sync {
    fn get_type(&self) -> ProjectType;
    fn run<'a>(
        &'a self,
        ctx: &'a HookContext<'a, AppState>,
        features: &'a [FeatureTest],
    ) -> Pin<Box<dyn Future<Output = EmptyResult> + Send + 'a>>;
}
