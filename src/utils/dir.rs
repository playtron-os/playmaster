use crate::utils::errors::{ResultTrait, ResultWithError};

pub struct DirUtils;

impl DirUtils {
    pub fn curr_dir() -> ResultWithError<std::path::PathBuf> {
        std::env::current_dir().auto_err("Could not read current directory")
    }
}
