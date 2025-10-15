use std::{
    io::{Read as _, Write as _},
    net::TcpStream,
    time::Duration,
};

use ssh2::Session;

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
        channel.request_pty("xterm", None, None)?;

        // Run the command safely through sh -c
        let shell_cmd = format!("sh -c '{}'", cmd.replace("'", "'\\''"));
        channel.exec(&shell_cmd)?;

        // Set non-blocking mode so we can read both stdout and stderr without deadlocking (including carriage returns)
        sess.set_blocking(false);

        let mut stdout = String::new();
        let mut stderr = String::new();

        let mut out_buf = [0u8; 4096];
        let mut err_buf = [0u8; 4096];

        loop {
            let mut made_progress = false;

            match channel.read(&mut out_buf) {
                Ok(n) if n > 0 => {
                    made_progress = true;
                    let chunk = String::from_utf8_lossy(&out_buf[..n]);
                    // print without forcing newlines so carriage returns updates correctly
                    print!("{chunk}");
                    std::io::stdout().flush().ok();
                    stdout.push_str(&chunk);
                }
                Err(e) => {
                    if !self.is_would_block(&e) {
                        return Err(e.into());
                    }
                }
                _ => {}
            }

            match channel.stderr().read(&mut err_buf) {
                Ok(n) if n > 0 => {
                    made_progress = true;
                    let chunk = String::from_utf8_lossy(&err_buf[..n]);
                    eprint!("{chunk}");
                    std::io::stderr().flush().ok();
                    stderr.push_str(&chunk);
                }
                Err(e) => {
                    if !self.is_would_block(&e) {
                        return Err(e.into());
                    }
                }
                _ => {}
            }

            // When command finished and no more data, stop
            if channel.eof() {
                break;
            }

            // If neither stream had data this tick, back off briefly
            if !made_progress {
                std::thread::sleep(Duration::from_millis(50));
            }
        }

        sess.set_blocking(true);
        channel.wait_close()?;
        let status = channel.exit_status()?;

        Ok(CommandOutput {
            stdout,
            stderr,
            status,
        })
    }

    fn is_would_block(&self, e: &std::io::Error) -> bool {
        matches!(e.kind(), std::io::ErrorKind::WouldBlock)
    }
}
