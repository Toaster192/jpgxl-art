// Gallery encode round-trip: every gallery file → jxl_from_tree → JXL bytes
// → jxl-oxide decode → RGBA. Gated with #[ignore] because it shells out 222
// times and needs ./jxl_from_tree built. Run with:
//
//     cargo test --test gallery_encode -- --ignored --nocapture
//
// from the project root.

use artxl::{codec, render};
use std::fs;
use std::path::PathBuf;

fn gallery_files() -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = fs::read_dir("gallery")
        .expect("gallery/ should exist when running tests from repo root")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("jxlart"))
        .collect();
    paths.sort();
    paths
}

#[test]
#[ignore]
fn every_gallery_entry_encodes_and_decodes() {
    if !std::path::Path::new("./jxl_from_tree").exists() {
        panic!("./jxl_from_tree not present — run `make setup` before running this test");
    }

    let entries = gallery_files();
    assert!(!entries.is_empty(), "no .jxlart files found under gallery/");

    let mut failures: Vec<String> = Vec::new();
    for path in &entries {
        let text = match fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) => {
                failures.push(format!("{}: read: {}", path.display(), e));
                continue;
            }
        };

        let bytes = match render::encode_jxl_from_tree(&text) {
            Ok(b) => b,
            Err(e) => {
                failures.push(format!("{}: encode: {}", path.display(), e));
                continue;
            }
        };
        if bytes.is_empty() {
            failures.push(format!("{}: empty encode output", path.display()));
            continue;
        }

        match codec::decode_jxl(&bytes, 0) {
            Ok((rgba, w, h)) => {
                if w == 0 || h == 0 {
                    failures.push(format!("{}: decode dim {}×{}", path.display(), w, h));
                    continue;
                }
                if rgba.len() != (w as usize) * (h as usize) * 4 {
                    failures.push(format!(
                        "{}: rgba buffer size {} != {}×{}×4",
                        path.display(),
                        rgba.len(),
                        w,
                        h,
                    ));
                }
            }
            Err(e) => failures.push(format!("{}: decode: {}", path.display(), e)),
        }
    }

    assert!(
        failures.is_empty(),
        "gallery encode round-trip failures ({}/{}):\n{}",
        failures.len(),
        entries.len(),
        failures.join("\n"),
    );
}
