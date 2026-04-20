use std::process::Command;

use rand::Rng;

use crate::codec;

/// Encode `program_text` with `./jxl_from_tree`, decode the resulting JXL
/// bytes via `jxl-oxide`, and return the rendered RGBA8 buffer along with
/// its dimensions.
///
/// `size == 0` → render at the JXL's native dimensions.
/// Any other value → longest edge scaled to `size` px (Lanczos3 / Triangle).
pub fn render_roundtrip(program_text: &str, size: u32) -> Result<(Vec<u8>, u32, u32), String> {
    let jxl = encode_jxl_from_tree(program_text)?;
    codec::decode_jxl(&jxl, size)
}

/// Shell out to `./jxl_from_tree` with the given program text and return
/// the generated JXL bytes.
pub fn encode_jxl_from_tree(program_text: &str) -> Result<Vec<u8>, String> {
    if !std::path::Path::new("./jxl_from_tree").exists() {
        return Err(
            "jxl_from_tree binary not found. Run 'make setup' to build it.".to_string()
        );
    }

    let id: u64 = rand::thread_rng().gen();
    let tmp = std::env::temp_dir();
    let input_path  = tmp.join(format!("artxl_{}.xl",  id));
    let output_path = tmp.join(format!("artxl_{}.jxl", id));

    std::fs::write(&input_path, program_text)
        .map_err(|e| format!("write temp input: {}", e))?;

    let status = Command::new("./jxl_from_tree")
        .arg(&input_path)
        .arg(&output_path)
        .status()
        .map_err(|e| format!("launch jxl_from_tree: {}", e))?;

    let _ = std::fs::remove_file(&input_path);

    if !status.success() {
        let _ = std::fs::remove_file(&output_path);
        return Err(format!("jxl_from_tree exited with {}", status));
    }

    let bytes = std::fs::read(&output_path)
        .map_err(|e| format!("read jxl output: {}", e))?;
    let _ = std::fs::remove_file(&output_path);

    Ok(bytes)
}
