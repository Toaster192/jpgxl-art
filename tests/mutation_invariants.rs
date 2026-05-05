use artxl::mutations::{random_compounds, Mutation};
use artxl::tree::{ImageProgram, Node, Op};
use std::panic;

fn seed_programs() -> Vec<(String, ImageProgram)> {
    // Default + a few deliberately-shaped gallery entries spanning headers,
    // splines, and exotic predictors.
    let names = [
        "00-sky-and-grass.jxlart",
        "11-luca-noise-xyb.jxlart",
        "27-luca-ne-avgwn.jxlart",
        "31-surma-squeeze-rct16.jxlart",
        "32-surma-splines.jxlart",
        "36-simple-gradient.jxlart",
    ];
    names
        .iter()
        .map(|n| {
            let path = format!("gallery/{}", n);
            let text = std::fs::read_to_string(&path)
                .unwrap_or_else(|_| panic!("missing seed program: {}", path));
            let prog = ImageProgram::from_text(&text)
                .unwrap_or_else(|e| panic!("seed {} unparseable: {}", n, e));
            (n.to_string(), prog)
        })
        .collect()
}

fn assert_invariants(node: &Node) -> Result<(), String> {
    match node {
        Node::If {
            condition,
            on_true,
            on_false,
        } => {
            // Op only has Gt today, but assert anyway so this catches a future
            // re-introduction of <, >=, etc. without test updates.
            if condition.op != Op::Gt {
                return Err(format!("non-Gt op found: {:?}", condition.op));
            }
            assert_invariants(on_true)?;
            assert_invariants(on_false)?;
            Ok(())
        }
        Node::Predict(_) => Ok(()),
    }
}

#[test]
fn showcase_mutations_preserve_invariants() {
    const TRIALS: usize = 25;
    let seeds = seed_programs();
    let mut failures: Vec<String> = Vec::new();

    for mutation in Mutation::showcase() {
        for (seed_name, baseline) in &seeds {
            for trial in 0..TRIALS {
                let result =
                    panic::catch_unwind(panic::AssertUnwindSafe(|| mutation.apply(baseline)));
                let mutated = match result {
                    Ok(p) => p,
                    Err(_) => {
                        failures.push(format!(
                            "{} on {} trial {}: panic during apply",
                            mutation.label(),
                            seed_name,
                            trial,
                        ));
                        continue;
                    }
                };

                if let Err(e) = assert_invariants(&mutated.root) {
                    failures.push(format!(
                        "{} on {} trial {}: invariant violation: {}",
                        mutation.label(),
                        seed_name,
                        trial,
                        e,
                    ));
                    continue;
                }

                let text = mutated.to_text();
                if let Err(e) = ImageProgram::from_text(&text) {
                    failures.push(format!(
                        "{} on {} trial {}: re-parse failed: {}",
                        mutation.label(),
                        seed_name,
                        trial,
                        e,
                    ));
                }
            }
        }
    }

    assert!(
        failures.is_empty(),
        "mutation invariant failures ({}):\n{}",
        failures.len(),
        failures.join("\n"),
    );
}

#[test]
fn random_compounds_preserve_invariants() {
    let seeds = seed_programs();
    let compounds = random_compounds(40);
    let mut failures: Vec<String> = Vec::new();

    for (i, mutation) in compounds.iter().enumerate() {
        for (seed_name, baseline) in &seeds {
            let result = panic::catch_unwind(panic::AssertUnwindSafe(|| mutation.apply(baseline)));
            let mutated = match result {
                Ok(p) => p,
                Err(_) => {
                    failures.push(format!("compound #{} on {}: panic", i, seed_name));
                    continue;
                }
            };
            if let Err(e) = assert_invariants(&mutated.root) {
                failures.push(format!(
                    "compound #{} ({}) on {}: invariant violation: {}",
                    i,
                    mutation.label(),
                    seed_name,
                    e,
                ));
                continue;
            }
            let text = mutated.to_text();
            if let Err(e) = ImageProgram::from_text(&text) {
                failures.push(format!(
                    "compound #{} ({}) on {}: re-parse: {}",
                    i,
                    mutation.label(),
                    seed_name,
                    e,
                ));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "random compound invariant failures:\n{}",
        failures.join("\n"),
    );
}
