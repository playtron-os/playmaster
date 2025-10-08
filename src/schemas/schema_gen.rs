use std::{fs, path::Path};

use schemars::{JsonSchema, schema_for};
use tracing::info;

use crate::utils::{self, errors::EmptyResult};

pub struct SchemaGen {}

impl SchemaGen {
    pub fn new() -> Self {
        Self {}
    }

    pub fn execute(&self) -> EmptyResult {
        self.generate_single::<crate::models::feature_test::FeatureTest>(
            "feature_test_schema.json",
        )?;
        self.generate_single::<crate::models::config::Config>("config.json")?;
        Ok(())
    }

    /// Generic JSON schema generator.
    ///
    /// # Arguments
    /// * `output_path` - Path where the schema should be saved.
    ///
    /// # Example
    /// ```rust
    /// generate_schema::<FeatureTest>("feature_test_schema.json")?;
    /// ```
    fn generate_single<T>(&self, file_name: impl AsRef<Path>) -> EmptyResult
    where
        T: JsonSchema,
    {
        let schema = schema_for!(T);
        let schema_str = serde_json::to_string_pretty(&schema)?;

        let path = utils::dir::DirUtils::curr_dir()?
            .join("src")
            .join("schemas")
            .join("generated")
            .join(file_name);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }

        fs::write(&path, schema_str)?;
        info!("✅ Schema generated successfully at {}", path.display());
        Ok(())
    }
}
