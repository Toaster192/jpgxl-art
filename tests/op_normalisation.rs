// jxl_from_tree only accepts `>`. The `Op` enum reflects that with a single
// `Gt` variant, and `from_text` rejects any other operator. These tests pin
// that behaviour.

use artxl::tree::ImageProgram;

#[test]
fn parser_rejects_non_gt_operators() {
    for op in ["<", ">=", "<=", "==", "!=", "=>"] {
        let text = format!("Bitdepth 8\n\nif x {} 100\n  - Set 0\n  - Set 255\n", op);
        let result = ImageProgram::from_text(&text);
        assert!(result.is_err(), "parser unexpectedly accepted op '{}'", op,);
        let err = result.unwrap_err();
        assert!(
            err.contains("'>'") || err.contains("operator"),
            "error for op '{}' should reference '>': got: {}",
            op,
            err,
        );
    }
}

#[test]
fn parser_accepts_gt() {
    let text = "Bitdepth 8\n\nif x > 100\n  - Set 0\n  - Set 255\n";
    let prog = ImageProgram::from_text(text).expect("plain `>` program should parse");
    let out = prog.to_text();
    assert!(out.contains("x > 100"), "to_text should emit `>`: {}", out);
}

#[test]
fn to_text_only_emits_gt_for_every_gallery_entry() {
    let entries: Vec<_> = std::fs::read_dir("gallery")
        .expect("gallery/")
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("jxlart"))
        .collect();

    let mut failures: Vec<String> = Vec::new();
    for path in &entries {
        let text = std::fs::read_to_string(path).unwrap();
        let Ok(prog) = ImageProgram::from_text(&text) else {
            continue;
        };
        let out = prog.to_text();
        // Body lines start with "if " — every condition line must contain `>`
        // and must not contain `<`, `>=`, `<=`, `==`, `!=` between the `if`
        // keyword and the threshold.
        for line in out.lines() {
            let trimmed = line.trim();
            if let Some(rest) = trimmed.strip_prefix("if ") {
                if !rest.contains(" > ") {
                    failures.push(format!(
                        "{}: condition line missing ` > `: `{}`",
                        path.display(),
                        line,
                    ));
                }
                for bad in [" < ", " >= ", " <= ", " == ", " != "] {
                    if rest.contains(bad) {
                        failures.push(format!(
                            "{}: condition line contains `{}`: `{}`",
                            path.display(),
                            bad.trim(),
                            line,
                        ));
                    }
                }
            }
        }
    }
    assert!(
        failures.is_empty(),
        "to_text emitted non-`>` operators:\n{}",
        failures.join("\n"),
    );
}
