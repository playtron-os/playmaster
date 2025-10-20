use std::net::TcpStream;

use regex::Regex;
use ssh2::Session;
use tracing::info;

use crate::{
    hooks::iface::{Hook, HookContext, HookType},
    models::{
        self,
        app_state::{AppState, RemoteInfo},
        args::AppMode,
    },
    utils::errors::{EmptyResult, OptionResultTrait, ResultTrait as _, ResultWithError},
};

/// Hook to establish connection to remote host if needed.
pub struct HookConnect {}
impl HookConnect {
    pub fn new() -> Self {
        HookConnect {}
    }

    fn prompt_for_remote_conn(&self) -> ResultWithError<bool> {
        let res = inquire::Select::new(
            "Do you want to connect to a remote host?",
            vec!["Yes", "No"],
        )
        .prompt()?;

        if res == "Yes" {
            return Ok(true);
        }

        info!("Proceeding with local connection.");
        Ok(false)
    }

    fn prompt_for_address(&self) -> ResultWithError<String> {
        let addr = inquire::Text::new("Enter the remote address (e.g., user@ip_address:port):")
            .with_placeholder("dev@192.168.1.100:22")
            .prompt()?;
        Ok(addr)
    }

    /// parse "user@host:port" (port optional, defaults to 22)
    fn parse_addr(&self, s: &str) -> ResultWithError<(String, String, u16)> {
        // simple regex parsing: user@hostname:port
        let re = Regex::new(r"^(?P<user>[^@]+)@(?P<host>[^:]+)(:(?P<port>\d+))?$")?;
        if let Some(caps) = re.captures(s) {
            let user = caps
                .name("user")
                .auto_err("failure to parse remote address user")?
                .as_str()
                .to_string();
            let host = caps
                .name("host")
                .auto_err("failure to parse remote address host")?
                .as_str()
                .to_string();
            let port = caps
                .name("port")
                .map(|m| m.as_str().parse::<u16>().unwrap_or(22))
                .unwrap_or(22);
            Ok((user, host, port))
        } else {
            Err("Invalid address format. Expected user@host:port".into())
        }
    }

    fn establish_ssh_connection(&self, ctx: &HookContext<'_, AppState>) -> EmptyResult {
        let remote_addr = if let models::args::Command::Run { remote_addr, .. } = &ctx.args.command
        {
            if let Some(addr) = remote_addr {
                addr.clone()
            } else {
                self.prompt_for_address()?
            }
        } else {
            return Err("Invalid command context for establishing SSH connection".into());
        };

        info!("Establishing SSH connection to remote host: {remote_addr}...");

        // prompt for password
        let password = if let Ok(pass) = std::env::var("REMOTE_PASSWORD") {
            pass
        } else {
            inquire::Password::new("Enter your remote device's password: ")
                .without_confirmation()
                .with_display_mode(inquire::PasswordDisplayMode::Hidden)
                .prompt()?
        };

        // parse address
        let (user, host, port) = self
            .parse_addr(&remote_addr)
            .map_err(|e| format!("address parse error: {}", e))?;

        // Test connection now (attempt handshake)
        let tcp = TcpStream::connect((host.as_str(), port))
            .map_err(|e| format!("unable to connect to {}:{} — {}", host, port, e))?;
        let mut sess = Session::new().auto_err("failed to create ssh session")?;
        sess.set_tcp_stream(tcp);
        sess.handshake()
            .map_err(|e| format!("SSH handshake failed: {}", e))?;
        sess.userauth_password(&user, &password)
            .map_err(|e| format!("SSH auth failed: {}", e))?;

        if !sess.authenticated() {
            return Err("SSH authentication unsuccessful".into());
        }

        // Save connection info into state so other parts of the app can use it.
        ctx.initiate_remote(RemoteInfo {
            user,
            host,
            port,
            password,
        })?;

        info!("Remote connection info stored in state.");

        Ok(())
    }
}

impl Hook for HookConnect {
    fn get_type(&self) -> HookType {
        HookType::Connect
    }

    fn run(&self, ctx: &HookContext<'_, AppState>) -> EmptyResult {
        if let models::args::Command::Run {
            mode: Some(mode), ..
        } = &ctx.args.command
        {
            info!("Connection mode specified via command line: {:?}", mode);

            if *mode == AppMode::Local {
                return Ok(());
            }
        } else if !self.prompt_for_remote_conn()? {
            return Ok(());
        }

        self.establish_ssh_connection(ctx)?;

        Ok(())
    }
}
