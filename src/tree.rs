use serde::{Deserialize, Serialize};

// ── Variables usable in conditions ─────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Var {
    X,
    Y,
    C,
    /// West (left) neighbour value
    W,
    /// North (above) neighbour value
    N,
    /// Weighted gradient heuristic: abs(W−NW) + abs(N−NW)
    WGH,
}

impl Var {
    pub fn label(&self) -> &'static str {
        match self {
            Var::X => "x",
            Var::Y => "y",
            Var::C => "c",
            Var::W => "W",
            Var::N => "N",
            Var::WGH => "WGH",
        }
    }
}

// ── Operators ───────────────────────────────────────────────────────────────

/// jxl_from_tree only supports `>` comparisons.  All other operators are kept
/// for internal evaluation but must never appear in `to_text()` output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Op {
    Gt,
    Lt,
    Gte,
    Lte,
    Eq,
}

impl Op {
    pub fn eval(&self, lhs: i64, rhs: i64) -> bool {
        match self {
            Op::Gt => lhs > rhs,
            Op::Lt => lhs < rhs,
            Op::Gte => lhs >= rhs,
            Op::Lte => lhs <= rhs,
            Op::Eq => lhs == rhs,
        }
    }

}

// ── Condition ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub var: Var,
    pub op: Op,
    pub threshold: i64,
}

impl Condition {
    /// Serialise to jxl-art text.  Non-`>` operators are converted to the
    /// closest valid equivalent so we never emit invalid syntax.
    pub fn label(&self) -> String {
        let threshold = match self.op {
            // `prop < T`  →  `prop > (T - 1)`  (equivalent for integers)
            Op::Lt => self.threshold - 1,
            // `prop >= T` →  `prop > (T - 1)`
            Op::Gte => self.threshold - 1,
            // `prop <= T` →  `prop > T` is wrong, but we have no `<=` in
            // jxl-art, so we use `prop > (T - 1)` as the best we can do.
            Op::Lte => self.threshold,
            // `prop == T` →  can't express; fall back to `prop > (T - 1)`
            Op::Eq => self.threshold - 1,
            Op::Gt => self.threshold,
        };
        format!("{} > {}", self.var.label(), threshold)
    }

    pub fn eval(&self, ctx: &Context) -> bool {
        let lhs = ctx.resolve_var(&self.var);
        self.op.eval(lhs, self.threshold)
    }
}

// ── Neighbours context ──────────────────────────────────────────────────────

/// All the values needed to evaluate a condition or predictor for one sample.
#[derive(Debug, Clone)]
pub struct Context {
    pub x: u32,
    pub y: u32,
    pub c: u32,
    /// North sample (same channel, row above). 0 at top edge.
    pub n: i64,
    /// West sample (same channel, column to the left). 0 at left edge.
    pub w: i64,
    /// North-west sample. 0 at edges.
    pub nw: i64,
    /// North-east sample. 0 at edges.
    pub ne: i64,
}

impl Context {
    pub fn resolve_var(&self, var: &Var) -> i64 {
        match var {
            Var::X => self.x as i64,
            Var::Y => self.y as i64,
            Var::C => self.c as i64,
            Var::W => self.w,
            Var::N => self.n,
            Var::WGH => (self.w - self.nw).abs() + (self.n - self.nw).abs(),
        }
    }
}

// ── Predictors ──────────────────────────────────────────────────────────────

/// How a leaf node computes the sample value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Predictor {
    /// Absolute literal value.
    Set(i64),
    /// North neighbour + offset.
    N(i64),
    /// West neighbour + offset.
    W(i64),
    /// avg(N, NW) + offset.
    AvgNNW(i64),
    /// avg(N, NE) + offset.
    AvgNNE(i64),
    /// avg(W, NW) + offset.
    AvgWNW(i64),
    /// Median-edge-detector: clamp(W+N−NW, min(W,N), max(W,N)) + offset.
    Weighted(i64),
}

impl Predictor {
    pub fn eval(&self, ctx: &Context) -> i64 {
        match self {
            Predictor::Set(v) => *v,
            Predictor::N(o) => ctx.n + o,
            Predictor::W(o) => ctx.w + o,
            Predictor::AvgNNW(o) => (ctx.n + ctx.nw) / 2 + o,
            Predictor::AvgNNE(o) => (ctx.n + ctx.ne) / 2 + o,
            Predictor::AvgWNW(o) => (ctx.w + ctx.nw) / 2 + o,
            Predictor::Weighted(o) => {
                let pred = (ctx.w + ctx.n - ctx.nw)
                    .clamp(ctx.w.min(ctx.n), ctx.w.max(ctx.n));
                pred + o
            }
        }
    }

    pub fn label(&self) -> String {
        fn fmt_offset(o: i64) -> String {
            if o >= 0 {
                format!("+ {}", o)
            } else {
                format!("- {}", o.abs())
            }
        }
        match self {
            Predictor::Set(v) => format!("Set {}", v),
            Predictor::N(o) => {
                if *o == 0 {
                    "N 0".to_string()
                } else {
                    format!("N {}", fmt_offset(*o))
                }
            }
            Predictor::W(o) => format!("W {}", fmt_offset(*o)),
            Predictor::AvgNNW(o) => format!("AvgN+NW {}", fmt_offset(*o)),
            Predictor::AvgNNE(o) => format!("AvgN+NE {}", fmt_offset(*o)),
            Predictor::AvgWNW(o) => format!("AvgW+NW {}", fmt_offset(*o)),
            Predictor::Weighted(o) => format!("Weighted {}", fmt_offset(*o)),
        }
    }
}

// ── Nodes ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Node {
    If {
        condition: Condition,
        body: Vec<Node>,
    },
    Predict(Predictor),
}

impl Node {
    pub fn execute(&self, ctx: &Context, value: &mut i64) {
        match self {
            Node::If { condition, body } => {
                if condition.eval(ctx) {
                    for child in body {
                        child.execute(ctx, value);
                    }
                }
            }
            Node::Predict(pred) => {
                *value = pred.eval(ctx);
            }
        }
    }
}

// ── Image program ───────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageProgram {
    pub width: u32,
    pub height: u32,
    pub bitdepth: u32,
    pub channels: u32,
    pub orientation: Option<u32>,
    pub rct: Option<u32>,
    pub nodes: Vec<Node>,
}

impl ImageProgram {
    /// Run the tree for a single sample — returns the raw (unclamped) value so
    /// that any colour-space transform can be applied before the final clamp.
    pub fn eval_with_context(&self, ctx: &Context) -> i64 {
        let mut value: i64 = 0;
        for node in &self.nodes {
            node.execute(ctx, &mut value);
        }
        value
    }

    /// Render to a flat RGBA byte buffer (8 bpc, 4 channels).
    ///
    /// Pipeline:
    ///   1. Evaluate the tree per sample with **no clamping**, storing raw i64
    ///      values so that neighbour predictors propagate correct values.
    ///   2. Apply the inverse RCT colour transform (if `self.rct` is set).
    ///      For RCT 6 (YCoCg-R, the most common jxl-art setting) the stored
    ///      channels are (Y, Co, Cg) and the transform yields (R, G, B).
    ///   3. Normalise to [0, 255] and write the RGBA output.
    pub fn render_rgba(&self) -> Vec<u8> {
        let (w, h, ch) = (self.width, self.height, self.channels);
        let max_val = (1i64 << self.bitdepth) - 1;

        // ── Stage 1: evaluate tree, no clamping ─────────────────────────────
        let mut rendered = vec![0i64; (w * h * ch) as usize];

        let sample = |buf: &[i64], x: i32, y: i32, c: u32| -> i64 {
            if x < 0 || y < 0 || x >= w as i32 || y >= h as i32 {
                return 0;
            }
            buf[(y as u32 * w * ch + x as u32 * ch + c) as usize]
        };

        for y in 0..h {
            for x in 0..w {
                for c in 0..ch {
                    let ctx = Context {
                        x,
                        y,
                        c,
                        n: sample(&rendered, x as i32, y as i32 - 1, c),
                        w: sample(&rendered, x as i32 - 1, y as i32, c),
                        nw: sample(&rendered, x as i32 - 1, y as i32 - 1, c),
                        ne: sample(&rendered, x as i32 + 1, y as i32 - 1, c),
                    };
                    let val = self.eval_with_context(&ctx);
                    rendered[(y * w * ch + x * ch + c) as usize] = val;
                }
            }
        }

        // ── Stage 2: inverse RCT colour transform ────────────────────────────
        if ch >= 3 {
            match self.rct {
                // RCT 6 — YCoCg-R (jxl-art default)
                // Stored as (Y=ch0, Co=ch1, Cg=ch2); inverse gives (R, G, B).
                //   tmp = Y - (Cg >> 1)
                //   G   = Cg + tmp
                //   B   = tmp - (Co >> 1)
                //   R   = B + Co
                Some(6) => {
                    for y in 0..h {
                        for x in 0..w {
                            let base = (y * w * ch + x * ch) as usize;
                            let (y_val, co, cg) =
                                (rendered[base], rendered[base + 1], rendered[base + 2]);
                            let tmp = y_val - (cg >> 1);
                            let g = cg + tmp;
                            let b = tmp - (co >> 1);
                            let r = b + co;
                            rendered[base] = r;
                            rendered[base + 1] = g;
                            rendered[base + 2] = b;
                        }
                    }
                }
                // RCT 0 or None — already in RGB, nothing to do.
                _ => {}
            }
        }

        // ── Stage 3: normalise to u8 and write RGBA ──────────────────────────
        let mut pixels = vec![255u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let dst = ((y * w + x) * 4) as usize;
                for c in 0..ch.min(3) {
                    let raw = rendered[(y * w * ch + x * ch + c) as usize];
                    let clamped = raw.clamp(0, max_val) as f32;
                    pixels[dst + c as usize] =
                        ((clamped / max_val as f32) * 255.0).round() as u8;
                }
            }
        }

        // ── Stage 4: scale to 1024×1024 ──────────────────────────────────────
        const TARGET: u32 = 1024;
        if w == TARGET && h == TARGET {
            return pixels;
        }
        use image::imageops::FilterType;
        let img = image::RgbaImage::from_raw(w, h, pixels).expect("valid pixel buffer");
        let resized = image::imageops::resize(&img, TARGET, TARGET, FilterType::Nearest);
        resized.into_raw()
    }

    pub fn example() -> Self {
        use Node::{If, Predict};
        use Predictor::*;

        let cond = |var, op, threshold| Condition { var, op, threshold };

        ImageProgram {
            width: 1024,
            height: 1024,
            bitdepth: 8,
            channels: 3,
            orientation: Some(7),
            rct: Some(6),
            nodes: vec![If {
                condition: cond(Var::Y, Op::Gt, 150),
                body: vec![
                    // ── if c > 0 ─────────────────────────────────────────
                    If {
                        condition: cond(Var::C, Op::Gt, 0),
                        body: vec![
                            Predict(N(0)),
                            If {
                                condition: cond(Var::X, Op::Gt, 500),
                                body: vec![
                                    If {
                                        condition: cond(Var::WGH, Op::Gt, 5),
                                        body: vec![
                                            Predict(AvgNNW(2)),
                                            Predict(AvgNNE(-2)),
                                        ],
                                    },
                                    If {
                                        condition: cond(Var::X, Op::Gt, 470),
                                        body: vec![
                                            Predict(AvgWNW(-2)),
                                            If {
                                                condition: cond(Var::WGH, Op::Gt, 0),
                                                body: vec![
                                                    Predict(AvgNNW(1)),
                                                    Predict(AvgNNE(-1)),
                                                ],
                                            },
                                        ],
                                    },
                                ],
                            },
                        ],
                    },
                    // ── if y > 136 ───────────────────────────────────────
                    If {
                        condition: cond(Var::Y, Op::Gt, 136),
                        body: vec![
                            If {
                                condition: cond(Var::C, Op::Gt, 0),
                                body: vec![
                                    If {
                                        condition: cond(Var::C, Op::Gt, 1),
                                        body: vec![
                                            If {
                                                condition: cond(Var::X, Op::Gt, 500),
                                                body: vec![
                                                    Predict(Set(-20)),
                                                    Predict(Set(40)),
                                                ],
                                            },
                                            If {
                                                condition: cond(Var::X, Op::Gt, 501),
                                                body: vec![
                                                    Predict(W(-1)),
                                                    Predict(Set(150)),
                                                ],
                                            },
                                        ],
                                    },
                                    If {
                                        condition: cond(Var::X, Op::Gt, 500),
                                        body: vec![
                                            Predict(N(5)),
                                            Predict(N(-15)),
                                        ],
                                    },
                                ],
                            },
                            If {
                                condition: cond(Var::W, Op::Gt, -50),
                                body: vec![
                                    Predict(Weighted(-1)),
                                    Predict(Set(320)),
                                ],
                            },
                        ],
                    },
                ],
            }],
        }
    }

    /// Serialise to the human-readable text format.
    pub fn to_text(&self) -> String {
        let mut out = format!("Bitdepth {}\n", self.bitdepth);
        if let Some(o) = self.orientation {
            out.push_str(&format!("Orientation {}\n", o));
        }
        if let Some(r) = self.rct {
            out.push_str(&format!("RCT {}\n", r));
        }
        // Width/Height default to 1024×1024 in jxl-art; only emit if different.
        if self.width != 1024 || self.height != 1024 {
            out.push_str(&format!("Width {}\nHeight {}\n", self.width, self.height));
        }
        out.push('\n');
        for node in &self.nodes {
            write_node(&mut out, node, 0);
        }
        out
    }
}

fn write_node(out: &mut String, node: &Node, depth: usize) {
    let indent = "  ".repeat(depth);
    match node {
        Node::If { condition, body } => {
            out.push_str(&format!("{}if {}\n", indent, condition.label()));
            for child in body {
                write_node(out, child, depth + 1);
            }
        }
        Node::Predict(pred) => {
            out.push_str(&format!("{}- {}\n", indent, pred.label()));
        }
    }
}
