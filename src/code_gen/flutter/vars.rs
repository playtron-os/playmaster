use crate::{
    code_gen::flutter::GenFlutter,
    models::vars::Vars,
    utils::{dir::DirUtils, errors::EmptyResult},
};
use std::fs;
use tracing::{error, info};

impl GenFlutter {
    pub fn generate_vars(&self) -> EmptyResult {
        let feature_test_dir = DirUtils::curr_dir()?.join("feature_test");

        if !feature_test_dir.exists() {
            error!("feature_test directory not found");
            return Ok(());
        }

        let all_vars = Vars::all_from_curr_dir()?;
        if all_vars.is_empty() {
            return Ok(());
        }

        let mut dart = String::new();
        dart.push_str("// GENERATED FILE - DO NOT EDIT\n");
        dart.push_str("// This file contains vars from all *.vars.yaml files in feature_test/\n\n");

        for yaml in all_vars {
            let name = yaml.file_name;
            let vars = yaml.content;

            // Dart class name = file name before ".vars.yaml", PascalCase
            let base_name = name.strip_suffix(".vars.yaml").unwrap();
            let class_name = Self::to_pascal_case(base_name);

            dart.push_str(&format!("class {} {{\n", class_name));

            let mut vars_vec: Vec<_> = vars.0.iter().collect();
            vars_vec.sort_by_key(|(key, _)| *key);
            for (key, value) in vars_vec {
                dart.push_str(&format!("  static const {} = '{}';\n", key, value));
            }

            dart.push_str("}\n\n");
        }

        let out_file = self.out_dir.join("vars.dart");
        fs::write(&out_file, dart)?;
        info!("Generated vars.dart");
        Ok(())
    }

    fn to_pascal_case(name: &str) -> String {
        name.split(['_', '-'])
            .filter(|s| !s.is_empty())
            .map(|s| {
                let mut c = s.chars();
                match c.next() {
                    None => String::new(),
                    Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
                }
            })
            .collect::<String>()
    }
}
