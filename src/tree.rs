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
    /// Max absolute transition error from the weighted predictor state.
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
            Op::Gt  => lhs > rhs,
            Op::Lt  => lhs < rhs,
            Op::Gte => lhs >= rhs,
            Op::Lte => lhs <= rhs,
            Op::Eq  => lhs == rhs,
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
            Op::Lt  => self.threshold - 1,
            Op::Gte => self.threshold - 1,
            Op::Lte => self.threshold,
            Op::Eq  => self.threshold - 1,
            Op::Gt  => self.threshold,
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
    pub n: i64,
    pub w: i64,
    pub nw: i64,
    pub ne: i64,
    /// WGH property: max |transition error| from the weighted predictor state.
    pub wgh: i64,
    /// Prediction from the weighted predictor (used by Predictor::Weighted).
    pub wp_pred: i64,
}

impl Context {
    pub fn resolve_var(&self, var: &Var) -> i64 {
        match var {
            Var::X   => self.x as i64,
            Var::Y   => self.y as i64,
            Var::C   => self.c as i64,
            Var::W   => self.w,
            Var::N   => self.n,
            Var::WGH => self.wgh,
        }
    }
}

// ── Weighted predictor state (libjxl context_predict.h port) ────────────────

const WP_EXTRA_BITS: i64 = 3;
const WP_ROUND: i64 = ((1i64 << WP_EXTRA_BITS) >> 1) - 1; // 3
const WP_NUM: usize = 4;
// Default header constants
const WP_P1C: i64 = 16;
const WP_P2C: i64 = 10;
const WP_P3CA: i64 = 7;
const WP_P3CB: i64 = 7;
const WP_P3CC: i64 = 7;
const WP_P3CD: i64 = 0;
const WP_P3CE: i64 = 0;
const WP_MAXW: [u32; WP_NUM] = [0xd, 0xc, 0xc, 0xc];
#[rustfmt::skip]
const WP_DIV: [u32; 64] = [
    16777216,8388608,5592405,4194304,3355443,2796202,2396745,2097152,
    1864135,1677721,1525201,1398101,1290555,1198372,1118481,1048576,
    986895,932067,883011,838860,798915,762600,729444,699050,
    671088,645277,621378,599186,578524,559240,541200,524288,
    508400,493447,479349,466033,453438,441505,430185,419430,
    409200,399457,390167,381300,372827,364722,356962,349525,
    342392,335544,328965,322638,316551,310689,305040,299593,
    294337,289262,284359,279620,275036,270600,266305,262144,
];

struct WpState {
    xsize: usize,
    /// Two-row circular buffer of accumulated prediction errors per predictor.
    pred_err: [Vec<u32>; WP_NUM],
    /// Two-row circular buffer of prediction residuals (pred - actual, shifted).
    err: Vec<i32>,
    /// Last computed sub-predictions (kept for UpdateErrors).
    preds: [i64; WP_NUM],
    /// Last weighted average prediction (kept for UpdateErrors).
    pred: i64,
}

impl WpState {
    fn new(xsize: usize) -> Self {
        let sz = (xsize + 2) * 2;
        WpState {
            xsize,
            pred_err: [vec![0u32; sz], vec![0u32; sz], vec![0u32; sz], vec![0u32; sz]],
            err: vec![0i32; sz],
            preds: [0i64; WP_NUM],
            pred: 0,
        }
    }

    fn floor_log2(x: u64) -> u32 {
        63u32.saturating_sub(x.leading_zeros())
    }

    fn error_weight(x: u64, maxw: u32) -> u32 {
        let log = Self::floor_log2(x + 1) as i32;
        let shift = (log - 5).max(0) as u32;
        let idx = ((x >> shift) as usize).min(63);
        4 + ((maxw as u64 * WP_DIV[idx] as u64) >> shift) as u32
    }

    fn weighted_avg(p: [i64; WP_NUM], mut w: [u32; WP_NUM]) -> i64 {
        let mut ws: u32 = w.iter().sum();
        let log_ws = Self::floor_log2(ws as u64);
        let shift = log_ws.saturating_sub(4);
        ws = 0;
        for wi in &mut w { *wi >>= shift; ws += *wi; }
        let mut sum = (ws as i64 >> 1) - 1;
        for i in 0..WP_NUM { sum += p[i] * w[i] as i64; }
        let idx = (ws as usize - 1).min(63);
        (sum * WP_DIV[idx] as i64) >> 24
    }

    /// Compute the weighted prediction and WGH property for pixel (x, y).
    /// `n/w/ne/nw/nn` are already edge-corrected neighbour values.
    /// Returns `(wp_prediction, wgh)`.
    fn predict(&mut self, x: usize, y: usize,
               n: i64, w: i64, ne: i64, nw: i64, nn: i64) -> (i64, i64) {
        let xs = self.xsize;
        let cur = if y & 1 == 1 { 0 } else { xs + 2 };
        let prv = if y & 1 == 1 { xs + 2 } else { 0 };
        let pn  = prv + x;
        let pne = if x + 1 < xs { pn + 1 } else { pn };
        let pnw = if x > 0 { pn - 1 } else { pn };

        let weights: [u32; WP_NUM] = std::array::from_fn(|i| {
            let s = self.pred_err[i][pn] as u64
                  + self.pred_err[i][pne] as u64
                  + self.pred_err[i][pnw] as u64;
            Self::error_weight(s, WP_MAXW[i])
        });

        let (n, w, ne, nw, nn) = (
            n << WP_EXTRA_BITS, w << WP_EXTRA_BITS,
            ne << WP_EXTRA_BITS, nw << WP_EXTRA_BITS,
            nn << WP_EXTRA_BITS,
        );

        let te_w  = if x == 0 { 0i64 } else { self.err[cur + x - 1] as i64 };
        let te_n  = self.err[pn]  as i64;
        let te_nw = self.err[pnw] as i64;
        let te_ne = self.err[pne] as i64;
        let sum_wn = te_n + te_w;

        // WGH = max absolute transition error
        let wgh = [te_w, te_n, te_nw, te_ne].into_iter()
            .max_by_key(|v| v.unsigned_abs())
            .unwrap_or(0);

        self.preds[0] = w + ne - n;
        self.preds[1] = n - ((sum_wn + te_ne) * WP_P1C >> 5);
        self.preds[2] = w - ((sum_wn + te_nw) * WP_P2C >> 5);
        self.preds[3] = n - ((te_nw * WP_P3CA + te_n * WP_P3CB + te_ne * WP_P3CC
                            + (nn - n) * WP_P3CD + (nw - w) * WP_P3CE) >> 5);

        self.pred = Self::weighted_avg(self.preds, weights);

        // Apply clamping unless all three errors agree in sign and differ from each other
        let result = if ((te_n ^ te_w) | (te_n ^ te_nw)) > 0 {
            (self.pred + WP_ROUND) >> WP_EXTRA_BITS
        } else {
            let mx = w.max(ne).max(n);
            let mn = w.min(ne).min(n);
            ((self.pred.clamp(mn, mx)) + WP_ROUND) >> WP_EXTRA_BITS
        };

        (result, wgh)
    }

    /// Update error state after decoding/evaluating pixel (x, y) = `val`.
    fn update(&mut self, val: i64, x: usize, y: usize) {
        let xs = self.xsize;
        let cur = if y & 1 == 1 { 0 } else { xs + 2 };
        let prv = if y & 1 == 1 { xs + 2 } else { 0 };
        let val_fp = val << WP_EXTRA_BITS;
        self.err[cur + x] = (self.pred - val_fp) as i32;
        for i in 0..WP_NUM {
            let e = ((self.preds[i] - val_fp).unsigned_abs() as i64
                     + WP_ROUND) >> WP_EXTRA_BITS;
            let e = e as u32;
            self.pred_err[i][cur + x] = e;
            self.pred_err[i][prv + x + 1] += e;
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
    /// Libjxl weighted predictor + offset.
    Weighted(i64),
}

impl Predictor {
    pub fn eval(&self, ctx: &Context) -> i64 {
        match self {
            Predictor::Set(v)    => *v,
            Predictor::N(o)      => ctx.n + o,
            Predictor::W(o)      => ctx.w + o,
            Predictor::AvgNNW(o) => (ctx.n + ctx.nw) / 2 + o,
            Predictor::AvgNNE(o) => (ctx.n + ctx.ne) / 2 + o,
            Predictor::AvgWNW(o) => (ctx.w + ctx.nw) / 2 + o,
            Predictor::Weighted(o) => ctx.wp_pred + o,
        }
    }

    pub fn label(&self) -> String {
        fn fmt_offset(o: i64) -> String {
            if o >= 0 { format!("+ {}", o) } else { format!("- {}", o.abs()) }
        }
        fn fmt_pred(name: &str, o: i64) -> String {
            if o == 0 { format!("{} 0", name) } else { format!("{} {}", name, fmt_offset(o)) }
        }
        match self {
            Predictor::Set(v)    => format!("Set {}", v),
            Predictor::N(o)      => fmt_pred("N", *o),
            Predictor::W(o)      => fmt_pred("W", *o),
            Predictor::AvgNNW(o) => fmt_pred("AvgN+NW", *o),
            Predictor::AvgNNE(o) => fmt_pred("AvgN+NE", *o),
            Predictor::AvgWNW(o) => fmt_pred("AvgW+NW", *o),
            Predictor::Weighted(o) => fmt_pred("Weighted", *o),
        }
    }
}

// ── Nodes ────────────────────────────────────────────────────────────────────

/// A binary decision tree node matching jxl_from_tree's ParseNode semantics:
/// each `If` has exactly one TRUE child (`on_true`) and one FALSE child
/// (`on_false`).  Evaluation always traverses to exactly one `Predict` leaf.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Node {
    If {
        condition: Condition,
        on_true:  Box<Node>,
        on_false: Box<Node>,
    },
    Predict(Predictor),
}

impl Node {
    /// Traverse the tree for a single sample.  Returns the value of the one
    /// `Predict` leaf that the condition path leads to.
    pub fn eval(&self, ctx: &Context) -> i64 {
        match self {
            Node::If { condition, on_true, on_false } => {
                if condition.eval(ctx) {
                    on_true.eval(ctx)
                } else {
                    on_false.eval(ctx)
                }
            }
            Node::Predict(pred) => pred.eval(ctx),
        }
    }
}

/// Returns true if the tree contains any `Var::WGH` condition or
/// `Predictor::Weighted` leaf — i.e. whether `WpState` needs to be computed.
fn needs_wp(node: &Node) -> bool {
    match node {
        Node::If { condition, on_true, on_false } => {
            condition.var == Var::WGH || needs_wp(on_true) || needs_wp(on_false)
        }
        Node::Predict(Predictor::Weighted(_)) => true,
        Node::Predict(_) => false,
    }
}

// ── Image program ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImageProgram {
    pub width: u32,
    pub height: u32,
    pub bitdepth: u32,
    pub channels: u32,
    pub orientation: Option<u32>,
    pub rct: Option<u32>,
    pub root: Node,
}

impl ImageProgram {
    /// Run the tree for a single sample.
    pub fn eval_with_context(&self, ctx: &Context) -> i64 {
        self.root.eval(ctx)
    }

    /// Render to a flat RGBA byte buffer (8 bpc, 4 channels).
    pub fn render_rgba(&self) -> Vec<u8> {
        self.render_rgba_at(self.width, self.height)
    }

    /// Render at `out_w × out_h` pixels.  Tree conditions receive coordinates
    /// scaled to the program's native `width × height` so absolute thresholds
    /// (e.g. `x > 500`) still fire at the same relative position in the image.
    ///
    /// If `out_w == self.width && out_h == self.height` this is identical to
    /// `render_rgba`.  Use `render_display_preview` for gallery thumbnails.
    fn render_rgba_at(&self, out_w: u32, out_h: u32) -> Vec<u8> {
        let (native_w, native_h) = (self.width, self.height);
        let ch = self.channels as usize;
        let ow = out_w as usize;
        let oh = out_h as usize;
        let max_val = (1i64 << self.bitdepth) - 1;

        // If the tree never uses WGH or Weighted we can skip the expensive
        // WpState computation (~40 ops + 13 array reads) for every pixel.
        let use_wp = needs_wp(&self.root);
        let same_w = out_w == native_w;
        let same_h = out_h == native_h;

        // i32 is plenty for bitdepth ≤ 16 even after RCT inflation, and halves
        // cache pressure vs. the previous i64 buffer.
        let stride = ow * ch;
        let mut rendered: Vec<i32> = vec![0; stride * oh];
        let mut wp: Vec<WpState> = (0..ch).map(|_| WpState::new(ow)).collect();

        for y in 0..oh {
            let row = y * stride;
            let prev_row = row.wrapping_sub(stride);
            let prev2_row = row.wrapping_sub(2 * stride);
            let cy = if same_h { y as u32 } else { (y as u64 * native_h as u64 / out_h as u64) as u32 };

            for x in 0..ow {
                let cx = if same_w { x as u32 } else { (x as u64 * native_w as u64 / out_w as u64) as u32 };
                let base = row + x * ch;
                let p_base = prev_row + x * ch;
                let p_base_l = prev_row + x.wrapping_sub(1) * ch;
                let p_base_r = prev_row + (x + 1) * ch;
                let p2_base = prev2_row + x * ch;

                for c in 0..ch {
                    let left: i32 = if x > 0 {
                        rendered[base - ch + c]
                    } else if y > 0 {
                        rendered[p_base + c]
                    } else {
                        0
                    };
                    let top: i32      = if y > 0 { rendered[p_base + c] } else { left };
                    let topleft: i32  = if x > 0 && y > 0 { rendered[p_base_l + c] } else { left };
                    let topright: i32 = if x + 1 < ow && y > 0 { rendered[p_base_r + c] } else { top };
                    let toptop: i32   = if y > 1 { rendered[p2_base + c] } else { top };

                    let (wp_pred, wgh) = if use_wp {
                        wp[c].predict(
                            x, y,
                            top as i64, left as i64, topright as i64, topleft as i64, toptop as i64,
                        )
                    } else {
                        (0, 0)
                    };

                    let ctx = Context {
                        x: cx, y: cy, c: c as u32,
                        n: top as i64, w: left as i64, nw: topleft as i64, ne: topright as i64,
                        wgh, wp_pred,
                    };
                    let val = self.eval_with_context(&ctx);
                    rendered[base + c] = val as i32;

                    if use_wp {
                        wp[c].update(val, x, y);
                    }
                }
            }
        }

        // Inverse RCT colour transform
        if ch >= 3 {
            if let Some(6) = self.rct {
                for y in 0..oh {
                    let row = y * stride;
                    for x in 0..ow {
                        let base = row + x * ch;
                        let y_val = rendered[base];
                        let co    = rendered[base + 1];
                        let cg    = rendered[base + 2];
                        let tmp = y_val - (cg >> 1);
                        let g   = cg + tmp;
                        let b   = tmp - (co >> 1);
                        let r   = b + co;
                        rendered[base]     = r;
                        rendered[base + 1] = g;
                        rendered[base + 2] = b;
                    }
                }
            }
        }

        let mut pixels = vec![255u8; ow * oh * 4];
        let cmax = ch.min(3);
        if max_val == 255 {
            // 8-bit fast path: direct clamp to u8.
            for y in 0..oh {
                let src_row = y * stride;
                let dst_row = y * ow * 4;
                for x in 0..ow {
                    let src = src_row + x * ch;
                    let dst = dst_row + x * 4;
                    for c in 0..cmax {
                        pixels[dst + c] = rendered[src + c].clamp(0, 255) as u8;
                    }
                }
            }
        } else {
            // Deeper bitdepth: integer scale to u8 (matches old f32 path within ±1 ULP).
            let max_u = max_val as u32;
            let half = max_u / 2;
            let max_i = max_val as i32;
            for y in 0..oh {
                let src_row = y * stride;
                let dst_row = y * ow * 4;
                for x in 0..ow {
                    let src = src_row + x * ch;
                    let dst = dst_row + x * 4;
                    for c in 0..cmax {
                        let raw = rendered[src + c].clamp(0, max_i) as u32;
                        pixels[dst + c] = ((raw * 255 + half) / max_u) as u8;
                    }
                }
            }
        }
        pixels
    }

    /// A 4-colour quadrant image (direct RGB, no RCT). Kept for reference.
    #[allow(dead_code)]
    /// Top-left: blue-purple  Top-right: orange
    /// Bottom-left: lime      Bottom-right: rose
    pub fn example() -> Self {
        fn cond(var: Var, threshold: i64) -> Condition {
            Condition { var, op: Op::Gt, threshold }
        }
        // Leaf subtree: select R/G/B by channel index.
        // c > 1 → B, else (c > 0 → G, else R)
        fn color(r: i64, g: i64, b: i64) -> Node {
            Node::If {
                condition: cond(Var::C, 1),
                on_true:  Box::new(Node::Predict(Predictor::Set(b))),
                on_false: Box::new(Node::If {
                    condition: cond(Var::C, 0),
                    on_true:  Box::new(Node::Predict(Predictor::Set(g))),
                    on_false: Box::new(Node::Predict(Predictor::Set(r))),
                }),
            }
        }

        let root = Node::If {
            condition: cond(Var::Y, 511),
            on_true: Box::new(Node::If {        // bottom half
                condition: cond(Var::X, 511),
                on_true:  Box::new(color(200,  50, 130)), // bottom-right: rose
                on_false: Box::new(color( 50, 200,  80)), // bottom-left:  lime
            }),
            on_false: Box::new(Node::If {       // top half
                condition: cond(Var::X, 511),
                on_true:  Box::new(color(220, 130,  30)), // top-right: orange
                on_false: Box::new(color( 80,  30, 180)), // top-left:  blue-purple
            }),
        };

        ImageProgram { width: 1024, height: 1024, bitdepth: 8, channels: 3,
                       orientation: None, rct: None, root }
    }

    /// The original jxl-art default program.
    pub fn example_jxlart() -> Self {
        Self::from_text(include_str!("../gallery/00-sky-and-grass.jxlart"))
            .expect("example_jxlart is always valid")
    }

    /// Render in display order at full native resolution.
    /// Returns `(pixels, display_width, display_height)`.
    pub fn render_display(&self) -> (Vec<u8>, u32, u32) {
        let pixels = self.render_rgba();
        self.apply_orientation(pixels, self.width, self.height)
    }

    /// Render with the longest edge scaled to `max_dim` pixels.
    /// Tree conditions are evaluated in the native coordinate space so
    /// absolute thresholds (e.g. `x > 500`) split at the same relative
    /// position regardless of output size.
    /// Falls through to `render_display` when `max_dim` matches the native size.
    pub fn render_display_at(&self, max_dim: u32) -> (Vec<u8>, u32, u32) {
        let src_max = self.width.max(self.height);
        let out_w = ((self.width  as u64 * max_dim as u64 / src_max as u64) as u32).max(1);
        let out_h = ((self.height as u64 * max_dim as u64 / src_max as u64) as u32).max(1);
        if out_w == self.width && out_h == self.height {
            return self.render_display();
        }
        let pixels = self.render_rgba_at(out_w, out_h);
        self.apply_orientation(pixels, out_w, out_h)
    }

    /// Apply the EXIF orientation transform to a flat RGBA pixel buffer.
    /// `coded_w × coded_h` are the buffer dimensions before the transform.
    ///
    /// EXIF/JXL orientation semantics — `display(dx,dy) = coded(cx,cy)`:
    ///   1: identity          2: flip horizontal   3: rotate 180
    ///   4: flip vertical     5: anti-transpose    6: rotate 90 CW
    ///   7: transpose         8: rotate 90 CCW
    fn apply_orientation(&self, pixels: Vec<u8>, coded_w: u32, coded_h: u32) -> (Vec<u8>, u32, u32) {
        let orient = self.orientation.unwrap_or(1);
        let (w, h) = (coded_w, coded_h);

        let resample = |dw: u32, dh: u32, coded: &dyn Fn(u32, u32) -> usize| -> Vec<u8> {
            let mut out = vec![0u8; (dw * dh * 4) as usize];
            for dy in 0..dh {
                for dx in 0..dw {
                    let src = coded(dx, dy);
                    let dst = ((dy * dw + dx) * 4) as usize;
                    out[dst..dst + 4].copy_from_slice(&pixels[src..src + 4]);
                }
            }
            out
        };

        match orient {
            2 => (resample(w, h, &|dx, dy| ((dy * w + (w-1-dx)) * 4) as usize), w, h),
            3 => (resample(w, h, &|dx, dy| (((h-1-dy) * w + (w-1-dx)) * 4) as usize), w, h),
            4 => (resample(w, h, &|dx, dy| (((h-1-dy) * w + dx) * 4) as usize), w, h),
            5 => (resample(h, w, &|dx, dy| ((dx * w + dy) * 4) as usize), h, w),
            6 => (resample(h, w, &|dx, dy| (((h-1-dx) * w + dy) * 4) as usize), h, w),
            7 => (resample(h, w, &|dx, dy| (((h-1-dx) * w + (w-1-dy)) * 4) as usize), h, w),
            8 => (resample(h, w, &|dx, dy| ((dx * w + (w-1-dy)) * 4) as usize), h, w),
            _ => (pixels, w, h),
        }
    }

    /// Serialise to the human-readable jxl-art text format.
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
        write_node(&mut out, &self.root, 0);
        out
    }
}

fn write_node(out: &mut String, node: &Node, depth: usize) {
    let indent = "  ".repeat(depth);
    match node {
        Node::If { condition, on_true, on_false } => {
            out.push_str(&format!("{}if {}\n", indent, condition.label()));
            write_node(out, on_true,  depth + 1);
            write_node(out, on_false, depth + 1);
        }
        Node::Predict(pred) => {
            out.push_str(&format!("{}- {}\n", indent, pred.label()));
        }
    }
}

// ── Parser ────────────────────────────────────────────────────────────────────

impl ImageProgram {
    /// Parse a jxl-art `.xl` text program.
    ///
    /// The body is tokenised with `split_whitespace` (identical to jxl_from_tree's
    /// `*f >> out` tokeniser), so indentation is entirely cosmetic and ignored.
    pub fn from_text(s: &str) -> Result<Self, String> {
        let mut bitdepth: u32    = 8;
        let mut width: u32       = 1024;
        let mut height: u32      = 1024;
        let mut channels: u32    = 3;
        let mut orientation: Option<u32> = None;
        let mut rct: Option<u32>         = None;

        let lines: Vec<&str> = s.lines().collect();
        let mut body_start = lines.len(); // default: empty body

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                body_start = i + 1;
                break;
            }
            let mut parts = trimmed.splitn(2, ' ');
            let key = parts.next().unwrap_or("");
            let val = parts.next().unwrap_or("").trim();
            match key {
                "Bitdepth"    => bitdepth     = val.parse().map_err(|_| format!("bad Bitdepth: {}", val))?,
                "Width"       => width        = val.parse().map_err(|_| format!("bad Width: {}", val))?,
                "Height"      => height       = val.parse().map_err(|_| format!("bad Height: {}", val))?,
                "Channels"    => channels     = val.parse().map_err(|_| format!("bad Channels: {}", val))?,
                "Orientation" => orientation  = Some(val.parse().map_err(|_| format!("bad Orientation: {}", val))?),
                "RCT"         => rct          = Some(val.parse().map_err(|_| format!("bad RCT: {}", val))?),
                _ => {
                    body_start = i;
                    break;
                }
            }
        }

        // Flatten all body lines into a token stream (ignoring indentation/blank lines)
        let tokens: Vec<&str> = lines[body_start..]
            .iter()
            .flat_map(|line| line.split_whitespace())
            .collect();

        if tokens.is_empty() {
            return Err("program has no tree body".to_string());
        }

        let mut pos = 0usize;
        let root = parse_node(&tokens, &mut pos)?;

        Ok(ImageProgram { width, height, bitdepth, channels, orientation, rct, root })
    }
}

/// Recursively parse one node (if-branch or predict leaf) from the token stream.
fn parse_node(tokens: &[&str], pos: &mut usize) -> Result<Node, String> {
    let tok = *tokens.get(*pos)
        .ok_or_else(|| "unexpected end of input while parsing node".to_string())?;
    *pos += 1;

    match tok {
        "if" => {
            let var_str = *tokens.get(*pos)
                .ok_or("expected variable name after 'if'")?;
            *pos += 1;
            let op_str = *tokens.get(*pos)
                .ok_or("expected operator after variable")?;
            *pos += 1;
            let thr_str = *tokens.get(*pos)
                .ok_or("expected threshold value")?;
            *pos += 1;

            if op_str != ">" {
                return Err(format!("only '>' operator is supported, got '{}'", op_str));
            }
            let var = parse_var(var_str)?;
            let threshold: i64 = thr_str.parse()
                .map_err(|_| format!("bad threshold: '{}'", thr_str))?;
            let condition = Condition { var, op: Op::Gt, threshold };

            let on_true  = Box::new(parse_node(tokens, pos)?);
            let on_false = Box::new(parse_node(tokens, pos)?);
            Ok(Node::If { condition, on_true, on_false })
        }
        "-" => {
            let name = *tokens.get(*pos)
                .ok_or("expected predictor name after '-'")?;
            *pos += 1;
            parse_predictor_tokens(name, tokens, pos)
        }
        other => Err(format!("expected 'if' or '-', got '{}'", other)),
    }
}

fn parse_predictor_tokens(name: &str, tokens: &[&str], pos: &mut usize) -> Result<Node, String> {
    if name == "Set" {
        let v_str = *tokens.get(*pos)
            .ok_or("expected value after 'Set'")?;
        *pos += 1;
        let v: i64 = v_str.parse()
            .map_err(|_| format!("bad Set value: '{}'", v_str))?;
        Ok(Node::Predict(Predictor::Set(v)))
    } else {
        // Offset formats: "0"  or  "+ n"  or  "- n"
        let sign_or_zero = *tokens.get(*pos)
            .ok_or_else(|| format!("expected offset after '{}'", name))?;
        *pos += 1;
        let offset: i64 = match sign_or_zero {
            "0" => 0,
            "+" => {
                let mag = *tokens.get(*pos)
                    .ok_or("expected magnitude after '+'")?;
                *pos += 1;
                mag.parse().map_err(|_| format!("bad magnitude: '{}'", mag))?
            }
            "-" => {
                let mag = *tokens.get(*pos)
                    .ok_or("expected magnitude after '-'")?;
                *pos += 1;
                let n: i64 = mag.parse()
                    .map_err(|_| format!("bad magnitude: '{}'", mag))?;
                -n
            }
            other => return Err(format!("expected '0', '+', or '-' for offset, got '{}'", other)),
        };
        Ok(Node::Predict(make_predictor(name, offset)?))
    }
}

fn parse_var(s: &str) -> Result<Var, String> {
    match s {
        "x"   => Ok(Var::X),
        "y"   => Ok(Var::Y),
        "c"   => Ok(Var::C),
        "W"   => Ok(Var::W),
        "N"   => Ok(Var::N),
        "WGH" => Ok(Var::WGH),
        _     => Err(format!("unknown variable: '{}'", s)),
    }
}

fn make_predictor(name: &str, offset: i64) -> Result<Predictor, String> {
    match name {
        "N"        => Ok(Predictor::N(offset)),
        "W"        => Ok(Predictor::W(offset)),
        "AvgN+NW"  => Ok(Predictor::AvgNNW(offset)),
        "AvgN+NE"  => Ok(Predictor::AvgNNE(offset)),
        "AvgW+NW"  => Ok(Predictor::AvgWNW(offset)),
        "Weighted" => Ok(Predictor::Weighted(offset)),
        _          => Err(format!("unknown predictor: '{}'", name)),
    }
}
