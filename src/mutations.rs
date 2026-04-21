use rand::Rng;
use serde::Serialize;

use crate::render;
use crate::tree::{Condition, ImageProgram, Node, Op, Predictor, Var};

// ── Random program generation ─────────────────────────────────────────────────

pub fn random_program() -> ImageProgram {
    let mut rng = rand::thread_rng();
    // Force a channel split at the root so RCT-6 (YCoCg inverse) actually
    // produces varied colour. Without this, trees that never condition on
    // `c` emit identical Y/Co/Cg values for all channels, and the inverse
    // transform of (V,V,V) is always yellow-green — the single biggest
    // source of colour bias in random output.
    let c_threshold: i64 = if rng.gen_bool(0.5) { 0 } else { 1 };
    let root = Node::If {
        condition: Condition { var: Var::C, op: Op::Gt, threshold: c_threshold },
        on_true:  Box::new(random_node(&mut rng, 1)),
        on_false: Box::new(random_node(&mut rng, 1)),
    };
    ImageProgram {
        width: 1024,
        height: 1024,
        bitdepth: 8,
        channels: 3,
        orientation: Some(rng.gen_range(1u32..=8)),
        rct: Some(6),
        extra_headers: Vec::new(),
        splines: Vec::new(),
        root,
    }
}

/// Generate a random program whose preview is not degenerate
/// (single-colour / flat). Falls through after `MAX_TRIES` attempts so the
/// caller is never blocked. Uses the roundtrip renderer at 64 px so the
/// check is accurate to what libjxl will actually produce.
pub fn random_program_non_degenerate() -> ImageProgram {
    const MAX_TRIES: usize = 5;
    let mut prog = random_program();
    for _ in 0..MAX_TRIES {
        let text = prog.to_text();
        if let Ok((rgba, _, _)) = render::render_roundtrip(&text, 64) {
            if !is_degenerate(&rgba) {
                return prog;
            }
        }
        prog = random_program();
    }
    prog
}

fn random_node(rng: &mut impl Rng, depth: usize) -> Node {
    // Branch probability falls off with depth; always a leaf at depth 5.
    let branch_prob = [0.95, 0.85, 0.70, 0.50, 0.25].get(depth).copied().unwrap_or(0.0);
    if rng.gen::<f64>() < branch_prob {
        // Pick threshold range appropriate to the variable.
        let var = random_var(rng);
        let threshold = match var {
            Var::X | Var::Y    => rng.gen_range(50i64..=950),
            Var::C             => rng.gen_range(0i64..=2),
            Var::W | Var::N    => rng.gen_range(-100i64..=300),
            Var::WGH           => rng.gen_range(0i64..=20),
            // `random_var` never returns Other, but keep the match total.
            Var::Other(_)      => rng.gen_range(0i64..=255),
        };
        Node::If {
            condition: Condition { var, op: Op::Gt, threshold },
            on_true:  Box::new(random_node(rng, depth + 1)),
            on_false: Box::new(random_node(rng, depth + 1)),
        }
    } else {
        Node::Predict(random_predictor(rng))
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "params")]
pub enum Mutation {
    // ── Single mutations ──────────────────────────────────────────────────────
    /// Nudge a randomly-chosen condition's threshold by ±scale of its current value.
    TweakThreshold { scale: f64 },
    /// Negate a randomly-chosen condition's threshold.
    NegateThreshold,
    /// Replace the variable in a randomly-chosen condition with a random one.
    SwapConditionVar,
    /// Swap on_true / on_false of a randomly-chosen If node.
    SwapBranches,
    /// Nudge a randomly-chosen Set predictor's value by ±scale of its current value.
    TweakSetValue { scale: f64 },
    /// Replace a randomly-chosen predictor leaf with a random neighbour-based one.
    SwapPredictor,
    /// Shift every predictor offset by ±scale of the average offset magnitude.
    TweakAllOffsets { scale: f64 },
    // ── Structural mutations ──────────────────────────────────────────────────
    /// Wrap the tree in a new random If (old tree becomes on_false).
    AddBranch,
    /// Replace root with its FALSE child.
    RemoveBranch,
    /// Replace root with its TRUE child.
    PromoteTrueBranch,
    /// Split a random Predict leaf into a new If with two leaves.
    /// Complements root-only `AddBranch`.
    InsertIfAtLeaf,
    /// Replace a random sub-tree (any node) with a freshly-generated random
    /// subtree. More disruptive than `SwapPredictor`.
    ReplaceSubtreeRandom,
    // ── Program-level (headers / colour transform) ────────────────────────────
    /// Pick a different RCT (reversible colour transform) preset.
    /// Huge visual impact — re-interprets channel values as a different
    /// colour space.
    CycleRct,
    /// Flip one gallery-relevant header flag in `extra_headers`
    /// (Gaborish / XYB / DeltaPalette / Squeeze).
    ToggleHeader,
    // ── Exotic predictor (newly-reachable via the tolerant parser) ────────────
    /// Replace a random Predict leaf with an exotic predictor:
    /// NE / NW / NN / WW / NWW / AvgW+N / AvgAll / Gradient / Select.
    SwapPredictorExotic,
    // ── Compound: apply multiple mutations in sequence ────────────────────────
    Chain(Vec<Mutation>),
}

/// Generate `n` random compound mutations, each a chain of 2–4 simple mutations.
pub fn random_compounds(n: usize) -> Vec<Mutation> {
    let mut rng = rand::thread_rng();
    (0..n)
        .map(|_| {
            let len = rng.gen_range(2..=4usize);
            let steps = (0..len).map(|_| random_simple_mutation(&mut rng)).collect();
            Mutation::Chain(steps)
        })
        .collect()
}

fn random_simple_mutation(rng: &mut impl Rng) -> Mutation {
    let mag: f64 = rng.gen_range(0.10..=0.50);
    let scale = if rng.gen_bool(0.5) { mag } else { -mag };
    match rng.gen_range(0..12u8) {
        0  => Mutation::TweakThreshold { scale },
        1  => Mutation::NegateThreshold,
        2  => Mutation::SwapConditionVar,
        3  => Mutation::SwapBranches,
        4  => Mutation::TweakSetValue { scale },
        5  => Mutation::SwapPredictor,
        6  => Mutation::TweakAllOffsets { scale },
        7  => Mutation::CycleRct,
        8  => Mutation::ToggleHeader,
        9  => Mutation::SwapPredictorExotic,
        10 => Mutation::InsertIfAtLeaf,
        _  => Mutation::ReplaceSubtreeRandom,
    }
}

impl Mutation {
    pub fn is_compound(&self) -> bool {
        matches!(self, Mutation::Chain(_))
    }

    pub fn label(&self) -> String {
        match self {
            Mutation::TweakThreshold { scale } =>
                format!("Threshold {:+}%", (scale * 100.0).round() as i64),
            Mutation::NegateThreshold    => "Negate threshold".into(),
            Mutation::SwapConditionVar   => "Swap cond var".into(),
            Mutation::SwapBranches       => "Swap branches".into(),
            Mutation::TweakSetValue { scale } =>
                format!("Set value {:+}%", (scale * 100.0).round() as i64),
            Mutation::SwapPredictor      => "Swap predictor".into(),
            Mutation::TweakAllOffsets { scale } =>
                format!("All offsets {:+}%", (scale * 100.0).round() as i64),
            Mutation::AddBranch          => "Add branch".into(),
            Mutation::RemoveBranch       => "Remove branch".into(),
            Mutation::PromoteTrueBranch  => "Promote true branch".into(),
            Mutation::InsertIfAtLeaf     => "Insert if at leaf".into(),
            Mutation::ReplaceSubtreeRandom => "Replace subtree".into(),
            Mutation::CycleRct           => "Cycle RCT".into(),
            Mutation::ToggleHeader       => "Toggle header".into(),
            Mutation::SwapPredictorExotic => "Swap predictor (exotic)".into(),
            Mutation::Chain(ms) =>
                ms.iter().map(|m| m.label()).collect::<Vec<_>>().join(" → "),
        }
    }

    pub fn showcase() -> Vec<Mutation> {
        use Mutation::*;
        vec![
            // ── Threshold tweaks ──────────────────────────────────────────────
            TweakThreshold { scale:  0.15 },
            TweakThreshold { scale: -0.15 },
            TweakThreshold { scale:  0.40 },
            TweakThreshold { scale: -0.40 },
            NegateThreshold,
            // ── Condition / branch structure ──────────────────────────────────
            SwapBranches,
            SwapConditionVar,
            AddBranch,
            RemoveBranch,
            PromoteTrueBranch,
            InsertIfAtLeaf,
            ReplaceSubtreeRandom,
            // ── Predictor / value ─────────────────────────────────────────────
            TweakSetValue { scale:  0.20 },
            TweakSetValue { scale: -0.20 },
            TweakAllOffsets { scale:  0.25 },
            TweakAllOffsets { scale: -0.25 },
            SwapPredictor,
            SwapPredictorExotic,
            // ── Program-level ─────────────────────────────────────────────────
            CycleRct,
            ToggleHeader,
            // ── Compound ──────────────────────────────────────────────────────
            Chain(vec![TweakThreshold { scale: 0.20 }, SwapPredictor]),
            Chain(vec![SwapBranches, TweakThreshold { scale: -0.30 }]),
            Chain(vec![SwapConditionVar, NegateThreshold]),
            Chain(vec![AddBranch, TweakAllOffsets { scale: 0.25 }]),
            Chain(vec![AddBranch, SwapConditionVar, TweakThreshold { scale: 0.20 }]),
            Chain(vec![SwapConditionVar, NegateThreshold, SwapPredictor]),
            Chain(vec![TweakThreshold { scale: 0.30 }, SwapBranches, TweakAllOffsets { scale: -0.20 }]),
            Chain(vec![CycleRct, SwapPredictorExotic]),
            Chain(vec![ToggleHeader, TweakAllOffsets { scale: 0.30 }]),
        ]
    }

    pub fn apply(&self, program: &ImageProgram) -> ImageProgram {
        if let Mutation::Chain(steps) = self {
            return steps.iter().fold(program.clone(), |p, m| m.apply(&p));
        }

        let mut rng = rand::thread_rng();
        let mut prog = program.clone();

        match self {
            Mutation::TweakThreshold { scale } => {
                let thresholds = collect_thresholds(&prog.root);
                if thresholds.is_empty() { return prog; }
                let n     = rng.gen_range(0..thresholds.len());
                let delta = relative_delta(thresholds[n], &thresholds, *scale);
                apply_nth_condition(&mut prog.root, n, &mut 0,
                    &mut |c| c.threshold += delta);
            }
            Mutation::NegateThreshold => {
                let n_conds = count_conditions(&prog.root);
                if n_conds == 0 { return prog; }
                let n = rng.gen_range(0..n_conds);
                apply_nth_condition(&mut prog.root, n, &mut 0,
                    &mut |c| c.threshold = -c.threshold);
            }
            Mutation::SwapConditionVar => {
                let n_conds = count_conditions(&prog.root);
                if n_conds == 0 { return prog; }
                let n    = rng.gen_range(0..n_conds);
                let pick = random_var(&mut rng);
                apply_nth_condition(&mut prog.root, n, &mut 0,
                    &mut |c| c.var = pick.clone());
            }
            Mutation::SwapBranches => {
                let n_conds = count_conditions(&prog.root);
                if n_conds == 0 { return prog; }
                let n = rng.gen_range(0..n_conds);
                swap_nth_branches(&mut prog.root, n, &mut 0);
            }
            Mutation::TweakSetValue { scale } => {
                let set_vals = collect_set_values(&prog.root);
                if set_vals.is_empty() { return prog; }
                let n     = rng.gen_range(0..set_vals.len());
                let delta = relative_delta(set_vals[n], &set_vals, *scale);
                apply_nth_set_predictor(&mut prog.root, n, &mut 0,
                    &mut |v| *v += delta);
            }
            Mutation::SwapPredictor => {
                let n_preds = count_predictors(&prog.root);
                if n_preds == 0 { return prog; }
                let n           = rng.gen_range(0..n_preds);
                let replacement = random_predictor(&mut rng);
                apply_nth_predictor(&mut prog.root, n, &mut 0,
                    &mut |p| *p = replacement.clone());
            }
            Mutation::TweakAllOffsets { scale } => {
                let offsets = collect_offsets(&prog.root);
                if offsets.is_empty() { return prog; }
                let avg_abs = (offsets.iter().map(|v| v.abs()).sum::<i64>() as f64
                               / offsets.len() as f64).max(1.0);
                let mag     = (avg_abs * scale.abs()).round().max(1.0) as i64;
                let delta   = if *scale >= 0.0 { mag } else { -mag };
                tweak_all_offsets(&mut prog.root, delta);
            }
            Mutation::AddBranch => {
                let old = std::mem::replace(&mut prog.root, Node::Predict(Predictor::Set(0)));
                prog.root = Node::If {
                    condition: Condition {
                        var: random_var(&mut rng),
                        op: Op::Gt,
                        threshold: rng.gen_range(0..=255),
                    },
                    on_true:  Box::new(Node::Predict(Predictor::Set(rng.gen_range(0i64..=255)))),
                    on_false: Box::new(old),
                };
            }
            Mutation::RemoveBranch => {
                let old = std::mem::replace(&mut prog.root, Node::Predict(Predictor::Set(0)));
                prog.root = match old {
                    Node::If { on_false, .. } => *on_false,
                    leaf => leaf,
                };
            }
            Mutation::PromoteTrueBranch => {
                let old = std::mem::replace(&mut prog.root, Node::Predict(Predictor::Set(0)));
                prog.root = match old {
                    Node::If { on_true, .. } => *on_true,
                    leaf => leaf,
                };
            }
            Mutation::InsertIfAtLeaf => {
                let n_leaves = count_predictors(&prog.root);
                if n_leaves == 0 { return prog; }
                let n = rng.gen_range(0..n_leaves);
                let condition = Condition {
                    var: random_var(&mut rng),
                    op: Op::Gt,
                    threshold: rng.gen_range(0..=255),
                };
                let sibling = Node::Predict(random_predictor(&mut rng));
                replace_nth_leaf(&mut prog.root, n, &mut 0, &mut |old_leaf| {
                    Node::If {
                        condition: condition.clone(),
                        on_true:  Box::new(old_leaf),
                        on_false: Box::new(sibling.clone()),
                    }
                });
            }
            Mutation::ReplaceSubtreeRandom => {
                let n_nodes = count_all_nodes(&prog.root);
                if n_nodes == 0 { return prog; }
                let n = rng.gen_range(0..n_nodes);
                let replacement = random_node(&mut rng, 1);
                replace_nth_node(&mut prog.root, n, &mut 0,
                    &mut |_| replacement.clone());
            }
            Mutation::CycleRct => {
                // Curated set of visually-distinct RCT presets. 6 (YCoCg) is
                // the default; skipping the current value keeps the mutation
                // always-visible.
                const RCTS: &[u32] = &[0, 2, 6, 10, 13, 16, 20, 27];
                let current = prog.rct.unwrap_or(0);
                let pick = loop {
                    let r = RCTS[rng.gen_range(0..RCTS.len())];
                    if r != current { break r; }
                };
                prog.rct = Some(pick);
            }
            Mutation::ToggleHeader => {
                const TOGGLES: &[&str] = &["Gaborish", "XYB", "DeltaPalette", "Squeeze"];
                let pick = TOGGLES[rng.gen_range(0..TOGGLES.len())];
                let existing = prog.extra_headers.iter().position(|h|
                    h.split_whitespace().next() == Some(pick));
                match existing {
                    Some(i) => { prog.extra_headers.remove(i); }
                    None    => { prog.extra_headers.push(pick.to_string()); }
                }
            }
            Mutation::SwapPredictorExotic => {
                let n_preds = count_predictors(&prog.root);
                if n_preds == 0 { return prog; }
                let n = rng.gen_range(0..n_preds);
                let replacement = random_exotic_predictor(&mut rng);
                apply_nth_predictor(&mut prog.root, n, &mut 0,
                    &mut |p| *p = replacement.clone());
            }
            Mutation::Chain(_) => unreachable!(),
        }
        prog
    }
}

// ── Degenerate check ──────────────────────────────────────────────────────────

pub fn is_degenerate(rgba: &[u8]) -> bool {
    if rgba.len() < 4 { return true; }
    let (mut mn_r, mut mx_r) = (255u8, 0u8);
    let (mut mn_g, mut mx_g) = (255u8, 0u8);
    let (mut mn_b, mut mx_b) = (255u8, 0u8);
    for px in rgba.chunks_exact(4) {
        mn_r = mn_r.min(px[0]); mx_r = mx_r.max(px[0]);
        mn_g = mn_g.min(px[1]); mx_g = mx_g.max(px[1]);
        mn_b = mn_b.min(px[2]); mx_b = mx_b.max(px[2]);
    }
    let range = (mx_r - mn_r) as u16 + (mx_g - mn_g) as u16 + (mx_b - mn_b) as u16;
    range < 10
}

// ── Random primitives ─────────────────────────────────────────────────────────

fn random_var(rng: &mut impl Rng) -> Var {
    match rng.gen_range(0..6u8) {
        0 => Var::X, 1 => Var::Y, 2 => Var::C,
        3 => Var::W, 4 => Var::N, _ => Var::WGH,
    }
}

fn random_predictor(rng: &mut impl Rng) -> Predictor {
    let offset = rng.gen_range(-32i64..=32);
    match rng.gen_range(0..7u8) {
        // Signed range so Co/Cg can go negative under RCT-6 — otherwise
        // red and blue are systematically suppressed.
        0 => Predictor::Set(rng.gen_range(-128i64..=256)),
        1 => Predictor::N(offset),
        2 => Predictor::W(offset),
        3 => Predictor::AvgNNW(offset),
        4 => Predictor::AvgNNE(offset),
        5 => Predictor::AvgWNW(offset),
        _ => Predictor::Weighted(offset),
    }
}

/// Predictors that `jxl_from_tree` accepts as leaves but our structured
/// `Predictor` enum treats as opaque. Newly reachable via the tolerant
/// parser; used by `SwapPredictorExotic`.
///
/// Whitelisted from the gallery corpus — some names like `NN` and `NWW`
/// are valid condition vars but crash `jxl_from_tree` when used as leaf
/// predictors, so they're deliberately excluded.
fn random_exotic_predictor(rng: &mut impl Rng) -> Predictor {
    const NAMES: &[&str] = &[
        "NE", "NW", "WW",
        "AvgW+N", "AvgAll", "Gradient", "Select",
    ];
    let name = NAMES[rng.gen_range(0..NAMES.len())];
    let offset = match rng.gen_range(0..3u8) {
        0 => "0".to_string(),
        1 => format!("+ {}", rng.gen_range(1i64..=32)),
        _ => format!("- {}", rng.gen_range(1i64..=32)),
    };
    Predictor::Other { name: name.to_string(), offset }
}

// ── Tree inspection ───────────────────────────────────────────────────────────

fn collect_thresholds(node: &Node) -> Vec<i64> {
    match node {
        Node::If { condition, on_true, on_false } => {
            let mut v = vec![condition.threshold];
            v.extend(collect_thresholds(on_true));
            v.extend(collect_thresholds(on_false));
            v
        }
        Node::Predict(_) => vec![],
    }
}

fn collect_set_values(node: &Node) -> Vec<i64> {
    match node {
        Node::If { on_true, on_false, .. } => {
            let mut v = collect_set_values(on_true);
            v.extend(collect_set_values(on_false));
            v
        }
        Node::Predict(Predictor::Set(v)) => vec![*v],
        Node::Predict(_)                 => vec![],
    }
}

fn collect_offsets(node: &Node) -> Vec<i64> {
    match node {
        Node::If { on_true, on_false, .. } => {
            let mut v = collect_offsets(on_true);
            v.extend(collect_offsets(on_false));
            v
        }
        Node::Predict(pred) => match pred {
            Predictor::N(o) | Predictor::W(o)
            | Predictor::AvgNNW(o) | Predictor::AvgNNE(o)
            | Predictor::AvgWNW(o) | Predictor::Weighted(o) => vec![*o],
            Predictor::Set(_) => vec![],
            Predictor::Other { .. } => vec![],
        },
    }
}

fn count_conditions(node: &Node) -> usize {
    match node {
        Node::If { on_true, on_false, .. } =>
            1 + count_conditions(on_true) + count_conditions(on_false),
        Node::Predict(_) => 0,
    }
}

fn count_predictors(node: &Node) -> usize {
    match node {
        Node::If { on_true, on_false, .. } =>
            count_predictors(on_true) + count_predictors(on_false),
        Node::Predict(_) => 1,
    }
}

fn count_all_nodes(node: &Node) -> usize {
    match node {
        Node::If { on_true, on_false, .. } =>
            1 + count_all_nodes(on_true) + count_all_nodes(on_false),
        Node::Predict(_) => 1,
    }
}

// ── Tree mutation (targeted) ──────────────────────────────────────────────────

/// Apply `f` to the n-th If node's condition (pre-order DFS).
fn apply_nth_condition(
    node: &mut Node, n: usize, seen: &mut usize,
    f: &mut dyn FnMut(&mut Condition),
) {
    if let Node::If { condition, on_true, on_false } = node {
        let idx = *seen;
        *seen += 1;
        if idx == n {
            f(condition);
        } else {
            apply_nth_condition(on_true, n, seen, f);
            apply_nth_condition(on_false, n, seen, f);
        }
    }
}

/// Swap on_true/on_false of the n-th If node (pre-order DFS).
fn swap_nth_branches(node: &mut Node, n: usize, seen: &mut usize) {
    if let Node::If { on_true, on_false, .. } = node {
        let idx = *seen;
        *seen += 1;
        if idx == n {
            std::mem::swap(on_true, on_false);
        } else {
            swap_nth_branches(on_true, n, seen);
            swap_nth_branches(on_false, n, seen);
        }
    }
}

/// Apply `f` to the n-th Predict leaf (DFS, on_true before on_false).
fn apply_nth_predictor(
    node: &mut Node, n: usize, seen: &mut usize,
    f: &mut dyn FnMut(&mut Predictor),
) {
    match node {
        Node::If { on_true, on_false, .. } => {
            apply_nth_predictor(on_true, n, seen, f);
            apply_nth_predictor(on_false, n, seen, f);
        }
        Node::Predict(pred) => {
            if *seen == n { f(pred); }
            *seen += 1;
        }
    }
}

/// Apply `f` to the value inside the n-th Set predictor leaf.
fn apply_nth_set_predictor(
    node: &mut Node, n: usize, seen: &mut usize,
    f: &mut dyn FnMut(&mut i64),
) {
    match node {
        Node::If { on_true, on_false, .. } => {
            apply_nth_set_predictor(on_true, n, seen, f);
            apply_nth_set_predictor(on_false, n, seen, f);
        }
        Node::Predict(Predictor::Set(v)) => {
            if *seen == n { f(v); }
            *seen += 1;
        }
        Node::Predict(_) => {}
    }
}

/// Replace the n-th Predict leaf (DFS, on_true before on_false) with
/// `f(old_leaf)`. Used by `InsertIfAtLeaf` to split a leaf into an If.
fn replace_nth_leaf(
    node: &mut Node, n: usize, seen: &mut usize,
    f: &mut dyn FnMut(Node) -> Node,
) {
    match node {
        Node::If { on_true, on_false, .. } => {
            replace_nth_leaf(on_true, n, seen, f);
            replace_nth_leaf(on_false, n, seen, f);
        }
        Node::Predict(_) => {
            if *seen == n {
                let old = std::mem::replace(node, Node::Predict(Predictor::Set(0)));
                *node = f(old);
            }
            *seen += 1;
        }
    }
}

/// Replace the n-th node (pre-order DFS, counting both If and Predict
/// nodes) with `f(old)`. Used by `ReplaceSubtreeRandom`. Once a node is
/// replaced we don't recurse into its (now-discarded) children.
fn replace_nth_node(
    node: &mut Node, n: usize, seen: &mut usize,
    f: &mut dyn FnMut(Node) -> Node,
) {
    let idx = *seen;
    *seen += 1;
    if idx == n {
        let old = std::mem::replace(node, Node::Predict(Predictor::Set(0)));
        *node = f(old);
        return;
    }
    if let Node::If { on_true, on_false, .. } = node {
        replace_nth_node(on_true, n, seen, f);
        replace_nth_node(on_false, n, seen, f);
    }
}

/// Add `delta` to every non-Set predictor offset in the tree.
fn tweak_all_offsets(node: &mut Node, delta: i64) {
    match node {
        Node::Predict(pred) => match pred {
            Predictor::N(o) | Predictor::W(o)
            | Predictor::AvgNNW(o) | Predictor::AvgNNE(o)
            | Predictor::AvgWNW(o) | Predictor::Weighted(o) => *o += delta,
            Predictor::Set(_) => {}
            Predictor::Other { .. } => {}
        },
        Node::If { on_true, on_false, .. } => {
            tweak_all_offsets(on_true, delta);
            tweak_all_offsets(on_false, delta);
        }
    }
}

// ── Relative delta ────────────────────────────────────────────────────────────

/// Compute a delta proportional to `current` using `scale` as a fraction of
/// its absolute value.  Falls back to the average of `all_values` when
/// `current` is zero.  Always returns at least ±1.
fn relative_delta(current: i64, all_values: &[i64], scale: f64) -> i64 {
    let base = if current.abs() > 0 {
        current.abs()
    } else {
        let nonzero: Vec<i64> = all_values.iter()
            .map(|v| v.abs()).filter(|&v| v > 0).collect();
        if nonzero.is_empty() { 10 }
        else { nonzero.iter().sum::<i64>() / nonzero.len() as i64 }
    };
    let magnitude = ((base as f64 * scale.abs()).round() as i64).max(1);
    if scale >= 0.0 { magnitude } else { -magnitude }
}
