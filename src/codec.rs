use anyhow::{Context, Result};
use image::imageops::FilterType;

/// Decode a JXL file from raw bytes into a 1024×1024 RGBA u8 buffer.
pub fn decode_jxl_to_rgba(data: &[u8]) -> Result<(u32, u32, Vec<u8>)> {
    use jpegxl_rs::{
        Endianness,
        decode::{Pixels, PixelFormat},
        decoder_builder,
    };

    let decoder = decoder_builder()
        .pixel_format(PixelFormat {
            num_channels: 4,
            endianness: Endianness::Native,
            align: 0,
        })
        .build()
        .context("failed to build JXL decoder")?;

    let (meta, pixels) = decoder
        .decode(data)
        .context("failed to decode JXL data")?;

    let raw: Vec<u8> = match pixels {
        Pixels::Uint8(v) => v,
        Pixels::Float(v) => v.iter().map(|&f| (f * 255.0).clamp(0.0, 255.0) as u8).collect(),
        Pixels::Uint16(v) => v.iter().map(|&u| (u >> 8) as u8).collect(),
        Pixels::Float16(v) => v
            .iter()
            .map(|&f| (f32::from(f) * 255.0).clamp(0.0, 255.0) as u8)
            .collect(),
    };

    let (src_w, src_h) = (meta.width, meta.height);
    let (dst_w, dst_h) = (1024u32, 1024u32);

    if src_w == dst_w && src_h == dst_h {
        return Ok((dst_w, dst_h, raw));
    }

    // Resize to 1024×1024 using the `image` crate.
    let img = image::RgbaImage::from_raw(src_w, src_h, raw)
        .context("failed to create RgbaImage from decoded pixels")?;
    let resized = image::imageops::resize(&img, dst_w, dst_h, FilterType::Lanczos3);
    Ok((dst_w, dst_h, resized.into_raw()))
}
