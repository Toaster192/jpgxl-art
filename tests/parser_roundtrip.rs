use artxl::tree::ImageProgram;
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
fn parser_roundtrips_every_gallery_entry() {
    let entries = gallery_files();
    assert!(!entries.is_empty(), "no .jxlart files found under gallery/");

    let mut failures: Vec<String> = Vec::new();
    for path in &entries {
        let text = fs::read_to_string(path).expect("read gallery file");
        let prog1 = match ImageProgram::from_text(&text) {
            Ok(p) => p,
            Err(e) => {
                failures.push(format!("{}: parse: {}", path.display(), e));
                continue;
            }
        };
        let text2 = prog1.to_text();
        let prog2 = match ImageProgram::from_text(&text2) {
            Ok(p) => p,
            Err(e) => {
                failures.push(format!("{}: re-parse: {}", path.display(), e));
                continue;
            }
        };
        if prog1 != prog2 {
            failures.push(format!("{}: round-trip differs", path.display()));
        }
    }

    assert!(
        failures.is_empty(),
        "parser round-trip failures ({}/{}):\n{}",
        failures.len(),
        entries.len(),
        failures.join("\n"),
    );
}

#[test]
fn to_text_is_idempotent_after_one_round() {
    // text → prog → text2 → prog2 → text3, assert text2 == text3.
    // Catches ordering / whitespace drift in to_text even when the AST
    // is stable.
    let entries = gallery_files();
    let mut failures: Vec<String> = Vec::new();
    for path in &entries {
        let text = fs::read_to_string(path).unwrap();
        let Ok(prog1) = ImageProgram::from_text(&text) else {
            continue;
        };
        let text2 = prog1.to_text();
        let Ok(prog2) = ImageProgram::from_text(&text2) else {
            continue;
        };
        let text3 = prog2.to_text();
        if text2 != text3 {
            failures.push(path.display().to_string());
        }
    }
    assert!(
        failures.is_empty(),
        "to_text not idempotent for: {}",
        failures.join(", "),
    );
}
