use std::io::Read;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use image::imageops::FilterType;
use rand::Rng;

/// The libjxl decoder, built and bundled alongside `jxl_from_tree` by
/// `make setup` (RPATH `$ORIGIN/lib`). We decode with libjxl rather than a
/// pure-Rust decoder so output matches the reference editors exactly —
/// jxl-oxide rendered XYB / XYB+Squeeze images differently.
const DJXL_BIN: &str = "./djxl";

fn decode_timeout() -> Duration {
    let secs = std::env::var("RENDER_TIMEOUT_SECS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(30);
    Duration::from_secs(secs)
}

/// Decode a JXL byte stream into RGBA8, optionally resized so the longest
/// edge matches `max_dim`. `max_dim == 0` means native dimensions.
pub fn decode_jxl(bytes: &[u8], max_dim: u32) -> Result<(Vec<u8>, u32, u32), String> {
    let (rgba, w, h) = decode_via_djxl(bytes)?;

    if max_dim == 0 {
        return Ok((rgba, w, h));
    }
    let longest = w.max(h);
    if longest == max_dim {
        return Ok((rgba, w, h));
    }
    let out_w = ((w as u64 * max_dim as u64 / longest as u64) as u32).max(1);
    let out_h = ((h as u64 * max_dim as u64 / longest as u64) as u32).max(1);
    let img = image::RgbaImage::from_raw(w, h, rgba)
        .ok_or_else(|| "rgba buffer shape mismatch".to_string())?;
    let filter = if out_w < w || out_h < h {
        FilterType::Lanczos3
    } else {
        FilterType::Triangle
    };
    let resized = image::imageops::resize(&img, out_w, out_h, filter);
    Ok((resized.into_raw(), out_w, out_h))
}

/// Shell out to `./djxl` to decode `bytes` to a PAM, parsed back to RGBA8 at
/// native resolution. Single-threaded — we parallelise at the render level.
fn decode_via_djxl(bytes: &[u8]) -> Result<(Vec<u8>, u32, u32), String> {
    if !std::path::Path::new(DJXL_BIN).exists() {
        return Err("djxl binary not found. Run 'make setup' to build it.".to_string());
    }

    let id: u64 = rand::thread_rng().gen();
    let tmp = std::env::temp_dir();
    let input_path = tmp.join(format!("artxl_dec_{}.jxl", id));
    let output_path = tmp.join(format!("artxl_dec_{}.pam", id));

    std::fs::write(&input_path, bytes).map_err(|e| format!("write temp jxl: {}", e))?;

    let mut child = Command::new(DJXL_BIN)
        .arg("--num_threads=1")
        .arg(&input_path)
        .arg(&output_path)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| {
            let _ = std::fs::remove_file(&input_path);
            format!("launch djxl: {}", e)
        })?;

    let timeout = decode_timeout();
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
                    return Err(format!("djxl timed out after {}s", timeout.as_secs()));
                }
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(e) => {
                let _ = std::fs::remove_file(&input_path);
                let _ = std::fs::remove_file(&output_path);
                return Err(format!("djxl wait: {}", e));
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
            format!("djxl exited with {}", status)
        } else {
            format!("djxl exited with {}: {}", status, stderr)
        });
    }

    let pam = std::fs::read(&output_path).map_err(|e| format!("read pam: {}", e))?;
    let _ = std::fs::remove_file(&output_path);
    parse_pam(&pam)
}

/// Parse a binary PAM (P7) into RGBA8.
///
/// - Bit depth: MAXVAL 255 → 1 byte/sample; higher (e.g. 4095 for 12-bit) →
///   2 bytes/sample big-endian. Samples are scaled to 8-bit by `v*255/MAXVAL`
///   (djxl writes MAXVAL = 2^bitdepth-1, not a left-justified 16-bit value).
/// - Channels: the **first** `TUPLTYPE` gives the displayable colour layout
///   (GRAYSCALE / GRAYSCALE_ALPHA / RGB / RGB_ALPHA). Images with hidden/extra
///   channels (`HiddenChannel`) come back with `DEPTH` > those colour channels
///   and extra `TUPLTYPE Optional` lines; we read the leading colour samples at
///   a full-`DEPTH` stride and ignore the extras (matching the displayed image).
fn parse_pam(b: &[u8]) -> Result<(Vec<u8>, u32, u32), String> {
    let marker = b"ENDHDR\n";
    let pos = b
        .windows(marker.len())
        .position(|w| w == marker)
        .ok_or("PAM: missing ENDHDR")?;
    let header = std::str::from_utf8(&b[..pos]).map_err(|_| "PAM: non-utf8 header")?;

    let (mut w, mut h, mut depth, mut maxval) = (0u32, 0u32, 0usize, 0u32);
    let mut tupltype: Option<&str> = None;
    let mut it = header.split_whitespace();
    while let Some(tok) = it.next() {
        match tok {
            "WIDTH" => w = it.next().and_then(|v| v.parse().ok()).ok_or("PAM: WIDTH")?,
            "HEIGHT" => {
                h = it
                    .next()
                    .and_then(|v| v.parse().ok())
                    .ok_or("PAM: HEIGHT")?
            }
            "DEPTH" => depth = it.next().and_then(|v| v.parse().ok()).ok_or("PAM: DEPTH")?,
            "MAXVAL" => {
                maxval = it
                    .next()
                    .and_then(|v| v.parse().ok())
                    .ok_or("PAM: MAXVAL")?
            }
            // Keep only the first TUPLTYPE (the colour layout); later ones are
            // "Optional" extra-channel markers.
            "TUPLTYPE" => {
                let v = it.next();
                if tupltype.is_none() {
                    tupltype = v;
                }
            }
            _ => {}
        }
    }
    if w == 0 || h == 0 || depth == 0 {
        return Err("PAM: incomplete header".to_string());
    }

    // Leading colour channels to read (the rest of each pixel's samples are
    // hidden/extra channels we skip). Fall back to DEPTH when no TUPLTYPE.
    let color = match tupltype {
        Some("GRAYSCALE") => 1,
        Some("GRAYSCALE_ALPHA") => 2,
        Some("RGB") => 3,
        Some("RGB_ALPHA") => 4,
        _ => depth.min(4),
    }
    .min(depth);

    let maxval = maxval.max(1);
    let bps = if maxval > 255 { 2usize } else { 1 };
    let data = &b[pos + marker.len()..];
    let px = (w as usize) * (h as usize);
    if data.len() < px * depth * bps {
        return Err("PAM: truncated pixel data".to_string());
    }

    // Read the i-th interleaved sample (over all DEPTH channels), scaled to 8-bit.
    let sample = |i: usize| -> u8 {
        let v = if bps == 2 {
            ((data[i * 2] as u32) << 8) | data[i * 2 + 1] as u32
        } else {
            data[i] as u32
        };
        (v * 255 / maxval) as u8
    };

    let mut out = Vec::with_capacity(px * 4);
    for p in 0..px {
        let base = p * depth; // stride over the full channel count
        let px4 = match color {
            1 => {
                let v = sample(base);
                [v, v, v, 255]
            }
            2 => {
                let v = sample(base);
                [v, v, v, sample(base + 1)]
            }
            3 => [sample(base), sample(base + 1), sample(base + 2), 255],
            _ => [
                sample(base),
                sample(base + 1),
                sample(base + 2),
                sample(base + 3),
            ],
        };
        out.extend_from_slice(&px4);
    }
    Ok((out, w, h))
}
