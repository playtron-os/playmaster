use crate::utils::command::CommandUtils;

impl CommandUtils {
    pub fn set_death_signal() {
        match prctl::set_death_signal(6) {
            Ok(_) => println!("Set death signal to 6"),
            Err(err) => println!("Error setting death signal to 6, error: {}", err),
        }
    }
}
