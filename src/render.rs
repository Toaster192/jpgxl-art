use crate::codec;
use crate::proc;

/// Encode `program_text` with `./jxl_from_tree`, decode the resulting JXL
/// bytes via `./djxl`, and return the rendered RGBA8 buffer, its
/// dimensions, and the encoded JXL bytes themselves (so callers can surface
/// the file size — via `.len()` — or ship the JXL straight to a capable
/// browser without re-invoking `jxl_from_tree`).
///
/// `size == 0` → render at the JXL's native dimensions.
/// Any other value → longest edge scaled to `size` px (Lanczos3 / Triangle).
pub fn render_roundtrip(
    program_text: &str,
    size: u32,
) -> Result<(Vec<u8>, u32, u32, Vec<u8>), String> {
    let jxl = encode_jxl_from_tree(program_text)?;
    let (rgba, w, h) = codec::decode_jxl(&jxl, size)?;
    Ok((rgba, w, h, jxl))
}

/// Shell out to `./jxl_from_tree` with the given program text and return
/// the generated JXL bytes.
pub fn encode_jxl_from_tree(program_text: &str) -> Result<Vec<u8>, String> {
    proc::run_with_temp_files("./jxl_from_tree", &[], program_text.as_bytes(), "xl", "jxl")
}
