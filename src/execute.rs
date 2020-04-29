use std::process::{Command, Stdio};

pub fn execute(path: &str) -> Result<(), std::io::Error> {
    let mut cmd = Command::new("vlc");
    cmd.arg(path)
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    match cmd.spawn() {
        Ok(_) => Ok(()),
        Err(err) => Err(err),
    }
}