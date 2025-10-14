use std::{
    io::{Read as _, Write as _},
    net::TcpStream,
    time::Duration,
};

use ssh2::Session;
use tracing::{error, info};

use crate::utils::errors::ResultWithError;

#[derive(Default)]
pub struct CommandOutput {
    pub stdout: String,
    pub stderr: String,
    pub status: i32,
}

/// Shared application state for hooks and runners.
/// Extend this struct with whatever runtime fields you need to share.
#[derive(Debug, Default, Clone)]
pub struct AppState {
    pub remote: Option<RemoteInfo>,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct RemoteInfo {
    pub user: String,
    pub host: String,
    pub port: u16,
    pub password: String,
}

impl RemoteInfo {
    pub fn get_sess(&self) -> ResultWithError<Session> {
        let tcp = TcpStream::connect((&self.host[..], self.port))?;
        let mut sess = Session::new()?;
        sess.set_tcp_stream(tcp);
        sess.handshake()?;
        sess.userauth_password(&self.user, &self.password)?;
        if !sess.authenticated() {
            return Err("SSH auth failed".into());
        }

        Ok(sess)
    }

    pub fn exec(&self, cmd: &str) -> ResultWithError<CommandOutput> {
        let cmd = cmd.replace("\\$", "$");

        let sess = self.get_sess()?;
        let mut channel = sess.channel_session()?;

        // Allocate a PTY to force line buffering
        channel.request_pty("xterm", None, None)?;
        channel.exec(&cmd)?;

        let mut stdout = String::new();
        let mut stderr = String::new();
        let mut stdout_buf = [0u8; 1024];
        let mut stderr_buf = [0u8; 1024];

        loop {
            let mut done = true;

            // Read stdout
            match channel.read(&mut stdout_buf) {
                Ok(n) if n > 0 => {
                    done = false;
                    let chunk = String::from_utf8_lossy(&stdout_buf[..n]);
                    info!("[Remote Output]: {}", chunk);
                    std::io::stdout().flush().ok();
                    stdout.push_str(&chunk);
                }
                _ => {}
            }

            // Read stderr
            match channel.stderr().read(&mut stderr_buf) {
                Ok(n) if n > 0 => {
                    done = false;
                    let chunk = String::from_utf8_lossy(&stderr_buf[..n]);
                    error!("[Remote Error]: {}", chunk);
                    std::io::stderr().flush().ok();
                    stderr.push_str(&chunk);
                }
                _ => {}
            }

            if done && channel.eof() {
                break;
            }
        }

        std::thread::sleep(Duration::from_millis(100));
        channel.wait_close()?;
        let exit_status = channel.exit_status()?;

        Ok(CommandOutput {
            stdout,
            stderr,
            status: exit_status,
        })
    }
}
