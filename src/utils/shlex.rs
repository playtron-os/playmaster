use std::process::Command;

use shlex::Shlex;

use crate::utils::errors::{OptionResultTrait as _, ResultWithError};

pub struct ShlexUtils {}

impl ShlexUtils {
    pub fn parse_command(input: &str) -> ResultWithError<Command> {
        let parts: Vec<_> = Shlex::new(input).collect();

        let (program, args) = parts
            .split_first()
            .auto_err(format!("Failed to split command: {}", input).as_str())?;
        let mut command = Command::new(program);
        command.args(args);
        Ok(command)
    }
}
