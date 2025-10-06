use tracing::info;

use crate::utils::command::CommandUtils;

impl CommandUtils {
    pub fn set_death_signal() {
        match prctl::set_death_signal(6) {
            Ok(_) => info!("Set death signal to 6"),
            Err(err) => info!("Error setting death signal to 6, error: {}", err),
        }
    }
}
