use image::imageops::FilterType;
use jxl_oxide::JxlImage;

/// Decode a JXL byte stream into RGBA8, optionally resized so the longest
/// edge matches `max_dim`. `max_dim == 0` means render at the JXL's native
/// dimensions.
pub fn decode_jxl(bytes: &[u8], max_dim: u32) -> Result<(Vec<u8>, u32, u32), String> {
    let image = JxlImage::builder()
        .read(bytes)
        .map_err(|e| format!("jxl open: {}", e))?;

    let render = image
        .render_frame(0)
        .map_err(|e| format!("jxl render: {}", e))?;

    let mut stream = render.stream();
    let w = stream.width();
    let h = stream.height();
    let ch = stream.channels() as usize;
    let pixel_count = (w as usize) * (h as usize);

    let mut raw = vec![0u8; pixel_count * ch];
    stream.write_to_buffer(&mut raw);

    let rgba = to_rgba8(&raw, ch, pixel_count)?;

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

fn to_rgba8(raw: &[u8], ch: usize, pixels: usize) -> Result<Vec<u8>, String> {
    match ch {
        4 => Ok(raw.to_vec()),
        3 => {
            let mut out = Vec::with_capacity(pixels * 4);
            for chunk in raw.chunks_exact(3) {
                out.extend_from_slice(chunk);
                out.push(255);
            }
            Ok(out)
        }
        2 => {
            let mut out = Vec::with_capacity(pixels * 4);
            for chunk in raw.chunks_exact(2) {
                let (v, a) = (chunk[0], chunk[1]);
                out.extend_from_slice(&[v, v, v, a]);
            }
            Ok(out)
        }
        1 => {
            let mut out = Vec::with_capacity(pixels * 4);
            for &v in raw {
                out.extend_from_slice(&[v, v, v, 255]);
            }
            Ok(out)
        }
        other => Err(format!("unsupported JXL channel count: {}", other)),
    }
}
