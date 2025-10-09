use std::fs;

use tracing::info;

use crate::{
    code_gen::flutter::GenFlutter, models::feature_test::FeatureTest, utils::errors::EmptyResult,
};

impl GenFlutter {
    /// Generate an `all_tests.dart` file that imports and runs all generated tests.
    pub fn generate_all_entrypoint(&self, features: &[FeatureTest]) -> EmptyResult {
        let entry_file = self.out_dir.join("all_tests.dart");
        let mut content = String::new();

        content.push_str("// GENERATED FILE - DO NOT EDIT\n");
        content.push_str("// This file aggregates all generated integration tests.\n\n");

        // Import each generated test file
        for feature in features {
            let import_name = feature.name.to_lowercase().replace(' ', "_") + "_test.dart";
            let alias = feature.name.to_lowercase().replace([' ', '-'], "_");
            content.push_str(&format!("import '{import_name}' as {alias};\n"));
        }

        content.push_str("\nvoid main() {\n");
        for feature in features {
            let alias = feature.name.to_lowercase().replace([' ', '-'], "_");
            content.push_str(&format!("  {alias}.main();\n"));
        }
        content.push_str("}\n");

        fs::write(entry_file, content)?;
        info!("Generated all_tests.dart entrypoint");
        Ok(())
    }
}
