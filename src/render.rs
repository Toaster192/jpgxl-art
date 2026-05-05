use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use rand::Rng;

use crate::codec;

/// Default wall-clock cap for a single `jxl_from_tree` invocation.
/// Override with `RENDER_TIMEOUT_SECS=N` for ops tuning. A normal render
/// completes in <100 ms; the cap exists to keep a hung child from
/// pinning a `spawn_blocking` worker forever.
const DEFAULT_RENDER_TIMEOUT_SECS: u64 = 30;

fn render_timeout() -> Duration {
    let secs = std::env::var("RENDER_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(DEFAULT_RENDER_TIMEOUT_SECS);
    Duration::from_secs(secs)
}

/// Encode `program_text` with `./jxl_from_tree`, decode the resulting JXL
/// bytes via `jxl-oxide`, and return the rendered RGBA8 buffer, its
/// dimensions, and the encoded JXL byte length (so callers can surface
/// the file size without re-invoking `jxl_from_tree`).
///
/// `size == 0` → render at the JXL's native dimensions.
/// Any other value → longest edge scaled to `size` px (Lanczos3 / Triangle).
pub fn render_roundtrip(program_text: &str, size: u32) -> Result<(Vec<u8>, u32, u32, u64), String> {
    let jxl = encode_jxl_from_tree(program_text)?;
    let jxl_size = jxl.len() as u64;
    let (rgba, w, h) = codec::decode_jxl(&jxl, size)?;
    Ok((rgba, w, h, jxl_size))
}

/// Shell out to `./jxl_from_tree` with the given program text and return
/// the generated JXL bytes.
pub fn encode_jxl_from_tree(program_text: &str) -> Result<Vec<u8>, String> {
    if !std::path::Path::new("./jxl_from_tree").exists() {
        return Err("jxl_from_tree binary not found. Run 'make setup' to build it.".to_string());
    }

    let id: u64 = rand::thread_rng().gen();
    let tmp = std::env::temp_dir();
    let input_path = tmp.join(format!("artxl_{}.xl", id));
    let output_path = tmp.join(format!("artxl_{}.jxl", id));

    std::fs::write(&input_path, program_text).map_err(|e| format!("write temp input: {}", e))?;

    let mut child = Command::new("./jxl_from_tree")
        .arg(&input_path)
        .arg(&output_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            let _ = std::fs::remove_file(&input_path);
            format!("launch jxl_from_tree: {}", e)
        })?;

    let timeout = render_timeout();
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
                    return Err(format!(
                        "jxl_from_tree timed out after {}s",
                        timeout.as_secs(),
                    ));
                }
                std::thread::sleep(Duration::from_millis(20));
            }
            Err(e) => {
                let _ = std::fs::remove_file(&input_path);
                let _ = std::fs::remove_file(&output_path);
                return Err(format!("jxl_from_tree wait: {}", e));
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
            format!("jxl_from_tree exited with {}", status)
        } else {
            format!("jxl_from_tree exited with {}: {}", status, stderr)
        });
    }

    let bytes = std::fs::read(&output_path).map_err(|e| format!("read jxl output: {}", e))?;
    let _ = std::fs::remove_file(&output_path);

    Ok(bytes)
}
