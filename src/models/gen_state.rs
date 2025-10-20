use crate::models::feature_test::FeatureTest;

/// Shared application state for code gen.
#[derive(Debug, Default, Clone)]
pub struct GenState {
    pub features: Vec<FeatureTest>,
}
