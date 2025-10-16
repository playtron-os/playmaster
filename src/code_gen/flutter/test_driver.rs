use std::fs;

use tracing::info;

use crate::{
    code_gen::flutter::GenFlutter,
    utils::{dir::DirUtils, errors::EmptyResult},
};

impl GenFlutter {
    pub fn generate_test_driver(&self) -> EmptyResult {
        let parent = DirUtils::curr_dir()?.join("test_driver");
        fs::create_dir_all(&parent)?;

        let file = parent.join("integration_test.dart");

        let content = r#"// GENERATED FILE - DO NOT EDIT
import 'package:integration_test/integration_test_driver.dart';

Future<void> main() => integrationDriver();
"#;

        fs::write(&file, content)?;
        info!("Generated integration_test.dart");
        Ok(())
    }
}
