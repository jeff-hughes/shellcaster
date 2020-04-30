use std::process::{Command, Stdio};

/// Execute an external shell command to play an episode file and/or URL.
pub fn execute(command: &str, path: &str) -> Result<(), std::io::Error> {
    // Command expects a command and then optional arguments (giving
    // everything to it in a string doesn't work), so we need to split
    // on white space and treat everything after the first word as args
    let cmd_string = String::from(command);
    let mut parts = cmd_string.trim().split_whitespace();
    let base_cmd = parts.next().unwrap();
    let args_iter = parts;

    let mut args: Vec<String>;
    if cmd_string.contains("%s") {
        args = args_iter.map(|a| {
            if a == "%s" {
                return a.replace("%s", path);
            } else {
                return a.to_string();
            }
        }).collect();
    } else {
        args = args_iter.map(|a| a.to_string()).collect();
        args.push(path.to_string());
    }

    let mut cmd = Command::new(base_cmd);
    cmd.args(args)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    match cmd.spawn() {
        Ok(_) => Ok(()),
        Err(err) => Err(err),
    }
}