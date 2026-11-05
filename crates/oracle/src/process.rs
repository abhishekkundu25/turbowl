//! Running an external oracle: a scratch directory and a subprocess with a
//! time limit.

use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

/// How long an external oracle may run: `ORACLE_TIMEOUT_SECS`, default 600
/// (the spec's ten minutes).
pub(crate) fn timeout() -> Duration {
    let secs = std::env::var("ORACLE_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(600);
    Duration::from_secs(secs)
}

/// A new, empty directory under `target/oracle/` for one run.
pub(crate) fn scratch_dir(tag: &str) -> Result<PathBuf, String> {
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = crate::workspace_root()
        .join("target/oracle")
        .join(format!("{tag}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    Ok(dir)
}

/// Runs the command with stdout and stderr captured to files in `dir`. Kills
/// it after `limit`. Returns stdout, or an error with the exit status and the
/// end of stderr.
pub(crate) fn run(cmd: &mut Command, dir: &std::path::Path, limit: Duration) -> Result<String, String> {
    let out_path = dir.join("stdout.txt");
    let err_path = dir.join("stderr.txt");
    let file = |p: &PathBuf| std::fs::File::create(p).map_err(|e| format!("{}: {e}", p.display()));
    let mut child = cmd
        .stdin(Stdio::null())
        .stdout(file(&out_path)?)
        .stderr(file(&err_path)?)
        .spawn()
        .map_err(|e| format!("cannot start {:?}: {e}", cmd.get_program()))?;
    let start = Instant::now();
    let status = loop {
        match child.try_wait().map_err(|e| e.to_string())? {
            Some(status) => break status,
            None if start.elapsed() > limit => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("timed out after {}s", limit.as_secs()));
            }
            None => std::thread::sleep(Duration::from_millis(5)),
        }
    };
    let stdout = std::fs::read_to_string(&out_path).unwrap_or_default();
    if status.success() {
        return Ok(stdout);
    }
    let stderr = std::fs::read_to_string(&err_path).unwrap_or_default();
    let tail: Vec<&str> = stderr.lines().rev().take(8).collect();
    Err(format!(
        "{status}: {}",
        tail.into_iter().rev().collect::<Vec<_>>().join("\n")
    ))
}
