use anyhow::{anyhow, Result};
use std::process::{Command, Stdio};

/// Execute an external shell command to play an episode file and/or URL.
pub fn execute(command: &str, path: &str) -> Result<()> {
    // Command expects a command and then optional arguments (giving
    // everything to it in a string doesn't work), so we need to split
    // on white space and treat everything after the first word as args
    let cmd_string = command.to_string();
    let mut parts = cmd_string.trim().split_whitespace();
    let base_cmd = parts.next().ok_or_else(|| anyhow!("Invalid command."))?;
    let mut cmd = Command::new(base_cmd);

    if cmd_string.contains("%s") {
        // if command contains "%s", replace the path with that value
        cmd.args(parts.map(|a| if a == "%s" { path } else { a }));
    } else {
        // otherwise, add path to the end of the command
        cmd.args(parts.chain(vec![path].into_iter()));
    }

    cmd.stdout(Stdio::null()).stderr(Stdio::null());
    match cmd.spawn() {
        Ok(_) => Ok(()),
        Err(err) => Err(anyhow!(err)),
    }
}
