use chrono::Utc;
use std::io::{Read, Write};
use std::process::{Command, Stdio};

use crate::storage::{RunEntry, Storage};

pub fn execute_and_record(command: &[String]) -> std::io::Result<i32> {
    let storage = Storage::new();
    storage.ensure_dirs()?;

    let cmd_str = command.join(" ");
    let id = storage.next_id()?;

    let mut child = Command::new(&command[0])
        .args(&command[1..])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let mut stdout_buf = Vec::new();
    let mut stderr_buf = Vec::new();

    if let Some(mut stdout) = child.stdout.take() {
        let mut buf = [0u8; 4096];
        loop {
            let n = stdout.read(&mut buf)?;
            if n == 0 {
                break;
            }
            std::io::stdout().write_all(&buf[..n])?;
            std::io::stdout().flush()?;
            stdout_buf.extend_from_slice(&buf[..n]);
        }
    }

    if let Some(mut stderr) = child.stderr.take() {
        let mut buf = [0u8; 4096];
        loop {
            let n = stderr.read(&mut buf)?;
            if n == 0 {
                break;
            }
            std::io::stderr().write_all(&buf[..n])?;
            std::io::stderr().flush()?;
            stderr_buf.extend_from_slice(&buf[..n]);
        }
    }

    let status = child.wait()?;
    let exit_code = status.code().unwrap_or(-1);

    let mut combined = stdout_buf;
    combined.extend_from_slice(&stderr_buf);
    storage.save_log(id, &combined)?;

    let entry = RunEntry {
        id,
        cmd: cmd_str,
        exit: exit_code,
        at: Utc::now(),
    };
    storage.append_entry(&entry)?;

    Ok(exit_code)
}
