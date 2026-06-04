use artxl::mutations::{random_compounds, random_program, Complexity, Mutation};
use artxl::tree::{ImageProgram, Node, Op};
use std::collections::HashMap;
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

/// Walk the tree tracking each property's [low, high] bound from ancestor
/// conditions on the same property; return Err on any condition that is
/// always-true or always-false (a "dead" nested same-property comparison).
/// libjxl's djxl decoder rejects a JXL containing one, so `simplify_degenerate`
/// must have removed them from every generated/mutated program.
fn assert_no_dead(node: &Node, bounds: &mut HashMap<String, (i64, i64)>) -> Result<(), String> {
    if let Node::If {
        condition,
        on_true,
        on_false,
    } = node
    {
        let key = condition.var.label().to_string();
        let t = condition.threshold;
        let orig = bounds.get(&key).copied();
        let (low, high) = orig.unwrap_or((i64::MIN, i64::MAX));
        if low > t {
            return Err(format!("always-true `{} > {}` (low={})", key, t, low));
        }
        if high <= t {
            return Err(format!("always-false `{} > {}` (high={})", key, t, high));
        }
        bounds.insert(key.clone(), (low.max(t.saturating_add(1)), high));
        assert_no_dead(on_true, bounds)?;
        bounds.insert(key.clone(), (low, high.min(t)));
        assert_no_dead(on_false, bounds)?;
        match orig {
            Some(v) => {
                bounds.insert(key, v);
            }
            None => {
                bounds.remove(&key);
            }
        }
    }
    Ok(())
}

/// Generated programs (and seed programs after a mutation) must contain no dead
/// nested same-property conditions — those make a JXL djxl can't decode.
#[test]
fn no_dead_conditions_after_generation_or_mutation() {
    let mut failures = Vec::new();

    for complexity in [Complexity::Simple, Complexity::Normal, Complexity::Complex] {
        for dims in [None, Some((1024, 576)), Some((576, 1024))] {
            for trial in 0..60 {
                let prog = random_program(complexity, dims);
                if let Err(e) = assert_no_dead(&prog.root, &mut HashMap::new()) {
                    failures.push(format!(
                        "gen {:?} dims={:?} trial {}: {}\n{}",
                        complexity,
                        dims,
                        trial,
                        e,
                        prog.to_text()
                    ));
                }
            }
        }
    }

    // Mutations call simplify_degenerate too, so their output must be clean
    // regardless of the seed.
    let seeds = seed_programs();
    let mut muts = Mutation::showcase();
    muts.extend(random_compounds(20));
    for m in &muts {
        for (name, baseline) in &seeds {
            let out = m.apply(baseline);
            if let Err(e) = assert_no_dead(&out.root, &mut HashMap::new()) {
                failures.push(format!("mut {} on {}: {}", m.label(), name, e));
            }
        }
    }

    assert!(
        failures.is_empty(),
        "dead conditions found ({}):\n{}",
        failures.len(),
        failures.join("\n"),
    );
}
