use anyhow::{anyhow, Result};
use std::process::{Command, Stdio};

/// Execute an external shell command to play an episode file and/or URL.
pub fn execute(command: &str, path: &str) -> Result<()> {
    let mut cmd_string = command.to_string();
    if cmd_string.contains("%s") {
        // if command contains "%s", replace the path with that value
        cmd_string = cmd_string.replace("%s", &format!("\"{}\"", path));
    } else {
        // otherwise, add path to the end of the command
        cmd_string = format!("{} \"{}\"", cmd_string, path);
    }

    let mut cmd = Command::new("/bin/sh");
    cmd.arg("-c").arg(cmd_string);
    cmd.stdout(Stdio::null()).stderr(Stdio::null());
    match cmd.spawn() {
        Ok(_) => Ok(()),
        Err(err) => Err(anyhow!(err)),
    }
}
