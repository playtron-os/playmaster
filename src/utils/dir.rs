use crate::utils::errors::{ResultTrait, ResultWithError};

pub struct DirUtils;

impl DirUtils {
    pub fn exec_dir() -> ResultWithError<std::path::PathBuf> {
        std::env::current_dir()
            .auto_err("Could not read current directory")
            .map(|p| {
                if cfg!(debug_assertions) {
                    p.join("sample_app")
                } else {
                    p
                }
            })
    }
}
