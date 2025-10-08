use crate::{models::config::ProjectType, utils::errors::{ResultTrait, ResultWithError}};

pub struct DirUtils;

impl DirUtils {
    pub fn exec_dir(project_type: &ProjectType) -> ResultWithError<std::path::PathBuf> {
        std::env::current_dir()
            .auto_err("Could not read current directory")
            .map(|p| {
                if cfg!(debug_assertions) {
                    p.join("samples").join(format!("{}_sample_app", project_type.to_string()))
                } else {
                    p
                }
            })
    }

    pub fn curr_dir() -> ResultWithError<std::path::PathBuf> {
        std::env::current_dir().auto_err("Could not read current directory")
    }
}
