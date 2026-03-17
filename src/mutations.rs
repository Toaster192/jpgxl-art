use rand::Rng;
use serde::Serialize;

use crate::tree::{Condition, ImageProgram, Node, Op, Predictor, Var};

/// All the ways the tree can be structurally mutated.
/// Every mutation produces output that is valid jxl-art syntax (only `>` in
/// conditions, correct predictor offset format).
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "params")]
pub enum Mutation {
    /// Shift a threshold value by ±delta.
    TweakThreshold { delta: i64 },
    /// Negate the first condition's threshold  (`prop > T` → `prop > -T`).
    /// Useful for W/N/WGH which can be negative; turns a tight guard loose or
    /// vice-versa.
    NegateThreshold,
    /// Swap the variable in a condition (e.g. `y` → `x`).
    SwapConditionVar,
    /// Change a `Set` predictor's absolute value by ±delta.
    TweakSetValue { delta: i64 },
    /// Replace a predictor with a randomly chosen neighbour-based one.
    SwapPredictor,
    /// Insert a new `If` node at the top level.
    AddBranch,
    /// Remove the first `If` node, promoting its body up one level.
    RemoveBranch,
}

impl Mutation {
    pub fn label(&self) -> String {
        match self {
            Mutation::TweakThreshold { delta } => format!("Threshold {:+}", delta),
            Mutation::NegateThreshold => "Negate threshold".into(),
            Mutation::SwapConditionVar => "Swap var".into(),
            Mutation::TweakSetValue { delta } => format!("Set value {:+}", delta),
            Mutation::SwapPredictor => "Swap predictor".into(),
            Mutation::AddBranch => "Add branch".into(),
            Mutation::RemoveBranch => "Remove branch".into(),
        }
    }

    /// A curated set that exercises every mutation kind.
    pub fn showcase() -> Vec<Mutation> {
        vec![
            Mutation::TweakThreshold { delta: 50 },
            Mutation::TweakThreshold { delta: -50 },
            Mutation::NegateThreshold,
            Mutation::SwapConditionVar,
            Mutation::TweakSetValue { delta: 30 },
            Mutation::TweakSetValue { delta: -30 },
            Mutation::SwapPredictor,
            Mutation::AddBranch,
            Mutation::RemoveBranch,
        ]
    }

    pub fn apply(&self, program: &ImageProgram) -> ImageProgram {
        let mut rng = rand::thread_rng();
        let mut prog = program.clone();
        match self {
            Mutation::TweakThreshold { delta } => {
                mutate_first_condition(&mut prog.nodes, &mut |c| c.threshold += delta);
            }
            Mutation::NegateThreshold => {
                mutate_first_condition(&mut prog.nodes, &mut |c| c.threshold = -c.threshold);
            }
            Mutation::SwapConditionVar => {
                let vars = [Var::X, Var::Y, Var::C, Var::W, Var::N, Var::WGH];
                let pick = vars[rng.gen_range(0..vars.len())].clone();
                mutate_first_condition(&mut prog.nodes, &mut |c| c.var = pick.clone());
            }
            Mutation::TweakSetValue { delta } => {
                mutate_first_predictor(&mut prog.nodes, &mut |p| {
                    if let Predictor::Set(v) = p {
                        *v += delta;
                    }
                });
            }
            Mutation::SwapPredictor => {
                let replacement = random_predictor(&mut rng);
                mutate_first_predictor(&mut prog.nodes, &mut |p| *p = replacement.clone());
            }
            Mutation::AddBranch => {
                let new_if = Node::If {
                    condition: Condition {
                        var: random_var(&mut rng),
                        op: Op::Gt,
                        threshold: rng.gen_range(0..=255),
                    },
                    body: vec![Node::Predict(Predictor::Set(rng.gen_range(0i64..=255)))],
                };
                prog.nodes.push(new_if);
            }
            Mutation::RemoveBranch => {
                remove_first_if(&mut prog.nodes);
            }
        }
        prog
    }
}

/// Check whether a rendered RGBA buffer is visually degenerate (essentially a
/// single flat colour — no meaningful content).
pub fn is_degenerate(rgba: &[u8]) -> bool {
    if rgba.len() < 4 {
        return true;
    }
    let (mut min_r, mut max_r) = (255u8, 0u8);
    let (mut min_g, mut max_g) = (255u8, 0u8);
    let (mut min_b, mut max_b) = (255u8, 0u8);
    for px in rgba.chunks_exact(4) {
        min_r = min_r.min(px[0]); max_r = max_r.max(px[0]);
        min_g = min_g.min(px[1]); max_g = max_g.max(px[1]);
        min_b = min_b.min(px[2]); max_b = max_b.max(px[2]);
    }
    let range = (max_r - min_r) as u16 + (max_g - min_g) as u16 + (max_b - min_b) as u16;
    range < 10
}

// ── Helpers ─────────────────────────────────────────────────────────────────

fn random_var(rng: &mut impl Rng) -> Var {
    match rng.gen_range(0..6u8) {
        0 => Var::X,
        1 => Var::Y,
        2 => Var::C,
        3 => Var::W,
        4 => Var::N,
        _ => Var::WGH,
    }
}

fn random_predictor(rng: &mut impl Rng) -> Predictor {
    let offset = rng.gen_range(-32i64..=32);
    match rng.gen_range(0..7u8) {
        0 => Predictor::Set(rng.gen_range(0i64..=255)),
        1 => Predictor::N(offset),
        2 => Predictor::W(offset),
        3 => Predictor::AvgNNW(offset),
        4 => Predictor::AvgNNE(offset),
        5 => Predictor::AvgWNW(offset),
        _ => Predictor::Weighted(offset),
    }
}

/// Walk the tree depth-first and apply `f` to the first `Condition` found.
fn mutate_first_condition(nodes: &mut Vec<Node>, f: &mut impl FnMut(&mut Condition)) -> bool {
    for node in nodes.iter_mut() {
        if let Node::If { condition, .. } = node {
            f(condition);
            return true;
        }
    }
    for node in nodes.iter_mut() {
        if let Node::If { body, .. } = node {
            if mutate_first_condition(body, f) {
                return true;
            }
        }
    }
    false
}

/// Walk the tree depth-first and apply `f` to the first `Predict` leaf found.
fn mutate_first_predictor(nodes: &mut Vec<Node>, f: &mut impl FnMut(&mut Predictor)) -> bool {
    for node in nodes.iter_mut() {
        match node {
            Node::Predict(pred) => {
                f(pred);
                return true;
            }
            Node::If { body, .. } => {
                if mutate_first_predictor(body, f) {
                    return true;
                }
            }
        }
    }
    false
}

/// Remove the first `If` node at the top level, splicing its body in-place.
fn remove_first_if(nodes: &mut Vec<Node>) {
    let pos = nodes.iter().position(|n| matches!(n, Node::If { .. }));
    if let Some(i) = pos {
        if let Node::If { body, .. } = nodes.remove(i) {
            for (offset, child) in body.into_iter().enumerate() {
                nodes.insert(i + offset, child);
            }
        }
    }
}
