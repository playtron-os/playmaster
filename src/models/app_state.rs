use std::{
    io::{Read as _, Write as _},
    net::TcpStream,
    sync::mpsc::{self, Receiver},
    time::Duration,
};

use ssh2::{PtyModes, Session};
use terminal_size::{Height, Width, terminal_size};

use crate::utils::errors::ResultWithError;

#[derive(Default, Debug)]
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
    pub os_info: OsInfo,
    // TODO: Implement ask for sudo password
    pub sudo_password: String,
    pub root_dir: String,
}

#[derive(Clone, Debug, Default)]
pub struct OsInfo {
    pub is_ostree: bool,
}

#[derive(Clone, Debug)]
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

        // Run the command safely through bash -c
        let bash_cmd = format!("bash -c '{}'", cmd.replace("'", "'\\''"));
        channel.exec(&bash_cmd)?;

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
                    if chunk != "exited" {
                        print!("[Remote Log]: {chunk}");
                    }
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
                    eprint!("[Remote Error Log]: {chunk}");
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

    /// Executes a remote command and yields stdout lines in real-time.
    pub fn exec_remote_stream<'a>(
        &'a self,
        cmd: &str,
    ) -> ResultWithError<impl Iterator<Item = String> + 'a> {
        let cmd = cmd.replace("\\$", "$");
        let sess = self.get_sess()?;

        // Get local terminal width/height
        let (cols, rows) = terminal_size()
            .map(|(Width(w), Height(h))| (w, h))
            .unwrap_or((120, 30)); // fallback if not a TTY

        // PTY modes
        let mut modes = PtyModes::new();
        modes.set_u32(ssh2::PtyModeOpcode::ECHO, 1);

        // request PTY with same dimensions
        let mut channel = sess.channel_session()?;
        channel.request_pty("xterm", Some(modes), Some((cols as u32, rows as u32, 0, 0)))?;

        let shell_cmd = format!("sh -c '{}'", cmd.replace("'", "'\\''"));
        channel.exec(&shell_cmd)?;

        sess.set_blocking(false);

        let (tx, rx) = mpsc::channel::<String>();

        // Spawn a thread that continuously reads stdout and sends complete lines
        std::thread::spawn(move || {
            let mut buffer = Vec::<u8>::new();
            let mut tmp_buf = [0u8; 4096];

            loop {
                match channel.read(&mut tmp_buf) {
                    Ok(n) if n > 0 => {
                        buffer.extend_from_slice(&tmp_buf[..n]);
                        // Split by newlines for streaming
                        while let Some(pos) = buffer.iter().position(|&b| b == b'\n') {
                            let line = buffer.drain(..=pos).collect::<Vec<_>>();
                            if let Ok(text) = String::from_utf8(line) {
                                let _ = tx.send(text.trim_end_matches('\n').to_string());
                            }
                        }
                    }
                    Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        if channel.eof() {
                            break;
                        }
                        std::thread::sleep(Duration::from_millis(30));
                    }
                    Err(_) => break,
                    _ => {}
                }
                if channel.eof() {
                    break;
                }
            }

            // Drain any remaining bytes
            if !buffer.is_empty()
                && let Ok(text) = String::from_utf8(buffer)
            {
                let _ = tx.send(text);
            }

            let _ = channel.wait_close();
        });

        // Return an iterator that yields lines from the channel
        Ok(RemoteLineIterator { rx })
    }
}

struct RemoteLineIterator {
    rx: Receiver<String>,
}

impl Iterator for RemoteLineIterator {
    type Item = String;
    fn next(&mut self) -> Option<Self::Item> {
        self.rx.recv().ok()
    }
}
