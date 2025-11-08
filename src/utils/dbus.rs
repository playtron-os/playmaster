const DBUS_PATH: &str = "/one/playmaster/E2E";
const DBUS_INTERFACE: &str = "one.playmaster.E2E";
const DBUS_METHOD_CONTINUE: &str = "Continue";

pub struct DbusUtils {}

impl DbusUtils {
    pub fn get_dbus_path() -> &'static str {
        DBUS_PATH
    }

    pub fn get_dbus_interface() -> &'static str {
        DBUS_INTERFACE
    }

    pub fn get_dbus_method_continue() -> &'static str {
        DBUS_METHOD_CONTINUE
    }

    pub fn dbus_method_continue_cmd(input: &str) -> String {
        format!(
            "busctl --user call {} {} {} {} s \"{}\"",
            Self::get_dbus_interface(),
            Self::get_dbus_path(),
            Self::get_dbus_interface(),
            Self::get_dbus_method_continue(),
            input
        )
    }
}
