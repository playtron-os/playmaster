use crate::{
    code_gen::flutter::GenFlutter,
    hooks::iface::HookContext,
    models::gen_state::GenState,
    utils::{errors::EmptyResult, string::StringUtils},
};
use std::fs;
use tracing::info;

impl GenFlutter {
    pub fn generate_vars(&self, ctx: &HookContext<'_, GenState>) -> EmptyResult {
        let vars_data = &ctx.vars.data;
        if vars_data.is_empty() {
            return Ok(());
        }

        let mut dart = String::new();
        dart.push_str("// GENERATED FILE - DO NOT EDIT\n");
        dart.push_str("// This file contains vars from all *.vars.yaml files in feature_test/\n\n");

        for yaml in vars_data {
            let name = &yaml.file_name;
            let vars = &yaml.content;
            let class_name = StringUtils::to_pascal_case(name);

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
}
