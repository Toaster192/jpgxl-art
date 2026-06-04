//! Shared "run a libjxl tool over a temp file" subprocess driver.
//!
//! Both halves of the render pipeline — `jxl_from_tree` (encode) and `djxl`
//! (decode) — follow the same recipe: write the input to a temp file, spawn
//! the tool with `<input> <output>` paths, wait with a wall-clock cap (killing
//! a hung child so it can't pin a `spawn_blocking` worker forever), then read
//! the output file back. `run_with_temp_files` is that recipe; the callers
//! only differ in binary, leading args, and file extensions.

use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use rand::Rng;

/// Default wall-clock cap for a single tool invocation. Override with
/// `RENDER_TIMEOUT_SECS=N` for ops tuning. A normal render completes in
/// <100 ms; the cap only exists to reap a hung child.
const DEFAULT_TIMEOUT_SECS: u64 = 30;

/// Per-invocation timeout, from `RENDER_TIMEOUT_SECS` or the default.
pub fn timeout() -> Duration {
    let secs = std::env::var("RENDER_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(DEFAULT_TIMEOUT_SECS);
    Duration::from_secs(secs)
}

/// How often we poll the child for completion. A render's two subprocess
/// hops are on the interactive hot path, so we keep this tight — the wasted
/// wakeups are negligible next to the decode itself.
const POLL_INTERVAL: Duration = Duration::from_millis(5);

/// Run `bin` over a temp input/output file pair and return the output bytes.
///
/// `input` is written to a temp file with extension `in_ext`; `bin` is invoked
/// as `bin <extra_args…> <input> <output>` (output uses extension `out_ext`),
/// and the output file's bytes are returned. Both temp files are removed on
/// every exit path. stdout is discarded (`Stdio::null`) so a chatty child
/// can't fill an unread pipe and block; stderr is captured for error context.
pub fn run_with_temp_files(
    bin: &str,
    extra_args: &[&str],
    input: &[u8],
    in_ext: &str,
    out_ext: &str,
) -> Result<Vec<u8>, String> {
    if !std::path::Path::new(bin).exists() {
        return Err(format!(
            "{bin} binary not found. Run 'make setup' to build it."
        ));
    }

    let id: u64 = rand::thread_rng().gen();
    let tmp = std::env::temp_dir();
    let input_path = tmp.join(format!("artxl_{id}.{in_ext}"));
    let output_path = tmp.join(format!("artxl_{id}.{out_ext}"));

    std::fs::write(&input_path, input).map_err(|e| format!("write temp input: {e}"))?;

    let mut child = Command::new(bin)
        .args(extra_args)
        .arg(&input_path)
        .arg(&output_path)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            let _ = std::fs::remove_file(&input_path);
            format!("launch {bin}: {e}")
        })?;

    let timeout = timeout();
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) => {
                if start.elapsed() > timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = std::fs::remove_file(&input_path);
                    let _ = std::fs::remove_file(&output_path);
                    return Err(format!("{bin} timed out after {}s", timeout.as_secs()));
                }
                std::thread::sleep(POLL_INTERVAL);
            }
            Err(e) => {
                let _ = std::fs::remove_file(&input_path);
                let _ = std::fs::remove_file(&output_path);
                return Err(format!("{bin} wait: {e}"));
            }
        }
    };

    let _ = std::fs::remove_file(&input_path);

    if !status.success() {
        let _ = std::fs::remove_file(&output_path);
        let mut stderr = String::new();
        if let Some(mut s) = child.stderr.take() {
            let _ = s.read_to_string(&mut stderr);
        }
        let stderr = stderr.trim();
        return Err(if stderr.is_empty() {
            format!("{bin} exited with {status}")
        } else {
            format!("{bin} exited with {status}: {stderr}")
        });
    }

    let bytes = std::fs::read(&output_path).map_err(|e| format!("read {bin} output: {e}"))?;
    let _ = std::fs::remove_file(&output_path);
    Ok(bytes)
}
