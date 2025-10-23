use std::sync::Mutex;

use tracing::error;

use crate::utils::errors::{EmptyResult, ResultTrait};

pub struct ExecutionUtils {}

lazy_static::lazy_static! {
    static ref IS_RUNNING: Mutex<bool> = Mutex::new(false);
}

impl ExecutionUtils {
    pub fn is_running() -> bool {
        match IS_RUNNING
            .lock()
            .auto_err("Failed to lock IS_RUNNING mutex")
        {
            Ok(locked) => *locked,
            Err(err) => {
                error!("{}", err);
                false
            }
        }
    }

    pub fn set_running(running: bool) -> EmptyResult {
        *IS_RUNNING
            .lock()
            .auto_err("Failed to lock IS_RUNNING mutex")? = running;
        Ok(())
    }
}
