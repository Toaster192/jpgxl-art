use std::collections::HashMap;

use rand::Rng;
use serde::Serialize;

use crate::render;
use crate::tree::{Condition, ImageProgram, Node, Op, Predictor, Var};

// ── Random program generation ─────────────────────────────────────────────────

/// RCT presets surfaced to the generator + `CycleRct` mutation.
/// A curated visually-distinct subset of libjxl's full RCT set.
pub(crate) const RCT_POOL: &[u32] = &[0, 2, 6, 10, 13, 16, 20, 27];

/// Header flags surfaced to the generator + `ToggleHeader` mutation.
/// Each one independently changes the visual character of the output.
pub(crate) const HEADER_POOL: &[&str] = &["Gaborish", "XYB", "DeltaPalette", "Squeeze"];

/// User-facing complexity dial for the random program generator. Controls
/// the branch-probability curve in `random_node` — header probability is
/// intentionally NOT scaled, since piling on headers tends to make outputs
/// look samey (Squeeze + DeltaPalette + XYB at once washes detail out).
#[derive(Debug, Clone, Copy)]
pub enum Complexity {
    Simple,
    Normal,
    Complex,
}

impl Complexity {
    pub fn from_u8(n: u8) -> Self {
        match n {
            0 => Self::Simple,
            2 => Self::Complex,
            _ => Self::Normal,
        }
    }

    /// Branch probabilities indexed by tree depth. After the slice ends, the
    /// generator forces a leaf — so the slice length sets the maximum depth.
    fn branch_probs(self) -> &'static [f64] {
        match self {
            // ~3-5 nodes typical: usually one or two splits then leaves.
            Self::Simple => &[0.65, 0.30, 0.10],
            // ~17 nodes typical: the long-standing default.
            Self::Normal => &[0.95, 0.85, 0.70, 0.50, 0.25],
            // ~60+ nodes typical: deeper trees with more conditioning.
            Self::Complex => &[0.98, 0.95, 0.90, 0.80, 0.60, 0.40, 0.20],
        }
    }
}

fn random_rct(rng: &mut impl Rng) -> u32 {
    RCT_POOL[rng.gen_range(0..RCT_POOL.len())]
}

/// Each header in the pool is flipped on with ~30% probability. Preserves
/// source order (Gaborish → XYB → DeltaPalette → Squeeze) for readability.
fn random_headers(rng: &mut impl Rng) -> Vec<String> {
    HEADER_POOL
        .iter()
        .filter(|_| rng.gen_bool(0.30))
        .map(|h| h.to_string())
        .collect()
}

/// Remove every `extra_headers` line whose first token is `key`.
fn remove_header(headers: &mut Vec<String>, key: &str) {
    headers.retain(|h| h.split_whitespace().next() != Some(key));
}

/// Curated bitdepths for the random generator. 8 is weighted (appears twice);
/// low values posterize, high values give smoother / true-HDR `.jxl` output.
/// Value ranges scale to the bitdepth (via `GenCtx`) so high bitdepth doesn't
/// just produce dark images.
const GEN_BITDEPTHS: &[u32] = &[4, 5, 6, 8, 8, 10, 12];

/// Generate a random program. `dims` overrides the canvas (from the UI's
/// size×aspect selector); `None` keeps the default 1024² with an occasional
/// small-canvas "pixel mode".
pub fn random_program(complexity: Complexity, dims: Option<(u32, u32)>) -> ImageProgram {
    let mut rng = rand::thread_rng();
    let probs = complexity.branch_probs();

    let mut bitdepth = GEN_BITDEPTHS[rng.gen_range(0..GEN_BITDEPTHS.len())];
    let mut extra_headers = random_headers(&mut rng);

    // Pixel mode (small canvas + Upsample) only applies when no explicit canvas
    // was requested. Decided up front because it's mutually exclusive with
    // alpha: Alpha + Upsample(≥2) crashes jxl_from_tree.
    let pixel_mode = dims.is_none() && rng.gen_bool(0.15);

    // ~18% alpha mode (never alongside pixel mode): add an Alpha plane so the
    // tree can condition on c=3. Carried purely by the header — we never emit
    // `Channels` (jxl_from_tree v0.11.2 rejects that directive).
    let alpha = !pixel_mode && rng.gen_bool(0.18);
    if alpha {
        extra_headers.push("Alpha".to_string());
    }
    let channels = if alpha { 4 } else { 3 };

    // Orientation 5/6/7/8 transpose width↔height on decode (libjxl applies
    // EXIF orientation), so a requested "wide" canvas would otherwise display
    // tall half the time. Pick it up front so explicit dims can compensate.
    let orientation = rng.gen_range(1u32..=8);
    let transposes = matches!(orientation, 5..=8);

    // Dimensions. An explicit canvas (size×aspect) is the requested *display*
    // shape — pre-swap it when the orientation transposes so the rendered card
    // matches. No pixel-mode/Upsample there (would override/balloon the chosen
    // size). Otherwise default 1024² with ~15% small-canvas pixel mode (square,
    // so transposition is a visual no-op). The Upsample factor is bounded so
    // native size (dim*factor) ≤ 1024 — `decode_jxl` decodes at native res.
    let (width, height) = match dims {
        Some((w, h)) => {
            let (w, h) = (w.max(1), h.max(1));
            if transposes {
                (h, w)
            } else {
                (w, h)
            }
        }
        None => {
            if pixel_mode {
                let dim = [64u32, 96, 128, 256][rng.gen_range(0..4)];
                let factor = (1024 / dim).min(8); // 64→8, 96→8, 128→8, 256→4
                extra_headers.push(format!("Upsample {}", factor));
                bitdepth = rng.gen_range(4u32..=6); // blocky look wants low bitdepth
                (dim, dim)
            } else {
                (1024, 1024)
            }
        }
    };

    let ctx = GenCtx {
        value_max: value_max_for(bitdepth),
        width,
        height,
        channels,
    };

    // Force a channel split at the root so RCT-6 (YCoCg inverse) actually
    // produces varied colour. Without this, trees that never condition on
    // `c` emit identical Y/Co/Cg values for all channels, and the inverse
    // transform of (V,V,V) is always yellow-green — the single biggest
    // source of colour bias in random output. Harmless under other RCTs.
    // With alpha, allow c>2 so the alpha plane gets its own subtree.
    let c_threshold: i64 = if alpha {
        rng.gen_range(0i64..=2)
    } else if rng.gen_bool(0.5) {
        0
    } else {
        1
    };
    let mut root = Node::If {
        condition: Condition {
            var: Var::C,
            op: Op::Gt,
            threshold: c_threshold,
        },
        on_true: Box::new(random_node(&mut rng, 1, probs, &ctx)),
        on_false: Box::new(random_node(&mut rng, 1, probs, &ctx)),
    };
    // Strip dead nested same-property comparisons libjxl's djxl can't decode.
    simplify_degenerate(&mut root);

    ImageProgram {
        width,
        height,
        bitdepth,
        channels: 3, // never emit `Channels`; alpha is a header
        orientation: Some(orientation),
        rct: Some(random_rct(&mut rng)),
        extra_headers,
        splines: Vec::new(),
        root,
    }
}

/// Generate a random program whose preview is not degenerate
/// (single-colour / flat / fully-transparent). Uses the roundtrip renderer at
/// 64 px so the check is accurate to what libjxl will actually produce. On
/// retry we first just re-roll the cheap program-level knobs (RCT + headers)
/// that often pull a flat result back into colour (this also drops a stray
/// Alpha/Upsample); only after several failures do we regenerate the tree.
pub fn random_program_non_degenerate(
    complexity: Complexity,
    dims: Option<(u32, u32)>,
) -> ImageProgram {
    const MAX_TRIES: usize = 10;
    const CHEAP_REROLLS: usize = MAX_TRIES - 3;
    let mut rng = rand::thread_rng();
    let mut prog = random_program(complexity, dims);
    for attempt in 0..MAX_TRIES {
        let text = prog.to_text();
        if let Ok((rgba, _, _, _)) = render::render_roundtrip(&text, 64) {
            if !is_degenerate(&rgba) {
                return prog;
            }
        }
        if attempt < CHEAP_REROLLS {
            prog.rct = Some(random_rct(&mut rng));
            prog.extra_headers = random_headers(&mut rng);
        } else {
            prog = random_program(complexity, dims);
        }
    }
    prog
}

fn random_node(rng: &mut impl Rng, depth: usize, branch_probs: &[f64], ctx: &GenCtx) -> Node {
    // Branch probability falls off with depth; always a leaf once we run off
    // the end of the curve.
    let branch_prob = branch_probs.get(depth).copied().unwrap_or(0.0);
    if rng.gen::<f64>() < branch_prob {
        // Pick threshold range appropriate to the variable.
        let var = random_var(rng);
        let threshold = random_threshold_for(&var, ctx, rng);
        Node::If {
            condition: Condition {
                var,
                op: Op::Gt,
                threshold,
            },
            on_true: Box::new(random_node(rng, depth + 1, branch_probs, ctx)),
            on_false: Box::new(random_node(rng, depth + 1, branch_probs, ctx)),
        }
    } else {
        Node::Predict(random_predictor(ctx, rng))
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "params")]
pub enum Mutation {
    // ── Single mutations ──────────────────────────────────────────────────────
    /// Nudge a randomly-chosen condition's threshold by ±scale of its current value.
    TweakThreshold {
        scale: f64,
    },
    /// Negate a randomly-chosen condition's threshold.
    NegateThreshold,
    /// Replace the variable in a randomly-chosen condition with a random one.
    SwapConditionVar,
    /// Swap on_true / on_false of a randomly-chosen If node.
    SwapBranches,
    /// Nudge a randomly-chosen Set predictor's value by ±scale of its current value.
    TweakSetValue {
        scale: f64,
    },
    /// Replace a randomly-chosen predictor leaf with a random neighbour-based one.
    SwapPredictor,
    /// Shift every predictor offset by ±scale of the average offset magnitude.
    TweakAllOffsets {
        scale: f64,
    },
    // ── Structural mutations ──────────────────────────────────────────────────
    /// Wrap the tree in a new random If (old tree becomes on_false).
    AddBranch,
    /// Replace root with its FALSE child.
    RemoveBranch,
    /// Replace root with its TRUE child.
    PromoteTrueBranch,
    /// Pick any If in the tree (not just root) and replace it with its
    /// FALSE child. Generalises `RemoveBranch`.
    RemoveBranchAt,
    /// Pick any If in the tree (not just root) and replace it with its
    /// TRUE child. Generalises `PromoteTrueBranch`.
    PromoteTrueAt,
    /// Swap two subtrees within the program. Both subtrees must lie on
    /// disjoint root-paths (neither is an ancestor of the other) — small
    /// trees may have no valid pair, in which case this is a no-op.
    SwapSubtrees,
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
    /// Add or cycle the `Upsample` factor (none → 2 → 4 → 2). Capped at 4:
    /// on a 1024² program `Upsample 8` would decode to 8192², so factor 8 is
    /// reserved for the small-canvas generation path. Renders each pixel as a
    /// blocky upscaled block.
    CycleUpsample,
    /// Step `Bitdepth` to a different curated value. No value rescale — the
    /// point is to see how the same tree reinterprets at another bit depth
    /// (lower → posterized, higher → darker/smoother).
    CycleBitdepth,
    /// Transpose width and height — flips a wide canvas tall and vice-versa.
    SwapWidthHeight,
    /// Add/remove an `Alpha` plane (transparency).
    ToggleAlpha,
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
    match rng.gen_range(0..19u8) {
        0 => Mutation::TweakThreshold { scale },
        1 => Mutation::NegateThreshold,
        2 => Mutation::SwapConditionVar,
        3 => Mutation::SwapBranches,
        4 => Mutation::TweakSetValue { scale },
        5 => Mutation::SwapPredictor,
        6 => Mutation::TweakAllOffsets { scale },
        7 => Mutation::CycleRct,
        8 => Mutation::ToggleHeader,
        9 => Mutation::SwapPredictorExotic,
        10 => Mutation::InsertIfAtLeaf,
        11 => Mutation::RemoveBranchAt,
        12 => Mutation::PromoteTrueAt,
        13 => Mutation::SwapSubtrees,
        14 => Mutation::CycleUpsample,
        15 => Mutation::CycleBitdepth,
        16 => Mutation::SwapWidthHeight,
        17 => Mutation::ToggleAlpha,
        _ => Mutation::ReplaceSubtreeRandom,
    }
}

impl Mutation {
    pub fn is_compound(&self) -> bool {
        matches!(self, Mutation::Chain(_))
    }

    pub fn label(&self) -> String {
        match self {
            Mutation::TweakThreshold { scale } => {
                format!("Threshold {:+}%", (scale * 100.0).round() as i64)
            }
            Mutation::NegateThreshold => "Negate threshold".into(),
            Mutation::SwapConditionVar => "Swap cond var".into(),
            Mutation::SwapBranches => "Swap branches".into(),
            Mutation::TweakSetValue { scale } => {
                format!("Set value {:+}%", (scale * 100.0).round() as i64)
            }
            Mutation::SwapPredictor => "Swap predictor".into(),
            Mutation::TweakAllOffsets { scale } => {
                format!("All offsets {:+}%", (scale * 100.0).round() as i64)
            }
            Mutation::AddBranch => "Add branch".into(),
            Mutation::RemoveBranch => "Remove branch".into(),
            Mutation::PromoteTrueBranch => "Promote true branch".into(),
            Mutation::RemoveBranchAt => "Remove branch (deep)".into(),
            Mutation::PromoteTrueAt => "Promote true branch (deep)".into(),
            Mutation::SwapSubtrees => "Swap subtrees".into(),
            Mutation::InsertIfAtLeaf => "Insert if at leaf".into(),
            Mutation::ReplaceSubtreeRandom => "Replace subtree".into(),
            Mutation::CycleRct => "Cycle RCT".into(),
            Mutation::ToggleHeader => "Toggle header".into(),
            Mutation::SwapPredictorExotic => "Swap predictor (exotic)".into(),
            Mutation::CycleUpsample => "Cycle upsample".into(),
            Mutation::CycleBitdepth => "Cycle bitdepth".into(),
            Mutation::SwapWidthHeight => "Swap W↔H".into(),
            Mutation::ToggleAlpha => "Toggle alpha".into(),
            Mutation::Chain(ms) => ms.iter().map(|m| m.label()).collect::<Vec<_>>().join(" → "),
        }
    }

    pub fn showcase() -> Vec<Mutation> {
        use Mutation::*;
        vec![
            // ── Threshold tweaks ──────────────────────────────────────────────
            TweakThreshold { scale: 0.15 },
            TweakThreshold { scale: -0.15 },
            TweakThreshold { scale: 0.40 },
            TweakThreshold { scale: -0.40 },
            NegateThreshold,
            // ── Condition / branch structure ──────────────────────────────────
            SwapBranches,
            SwapConditionVar,
            AddBranch,
            RemoveBranch,
            PromoteTrueBranch,
            RemoveBranchAt,
            PromoteTrueAt,
            SwapSubtrees,
            InsertIfAtLeaf,
            ReplaceSubtreeRandom,
            // ── Predictor / value ─────────────────────────────────────────────
            TweakSetValue { scale: 0.20 },
            TweakSetValue { scale: -0.20 },
            TweakAllOffsets { scale: 0.25 },
            TweakAllOffsets { scale: -0.25 },
            SwapPredictor,
            SwapPredictorExotic,
            // ── Program-level ─────────────────────────────────────────────────
            CycleRct,
            ToggleHeader,
            CycleUpsample,
            CycleBitdepth,
            ToggleAlpha,
            // ── Compound ──────────────────────────────────────────────────────
            Chain(vec![TweakThreshold { scale: 0.20 }, SwapPredictor]),
            Chain(vec![SwapBranches, TweakThreshold { scale: -0.30 }]),
            Chain(vec![SwapConditionVar, NegateThreshold]),
            Chain(vec![AddBranch, TweakAllOffsets { scale: 0.25 }]),
            Chain(vec![
                AddBranch,
                SwapConditionVar,
                TweakThreshold { scale: 0.20 },
            ]),
            Chain(vec![SwapConditionVar, NegateThreshold, SwapPredictor]),
            Chain(vec![
                TweakThreshold { scale: 0.30 },
                SwapBranches,
                TweakAllOffsets { scale: -0.20 },
            ]),
            Chain(vec![CycleRct, SwapPredictorExotic]),
            Chain(vec![ToggleHeader, TweakAllOffsets { scale: 0.30 }]),
        ]
    }

    pub fn apply(&self, program: &ImageProgram) -> ImageProgram {
        if let Mutation::Chain(steps) = self {
            return steps.iter().fold(program.clone(), |p, m| m.apply(&p));
        }
        let mut prog = self.apply_one(program);
        // Strip any dead nested same-property comparison the mutation may have
        // created (SwapConditionVar / AddBranch / InsertIfAtLeaf / SwapSubtrees
        // can all produce one) — djxl can't decode those. Runs on every path,
        // including the early returns inside apply_one.
        simplify_degenerate(&mut prog.root);
        prog
    }

    /// Apply a single (non-`Chain`) mutation and return the raw result. The
    /// `apply` wrapper simplifies afterwards.
    fn apply_one(&self, program: &ImageProgram) -> ImageProgram {
        let mut rng = rand::thread_rng();
        let mut prog = program.clone();
        // Value/threshold ranges for any freshly-generated nodes match the
        // program's own bitdepth / canvas / channels.
        let ctx = GenCtx::from_program(&prog);

        match self {
            Mutation::TweakThreshold { scale } => {
                let thresholds = collect_thresholds(&prog.root);
                if thresholds.is_empty() {
                    return prog;
                }
                let n = rng.gen_range(0..thresholds.len());
                let delta = relative_delta(thresholds[n], &thresholds, *scale);
                apply_nth_condition(&mut prog.root, n, &mut 0, &mut |c| c.threshold += delta);
            }
            Mutation::NegateThreshold => {
                let n_conds = count_conditions(&prog.root);
                if n_conds == 0 {
                    return prog;
                }
                let n = rng.gen_range(0..n_conds);
                apply_nth_condition(&mut prog.root, n, &mut 0, &mut |c| {
                    c.threshold = -c.threshold
                });
            }
            Mutation::SwapConditionVar => {
                let n_conds = count_conditions(&prog.root);
                if n_conds == 0 {
                    return prog;
                }
                let n = rng.gen_range(0..n_conds);
                let pick = random_var(&mut rng);
                // Reset the threshold too — a difference-type var keeps the old
                // coordinate threshold otherwise, making the branch near-dead.
                let thr = random_threshold_for(&pick, &ctx, &mut rng);
                apply_nth_condition(&mut prog.root, n, &mut 0, &mut |c| {
                    c.var = pick.clone();
                    c.threshold = thr;
                });
            }
            Mutation::SwapBranches => {
                let n_conds = count_conditions(&prog.root);
                if n_conds == 0 {
                    return prog;
                }
                let n = rng.gen_range(0..n_conds);
                swap_nth_branches(&mut prog.root, n, &mut 0);
            }
            Mutation::TweakSetValue { scale } => {
                let set_vals = collect_set_values(&prog.root);
                if set_vals.is_empty() {
                    return prog;
                }
                let n = rng.gen_range(0..set_vals.len());
                let delta = relative_delta(set_vals[n], &set_vals, *scale);
                apply_nth_set_predictor(&mut prog.root, n, &mut 0, &mut |v| *v += delta);
            }
            Mutation::SwapPredictor => {
                let n_preds = count_predictors(&prog.root);
                if n_preds == 0 {
                    return prog;
                }
                let n = rng.gen_range(0..n_preds);
                let replacement = random_predictor(&ctx, &mut rng);
                apply_nth_predictor(&mut prog.root, n, &mut 0, &mut |p| *p = replacement.clone());
            }
            Mutation::TweakAllOffsets { scale } => {
                let offsets = collect_offsets(&prog.root);
                if offsets.is_empty() {
                    return prog;
                }
                let avg_abs = (offsets.iter().map(|v| v.abs()).sum::<i64>() as f64
                    / offsets.len() as f64)
                    .max(1.0);
                let mag = (avg_abs * scale.abs()).round().max(1.0) as i64;
                let delta = if *scale >= 0.0 { mag } else { -mag };
                tweak_all_offsets(&mut prog.root, delta);
            }
            Mutation::AddBranch => {
                let old = std::mem::replace(&mut prog.root, Node::Predict(Predictor::Set(0)));
                let var = random_var(&mut rng);
                let threshold = random_threshold_for(&var, &ctx, &mut rng);
                prog.root = Node::If {
                    condition: Condition {
                        var,
                        op: Op::Gt,
                        threshold,
                    },
                    on_true: Box::new(Node::Predict(random_predictor(&ctx, &mut rng))),
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
            Mutation::RemoveBranchAt => {
                let n_ifs = count_conditions(&prog.root);
                if n_ifs == 0 {
                    return prog;
                }
                let n = rng.gen_range(0..n_ifs);
                collapse_nth_if(&mut prog.root, n, &mut 0, false);
            }
            Mutation::PromoteTrueAt => {
                let n_ifs = count_conditions(&prog.root);
                if n_ifs == 0 {
                    return prog;
                }
                let n = rng.gen_range(0..n_ifs);
                collapse_nth_if(&mut prog.root, n, &mut 0, true);
            }
            Mutation::SwapSubtrees => {
                let mut paths: Vec<Vec<bool>> = Vec::new();
                collect_node_paths(&prog.root, &mut Vec::new(), &mut paths);
                if paths.len() < 2 {
                    return prog;
                }
                // A few rejection-sampling attempts; small trees may have no
                // valid disjoint pair (e.g. only root + two leaves), in which
                // case we fall through to the no-op path.
                for _ in 0..16 {
                    let i = rng.gen_range(0..paths.len());
                    let j = rng.gen_range(0..paths.len());
                    if i == j {
                        continue;
                    }
                    let a = paths[i].clone();
                    let b = paths[j].clone();
                    if is_path_ancestor(&a, &b) || is_path_ancestor(&b, &a) {
                        continue;
                    }
                    swap_subtrees_at(&mut prog.root, &a, &b);
                    return prog;
                }
            }
            Mutation::InsertIfAtLeaf => {
                let n_leaves = count_predictors(&prog.root);
                if n_leaves == 0 {
                    return prog;
                }
                let n = rng.gen_range(0..n_leaves);
                let var = random_var(&mut rng);
                let condition = Condition {
                    threshold: random_threshold_for(&var, &ctx, &mut rng),
                    var,
                    op: Op::Gt,
                };
                let sibling = Node::Predict(random_predictor(&ctx, &mut rng));
                replace_nth_leaf(&mut prog.root, n, &mut 0, &mut |old_leaf| Node::If {
                    condition: condition.clone(),
                    on_true: Box::new(old_leaf),
                    on_false: Box::new(sibling.clone()),
                });
            }
            Mutation::ReplaceSubtreeRandom => {
                let n_nodes = count_all_nodes(&prog.root);
                if n_nodes == 0 {
                    return prog;
                }
                let n = rng.gen_range(0..n_nodes);
                // Mid-mutation subtree generation always uses the Normal
                // curve — the user's chosen complexity only governs the
                // initial random program, not subsequent mutations.
                let replacement = random_node(&mut rng, 1, Complexity::Normal.branch_probs(), &ctx);
                replace_nth_node(&mut prog.root, n, &mut 0, &mut |_| replacement.clone());
            }
            Mutation::CycleRct => {
                // Skip the current value so the mutation is always-visible.
                let current = prog.rct.unwrap_or(0);
                let pick = loop {
                    let r = RCT_POOL[rng.gen_range(0..RCT_POOL.len())];
                    if r != current {
                        break r;
                    }
                };
                prog.rct = Some(pick);
            }
            Mutation::ToggleHeader => {
                let pick = HEADER_POOL[rng.gen_range(0..HEADER_POOL.len())];
                let existing = prog
                    .extra_headers
                    .iter()
                    .position(|h| h.split_whitespace().next() == Some(pick));
                match existing {
                    Some(i) => {
                        prog.extra_headers.remove(i);
                    }
                    None => {
                        prog.extra_headers.push(pick.to_string());
                    }
                }
            }
            Mutation::CycleUpsample => {
                // Alpha + Upsample(≥2) crashes jxl_from_tree; they're mutually
                // exclusive, so adding/raising Upsample drops Alpha.
                remove_header(&mut prog.extra_headers, "Alpha");
                let pos = prog
                    .extra_headers
                    .iter()
                    .position(|h| h.split_whitespace().next() == Some("Upsample"));
                match pos {
                    Some(i) => {
                        let cur: u32 = prog.extra_headers[i]
                            .split_whitespace()
                            .nth(1)
                            .and_then(|v| v.parse().ok())
                            .unwrap_or(2);
                        // none → +2 (handled below) → 4 → 2 → 4 …; never 8 here.
                        let next = if cur >= 4 { 2 } else { 4 };
                        prog.extra_headers[i] = format!("Upsample {}", next);
                    }
                    None => prog.extra_headers.push("Upsample 2".to_string()),
                }
            }
            Mutation::CycleBitdepth => {
                // Step to a different curated bitdepth; no value rescale.
                const POOL: &[u32] = &[4, 5, 6, 8, 10, 12];
                let current = prog.bitdepth;
                prog.bitdepth = loop {
                    let b = POOL[rng.gen_range(0..POOL.len())];
                    if b != current {
                        break b;
                    }
                };
            }
            Mutation::SwapWidthHeight => {
                std::mem::swap(&mut prog.width, &mut prog.height);
            }
            Mutation::ToggleAlpha => {
                let pos = prog
                    .extra_headers
                    .iter()
                    .position(|h| h.split_whitespace().next() == Some("Alpha"));
                match pos {
                    Some(i) => {
                        prog.extra_headers.remove(i);
                    }
                    None => {
                        // Alpha + Upsample(≥2) crashes jxl_from_tree; they're
                        // mutually exclusive, so adding Alpha drops Upsample.
                        remove_header(&mut prog.extra_headers, "Upsample");
                        prog.extra_headers.push("Alpha".to_string());
                    }
                }
            }
            Mutation::SwapPredictorExotic => {
                let n_preds = count_predictors(&prog.root);
                if n_preds == 0 {
                    return prog;
                }
                let n = rng.gen_range(0..n_preds);
                let replacement = random_exotic_predictor(&mut rng);
                apply_nth_predictor(&mut prog.root, n, &mut 0, &mut |p| *p = replacement.clone());
            }
            Mutation::Chain(_) => unreachable!(),
        }
        prog
    }
}

// ── Dead-condition simplification ───────────────────────────────────────────

/// Collapse conditions that are always-true or always-false given the ancestor
/// conditions on the **same property** along the path. libjxl's `djxl` decoder
/// rejects a JXL whose MA tree contains such a degenerate nested comparison
/// (e.g. `W > 18` with a nested `W > 16`, or `c > 1` inside `c > 1`) — even
/// though `jxl_from_tree` happily encodes it and jxl-oxide used to decode it.
/// Removing the dead branch never changes the rendered image (it was
/// unreachable), and it leaves genuinely-narrowing nests (`c > 0` → `c > 1`)
/// intact. Applied to every generated and mutated program before it's encoded.
pub(crate) fn simplify_degenerate(node: &mut Node) {
    // bounds: per-property inclusive [low, high] the value is constrained to by
    // ancestors on this path. `prop > t` ⇒ true branch low = t+1, false high = t.
    fn rec(node: &mut Node, bounds: &mut HashMap<String, (i64, i64)>) {
        let taken = std::mem::replace(node, Node::Predict(Predictor::Set(0)));
        match taken {
            Node::If {
                condition,
                mut on_true,
                mut on_false,
            } => {
                let key = condition.var.label().to_string();
                let t = condition.threshold;
                let orig = bounds.get(&key).copied();
                let (low, high) = orig.unwrap_or((i64::MIN, i64::MAX));

                if low > t {
                    // prop is already > t everywhere here → always true.
                    *node = *on_true;
                    rec(node, bounds);
                    return;
                }
                if high <= t {
                    // prop is already ≤ t everywhere here → always false.
                    *node = *on_false;
                    rec(node, bounds);
                    return;
                }

                bounds.insert(key.clone(), (low.max(t.saturating_add(1)), high));
                rec(on_true.as_mut(), bounds);
                bounds.insert(key.clone(), (low, high.min(t)));
                rec(on_false.as_mut(), bounds);
                match orig {
                    Some(v) => {
                        bounds.insert(key, v);
                    }
                    None => {
                        bounds.remove(&key);
                    }
                }
                *node = Node::If {
                    condition,
                    on_true,
                    on_false,
                };
            }
            leaf => *node = leaf,
        }
    }
    rec(node, &mut HashMap::new());
}

// ── Degenerate check ──────────────────────────────────────────────────────────

pub fn is_degenerate(rgba: &[u8]) -> bool {
    if rgba.len() < 4 {
        return true;
    }
    let (mut mn_r, mut mx_r) = (255u8, 0u8);
    let (mut mn_g, mut mx_g) = (255u8, 0u8);
    let (mut mn_b, mut mx_b) = (255u8, 0u8);
    let mut alpha_sum: u64 = 0;
    let mut count: u64 = 0;
    for px in rgba.chunks_exact(4) {
        mn_r = mn_r.min(px[0]);
        mx_r = mx_r.max(px[0]);
        mn_g = mn_g.min(px[1]);
        mx_g = mx_g.max(px[1]);
        mn_b = mn_b.min(px[2]);
        mx_b = mx_b.max(px[2]);
        alpha_sum += px[3] as u64;
        count += 1;
    }
    let range = (mx_r - mn_r) as u16 + (mx_g - mn_g) as u16 + (mx_b - mn_b) as u16;
    // Non-alpha images decode to alpha=255 everywhere, so this only trips for
    // alpha-mode programs whose plane renders (near-)fully transparent.
    let mean_alpha = alpha_sum.checked_div(count).unwrap_or(0);
    range < 10 || mean_alpha < 8
}

// ── Random primitives ─────────────────────────────────────────────────────────

/// Context for value-scaled generation. Predictor values, thresholds and
/// offsets are expressed in `0 .. value_max` (= `2^bitdepth - 1`), so a
/// high-bitdepth program isn't dominated by tiny (dark) values; coordinate
/// thresholds scale to the actual canvas; `c` conditions scale to the channel
/// count (4 when an Alpha plane is present).
#[derive(Debug, Clone, Copy)]
pub struct GenCtx {
    value_max: i64,
    width: u32,
    height: u32,
    channels: u32,
}

impl GenCtx {
    /// Derive a context from an existing program. Alpha is detected from the
    /// header (we never emit a `Channels` directive — jxl_from_tree v0.11.2
    /// rejects it — so the struct's `channels` field stays 3 and the logical
    /// count comes from the `Alpha` header).
    fn from_program(p: &ImageProgram) -> Self {
        let has_alpha = p
            .extra_headers
            .iter()
            .any(|h| h.split_whitespace().next() == Some("Alpha"));
        GenCtx {
            value_max: value_max_for(p.bitdepth),
            width: p.width,
            height: p.height,
            channels: if has_alpha { 4 } else { p.channels.max(1) },
        }
    }
}

/// `2^bitdepth - 1`, clamped so the shift can't overflow and is never zero.
fn value_max_for(bitdepth: u32) -> i64 {
    ((1i64 << bitdepth.clamp(1, 30)) - 1).max(1)
}

/// Threshold range class for a condition variable. Picks a sane comparison
/// range so a swapped-in var isn't compared against a wildly off threshold.
#[derive(Debug, Clone, Copy)]
enum VarClass {
    Coord,    // x, y — image coordinates
    Channel,  // c — channel index
    Small,    // g — group index
    Neighbor, // W, N — neighbour pixel values
    Wgh,      // WGH — weighted-predictor error magnitude
    Value,    // gradient / abs / prev value, pixel-valued
    Diff,     // neighbour differences, centred on 0
}

impl VarClass {
    fn sample(self, ctx: &GenCtx, rng: &mut impl Rng) -> i64 {
        let m = ctx.value_max;
        match self {
            // Coord/Channel are normally intercepted by random_threshold_for;
            // these are safe fallbacks.
            VarClass::Coord => {
                rng.gen_range((ctx.width as i64 / 20)..=(ctx.width as i64 * 19 / 20).max(1))
            }
            VarClass::Channel => rng.gen_range(0i64..=(ctx.channels as i64 - 1).max(0)),
            VarClass::Small => rng.gen_range(0i64..=3),
            VarClass::Neighbor => rng.gen_range((-2 * m / 5)..=(6 * m / 5)),
            VarClass::Wgh => rng.gen_range(0i64..=(m / 12).max(1)),
            VarClass::Value => rng.gen_range(0i64..=m),
            VarClass::Diff => {
                let d = (m / 5).max(1);
                rng.gen_range(-d..=d)
            }
        }
    }
}

/// Composite/expression condition variables drawn from the gallery corpus.
/// These are *expressions*, not modelled internals, so they ride along as
/// `Var::Other` and round-trip verbatim through `to_text`. All are confirmed
/// valid as `jxl_from_tree` condition properties (unlike bare neighbour names
/// such as `NE`/`NN`, which are only legal *inside* composites like these).
const COMPOSITE_VARS: &[(&str, VarClass)] = &[
    ("W+N-NW", VarClass::Value), // gradient-predictor expression (most common)
    ("|W|", VarClass::Value),
    ("|N|", VarClass::Value),
    ("N-NE", VarClass::Diff),
    ("NW-N", VarClass::Diff),
    ("W-NW", VarClass::Diff),
    ("N-NN", VarClass::Diff),
    ("W-WW", VarClass::Diff),
    ("W-WW-NW+NWW", VarClass::Diff),
    ("g", VarClass::Small),
    ("Prev", VarClass::Value), // previous-channel value (needs a `c` split to matter)
    ("PrevErr", VarClass::Diff),
];

fn var_class(var: &Var) -> VarClass {
    if let Var::Other(s) = var {
        for (name, class) in COMPOSITE_VARS {
            if *name == s.as_str() {
                return *class;
            }
        }
    }
    match var {
        Var::X | Var::Y => VarClass::Coord,
        Var::C => VarClass::Channel,
        Var::W | Var::N => VarClass::Neighbor,
        Var::WGH => VarClass::Wgh,
        // Unknown pasted expression: treat as pixel-valued.
        Var::Other(_) => VarClass::Value,
    }
}

/// Pick a comparison threshold appropriate to `var`'s value class, scaled to
/// the program's bitdepth / canvas / channel count via `ctx`.
fn random_threshold_for(var: &Var, ctx: &GenCtx, rng: &mut impl Rng) -> i64 {
    match var {
        Var::X => rng.gen_range((ctx.width as i64 / 20)..=(ctx.width as i64 * 19 / 20).max(1)),
        Var::Y => rng.gen_range((ctx.height as i64 / 20)..=(ctx.height as i64 * 19 / 20).max(1)),
        Var::C => rng.gen_range(0i64..=(ctx.channels as i64 - 1).max(0)),
        _ => var_class(var).sample(ctx, rng),
    }
}

fn random_var(rng: &mut impl Rng) -> Var {
    // ~35% of the time use a composite/expression var from the corpus. These
    // parse as Var::Other and round-trip verbatim. Bare exotic neighbour names
    // (NE/NW/NN/NWW/WW) are deliberately absent — they're not in libjxl's
    // condition-property whitelist on their own, only inside these composites.
    if rng.gen_bool(0.35) {
        // `W+N-NW` dominates real-world art, so give it extra weight.
        if rng.gen_bool(0.40) {
            return Var::Other("W+N-NW".to_string());
        }
        let (name, _) = COMPOSITE_VARS[rng.gen_range(0..COMPOSITE_VARS.len())];
        return Var::Other(name.to_string());
    }
    // `c` may nest (e.g. `c > 1` inside `c > 0`); `simplify_degenerate` strips
    // only the *dead* same-property nests (like `c > 1` inside `c > 1`) that
    // djxl can't decode, so we don't need to restrict it here.
    match rng.gen_range(0..6u8) {
        0 => Var::X,
        1 => Var::Y,
        2 => Var::C,
        3 => Var::W,
        4 => Var::N,
        _ => Var::WGH,
    }
}

fn random_predictor(ctx: &GenCtx, rng: &mut impl Rng) -> Predictor {
    // 20% exotic leaf predictor for extra visual range.
    if rng.gen_bool(0.20) {
        return random_exotic_predictor(rng);
    }
    let m = ctx.value_max;
    let off = (m / 8).max(1);
    let offset = rng.gen_range(-off..=off);
    match rng.gen_range(0..7u8) {
        // Signed range (slightly past the ends) so Co/Cg can go negative under
        // RCT-6 — otherwise red and blue are systematically suppressed.
        0 => Predictor::Set(rng.gen_range((-m / 2)..=(m + m / 4))),
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
    const NAMES: &[&str] = &["NE", "NW", "WW", "AvgW+N", "AvgAll", "Gradient", "Select"];
    let name = NAMES[rng.gen_range(0..NAMES.len())];
    let offset = match rng.gen_range(0..3u8) {
        0 => "0".to_string(),
        1 => format!("+ {}", rng.gen_range(1i64..=32)),
        _ => format!("- {}", rng.gen_range(1i64..=32)),
    };
    Predictor::Other {
        name: name.to_string(),
        offset,
    }
}

// ── Tree inspection ───────────────────────────────────────────────────────────

fn collect_thresholds(node: &Node) -> Vec<i64> {
    match node {
        Node::If {
            condition,
            on_true,
            on_false,
        } => {
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
        Node::If {
            on_true, on_false, ..
        } => {
            let mut v = collect_set_values(on_true);
            v.extend(collect_set_values(on_false));
            v
        }
        Node::Predict(Predictor::Set(v)) => vec![*v],
        Node::Predict(_) => vec![],
    }
}

fn collect_offsets(node: &Node) -> Vec<i64> {
    match node {
        Node::If {
            on_true, on_false, ..
        } => {
            let mut v = collect_offsets(on_true);
            v.extend(collect_offsets(on_false));
            v
        }
        Node::Predict(pred) => match pred {
            Predictor::N(o)
            | Predictor::W(o)
            | Predictor::AvgNNW(o)
            | Predictor::AvgNNE(o)
            | Predictor::AvgWNW(o)
            | Predictor::Weighted(o) => vec![*o],
            Predictor::Set(_) => vec![],
            Predictor::Other { .. } => vec![],
        },
    }
}

fn count_conditions(node: &Node) -> usize {
    match node {
        Node::If {
            on_true, on_false, ..
        } => 1 + count_conditions(on_true) + count_conditions(on_false),
        Node::Predict(_) => 0,
    }
}

fn count_predictors(node: &Node) -> usize {
    match node {
        Node::If {
            on_true, on_false, ..
        } => count_predictors(on_true) + count_predictors(on_false),
        Node::Predict(_) => 1,
    }
}

fn count_all_nodes(node: &Node) -> usize {
    match node {
        Node::If {
            on_true, on_false, ..
        } => 1 + count_all_nodes(on_true) + count_all_nodes(on_false),
        Node::Predict(_) => 1,
    }
}

// ── Tree mutation (targeted) ──────────────────────────────────────────────────

/// Apply `f` to the n-th If node's condition (pre-order DFS).
fn apply_nth_condition(
    node: &mut Node,
    n: usize,
    seen: &mut usize,
    f: &mut dyn FnMut(&mut Condition),
) {
    if let Node::If {
        condition,
        on_true,
        on_false,
    } = node
    {
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
    if let Node::If {
        on_true, on_false, ..
    } = node
    {
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
    node: &mut Node,
    n: usize,
    seen: &mut usize,
    f: &mut dyn FnMut(&mut Predictor),
) {
    match node {
        Node::If {
            on_true, on_false, ..
        } => {
            apply_nth_predictor(on_true, n, seen, f);
            apply_nth_predictor(on_false, n, seen, f);
        }
        Node::Predict(pred) => {
            if *seen == n {
                f(pred);
            }
            *seen += 1;
        }
    }
}

/// Apply `f` to the value inside the n-th Set predictor leaf.
fn apply_nth_set_predictor(
    node: &mut Node,
    n: usize,
    seen: &mut usize,
    f: &mut dyn FnMut(&mut i64),
) {
    match node {
        Node::If {
            on_true, on_false, ..
        } => {
            apply_nth_set_predictor(on_true, n, seen, f);
            apply_nth_set_predictor(on_false, n, seen, f);
        }
        Node::Predict(Predictor::Set(v)) => {
            if *seen == n {
                f(v);
            }
            *seen += 1;
        }
        Node::Predict(_) => {}
    }
}

/// Replace the n-th Predict leaf (DFS, on_true before on_false) with
/// `f(old_leaf)`. Used by `InsertIfAtLeaf` to split a leaf into an If.
fn replace_nth_leaf(node: &mut Node, n: usize, seen: &mut usize, f: &mut dyn FnMut(Node) -> Node) {
    match node {
        Node::If {
            on_true, on_false, ..
        } => {
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
fn replace_nth_node(node: &mut Node, n: usize, seen: &mut usize, f: &mut dyn FnMut(Node) -> Node) {
    let idx = *seen;
    *seen += 1;
    if idx == n {
        let old = std::mem::replace(node, Node::Predict(Predictor::Set(0)));
        *node = f(old);
        return;
    }
    if let Node::If {
        on_true, on_false, ..
    } = node
    {
        replace_nth_node(on_true, n, seen, f);
        replace_nth_node(on_false, n, seen, f);
    }
}

/// Replace the n-th If (pre-order DFS) with one of its children. If
/// `take_true` is true the If is replaced by `on_true`; otherwise by
/// `on_false`. Used by `RemoveBranchAt` / `PromoteTrueAt`.
fn collapse_nth_if(node: &mut Node, n: usize, seen: &mut usize, take_true: bool) -> bool {
    if !matches!(node, Node::If { .. }) {
        return false;
    }
    let idx = *seen;
    *seen += 1;
    if idx == n {
        let old = std::mem::replace(node, Node::Predict(Predictor::Set(0)));
        if let Node::If {
            on_true, on_false, ..
        } = old
        {
            *node = if take_true { *on_true } else { *on_false };
        }
        return true;
    }
    if let Node::If {
        on_true, on_false, ..
    } = node
    {
        if collapse_nth_if(on_true, n, seen, take_true) {
            return true;
        }
        if collapse_nth_if(on_false, n, seen, take_true) {
            return true;
        }
    }
    false
}

/// Walk the tree in pre-order, recording the boolean path
/// (`true` = on_true, `false` = on_false) from the root to every node
/// (including the root, whose path is empty).
fn collect_node_paths(node: &Node, prefix: &mut Vec<bool>, out: &mut Vec<Vec<bool>>) {
    out.push(prefix.clone());
    if let Node::If {
        on_true, on_false, ..
    } = node
    {
        prefix.push(true);
        collect_node_paths(on_true, prefix, out);
        prefix.pop();
        prefix.push(false);
        collect_node_paths(on_false, prefix, out);
        prefix.pop();
    }
}

/// True iff `a` is a strict prefix of `b` — i.e. `a` is an ancestor of `b`
/// in the tree. Used to reject swap-subtree pairs that overlap.
fn is_path_ancestor(a: &[bool], b: &[bool]) -> bool {
    a.len() < b.len() && b.starts_with(a)
}

fn get_subtree_mut<'a>(root: &'a mut Node, path: &[bool]) -> &'a mut Node {
    let mut cur = root;
    for &dir in path {
        cur = match cur {
            Node::If {
                on_true, on_false, ..
            } => {
                if dir {
                    on_true
                } else {
                    on_false
                }
            }
            // collect_node_paths only emits paths reachable in the tree, so
            // this is unreachable for any path it produced.
            _ => unreachable!("path runs past a leaf"),
        };
    }
    cur
}

fn swap_subtrees_at(root: &mut Node, a: &[bool], b: &[bool]) {
    // Two sequential mut borrows of `root`, broken by the std::mem::replace
    // call that returns ownership of the prior subtree. The placeholder is
    // a cheap leaf that gets overwritten on the second pass.
    let placeholder = Node::Predict(Predictor::Set(0));
    let subtree_a = std::mem::replace(get_subtree_mut(root, a), placeholder);
    let subtree_b = std::mem::replace(get_subtree_mut(root, b), subtree_a);
    *get_subtree_mut(root, a) = subtree_b;
}

/// Add `delta` to every non-Set predictor offset in the tree.
fn tweak_all_offsets(node: &mut Node, delta: i64) {
    match node {
        Node::Predict(pred) => match pred {
            Predictor::N(o)
            | Predictor::W(o)
            | Predictor::AvgNNW(o)
            | Predictor::AvgNNE(o)
            | Predictor::AvgWNW(o)
            | Predictor::Weighted(o) => *o += delta,
            Predictor::Set(_) => {}
            Predictor::Other { .. } => {}
        },
        Node::If {
            on_true, on_false, ..
        } => {
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
        let nonzero: Vec<i64> = all_values
            .iter()
            .map(|v| v.abs())
            .filter(|&v| v > 0)
            .collect();
        if nonzero.is_empty() {
            10
        } else {
            nonzero.iter().sum::<i64>() / nonzero.len() as i64
        }
    };
    let magnitude = ((base as f64 * scale.abs()).round() as i64).max(1);
    if scale >= 0.0 {
        magnitude
    } else {
        -magnitude
    }
}
