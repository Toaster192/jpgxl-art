use serde::{Deserialize, Serialize};

// ── Variables usable in conditions ─────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[allow(clippy::upper_case_acronyms)] // names mirror jxl_from_tree's identifiers
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
    /// Any other jxl_from_tree-accepted variable we don't model structurally
    /// (e.g. `NE`, `W+N-NW`, `W-WW-NW+NWW`, `Prev5`, `|W|`). Preserved
    /// verbatim so `to_text` round-trips through `jxl_from_tree`.
    Other(String),
}

impl Var {
    pub fn label(&self) -> &str {
        match self {
            Var::X => "x",
            Var::Y => "y",
            Var::C => "c",
            Var::W => "W",
            Var::N => "N",
            Var::WGH => "WGH",
            Var::Other(s) => s.as_str(),
        }
    }
}

// ── Operators ───────────────────────────────────────────────────────────────

/// `jxl_from_tree` only accepts `>` comparisons, so that's all we model.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Op {
    Gt,
}

// ── Condition ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Condition {
    pub var: Var,
    pub op: Op,
    pub threshold: i64,
}

impl Condition {
    pub fn label(&self) -> String {
        format!("{} > {}", self.var.label(), self.threshold)
    }
}

// ── Predictors ──────────────────────────────────────────────────────────────

/// Leaf predictor. Rendering happens via libjxl (see `crate::render`); this
/// type only exists so the mutation engine has something to inspect and
/// rewrite.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// Any other jxl_from_tree-accepted leaf we don't model structurally
    /// (`NE`, `NW`, `WW`, `AvgW+N`, `AvgAll`, `Gradient`, `Select`).
    /// `NN` and `NWW` are valid inside *condition* composites but crash
    /// as standalone leaf predictors, so they're excluded here.
    /// `offset` stores the raw source offset text so `to_text` re-emits
    /// verbatim (`"0"`, `"+ 5"`, `"- 12"`, `"+137"`).
    Other { name: String, offset: String },
}

impl Predictor {
    pub fn label(&self) -> String {
        fn fmt_offset(o: i64) -> String {
            if o >= 0 {
                format!("+ {}", o)
            } else {
                format!("- {}", o.abs())
            }
        }
        fn fmt_pred(name: &str, o: i64) -> String {
            if o == 0 {
                format!("{} 0", name)
            } else {
                format!("{} {}", name, fmt_offset(o))
            }
        }
        match self {
            Predictor::Set(v) => format!("Set {}", v),
            Predictor::N(o) => fmt_pred("N", *o),
            Predictor::W(o) => fmt_pred("W", *o),
            Predictor::AvgNNW(o) => fmt_pred("AvgN+NW", *o),
            Predictor::AvgNNE(o) => fmt_pred("AvgN+NE", *o),
            Predictor::AvgWNW(o) => fmt_pred("AvgW+NW", *o),
            Predictor::Weighted(o) => fmt_pred("Weighted", *o),
            Predictor::Other { name, offset } => format!("{} {}", name, offset),
        }
    }
}

// ── Nodes ────────────────────────────────────────────────────────────────────

/// Binary decision tree matching `jxl_from_tree`'s `ParseNode` shape: every
/// `If` has exactly one `on_true` and one `on_false`, every path ends at a
/// `Predict` leaf.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Node {
    If {
        condition: Condition,
        on_true: Box<Node>,
        on_false: Box<Node>,
    },
    Predict(Predictor),
}

// ── Image program ─────────────────────────────────────────────────────────────

/// One extra frame in a multi-frame (`NotLast`) program. The first frame lives
/// directly on `ImageProgram` (its global header + `root`); every subsequent
/// frame is a `Frame` carrying its own verbatim header directives and tree.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    /// This frame's header directive lines, verbatim and in source order
    /// (e.g. `"RCT 0"`, `"FramePos -662 -100"`). Later frames' directives are
    /// passed through unmodelled — we only structure frame 0's global header.
    #[serde(default)]
    pub extra_headers: Vec<String>,
    /// Verbatim `Spline … EndSpline` blocks for this frame.
    #[serde(default)]
    pub splines: Vec<String>,
    pub root: Node,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImageProgram {
    pub width: u32,
    pub height: u32,
    pub bitdepth: u32,
    pub channels: u32,
    pub orientation: Option<u32>,
    pub rct: Option<u32>,
    /// Header directives we don't model structurally, as raw lines in source
    /// order. Examples: `"DeltaPalette"`, `"Alpha"`, `"HiddenChannel 15"`,
    /// `"Noise 0 0 0 0 0 0 0 0"`, `"Rec2100 PQ"`.
    #[serde(default)]
    pub extra_headers: Vec<String>,
    /// Verbatim `Spline … EndSpline` blocks; emitted between the header
    /// and the body.
    #[serde(default)]
    pub splines: Vec<String>,
    pub root: Node,
    /// Frames after the first, for multi-frame (`NotLast`) programs. Empty for
    /// the common single-frame case.
    #[serde(default)]
    pub extra_frames: Vec<Frame>,
}

impl ImageProgram {
    /// The original jxl-art default program.
    pub fn example_jxlart() -> Self {
        Self::from_text(include_str!("../gallery/00-sky-and-grass.jxlart"))
            .expect("example_jxlart is always valid")
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
        if self.channels != 3 {
            out.push_str(&format!("Channels {}\n", self.channels));
        }
        // Width/Height default to 1024×1024 in jxl-art; only emit if different.
        if self.width != 1024 || self.height != 1024 {
            out.push_str(&format!("Width {}\nHeight {}\n", self.width, self.height));
        }
        for h in &self.extra_headers {
            out.push_str(h);
            out.push('\n');
        }
        out.push('\n');
        for s in &self.splines {
            out.push_str(s);
            if !s.ends_with('\n') {
                out.push('\n');
            }
            out.push('\n');
        }
        write_node(&mut out, &self.root, 0);
        // Subsequent frames: their own verbatim header block, then their tree.
        for frame in &self.extra_frames {
            out.push('\n');
            for h in &frame.extra_headers {
                out.push_str(h);
                out.push('\n');
            }
            out.push('\n');
            for s in &frame.splines {
                out.push_str(s);
                if !s.ends_with('\n') {
                    out.push('\n');
                }
                out.push('\n');
            }
            write_node(&mut out, &frame.root, 0);
        }
        out
    }
}

fn write_node(out: &mut String, node: &Node, depth: usize) {
    let indent = "  ".repeat(depth);
    match node {
        Node::If {
            condition,
            on_true,
            on_false,
        } => {
            out.push_str(&format!("{}if {}\n", indent, condition.label()));
            write_node(out, on_true, depth + 1);
            write_node(out, on_false, depth + 1);
        }
        Node::Predict(pred) => {
            out.push_str(&format!("{}- {}\n", indent, pred.label()));
        }
    }
}

// ── Parser ────────────────────────────────────────────────────────────────────

/// Directives we don't model structurally but recognise so the header loop
/// doesn't mistake them for body tokens. Passed through verbatim via
/// `extra_headers`.
const EXTRA_HEADER_KEYS: &[&str] = &[
    "Squeeze",
    "DeltaPalette",
    "Gaborish",
    "XYB",
    "XYBFactors",
    "Alpha",
    "NotLast",
    "EPF",
    "Upsample",
    "HiddenChannel",
    "Rec2100",
    "Noise",
    "FramePos",
    "SplineQuantizationAdjustment",
    "CbYCr",
    "PQ",
    "GroupShift",
    "FloatExpBits",
];

impl ImageProgram {
    /// Parse a jxl-art text program.
    ///
    /// Accepts anything `jxl_from_tree` does: unknown header directives are
    /// preserved via `extra_headers`, `Spline … EndSpline` blocks go to
    /// `splines`, and unknown condition variables / predictor names are
    /// wrapped in `Var::Other` / `Predictor::Other` so they round-trip
    /// through `to_text` unchanged.
    pub fn from_text(s: &str) -> Result<Self, String> {
        let stripped = strip_block_comments(s);
        let lines: Vec<&str> = stripped.lines().collect();

        let mut bitdepth: u32 = 8;
        let mut width: u32 = 1024;
        let mut height: u32 = 1024;
        let mut channels: u32 = 3;
        let mut orientation: Option<u32> = None;
        let mut rct: Option<u32> = None;
        let mut extra_headers: Vec<String> = Vec::new();
        let mut splines: Vec<String> = Vec::new();

        // Frame 0 header phase: walk all lines until the first body-starter
        // (`if` or `-`). Blank lines and `Spline … EndSpline` blocks can be
        // interleaved with header directives, so we don't stop at the first
        // blank. `i` is left pointing at the first body line.
        let mut i = 0;
        loop {
            if i >= lines.len() {
                break;
            }
            let trimmed = lines[i].trim();
            if trimmed.is_empty() {
                i += 1;
                continue;
            }
            let first_tok = trimmed.split_whitespace().next().unwrap_or("");
            if first_tok == "if" || first_tok == "-" {
                break;
            }
            if first_tok == "Spline" {
                let mut block: Vec<&str> = vec![lines[i]];
                i += 1;
                while i < lines.len() {
                    let cur = lines[i];
                    block.push(cur);
                    let has_end = cur.split_whitespace().any(|t| t == "EndSpline");
                    i += 1;
                    if has_end {
                        break;
                    }
                }
                splines.push(block.join("\n"));
                continue;
            }
            let mut it = trimmed.split_whitespace();
            let key = it.next().unwrap_or("");
            let rest: Vec<&str> = it.collect();
            match key {
                "Bitdepth" => bitdepth = parse_u32(&rest, "Bitdepth")?,
                "Width" => width = parse_u32(&rest, "Width")?,
                "Height" => height = parse_u32(&rest, "Height")?,
                "Channels" => channels = parse_u32(&rest, "Channels")?,
                "Orientation" => orientation = Some(parse_u32(&rest, "Orientation")?),
                "RCT" => rct = Some(parse_u32(&rest, "RCT")?),
                k if EXTRA_HEADER_KEYS.contains(&k) => {
                    let mut line = String::from(k);
                    for a in &rest {
                        line.push(' ');
                        line.push_str(a);
                    }
                    extra_headers.push(line);
                }
                other => return Err(format!("unknown directive '{}' in header", other)),
            }
            i += 1;
        };

        // Frame 0 body: parse exactly one tree, advancing `i` past it.
        let root = parse_tree_advancing(&lines, &mut i)?;

        // Additional frames are governed by `NotLast`, exactly like
        // jxl_from_tree: a frame is followed by another iff its header carries a
        // `NotLast` directive. Anything after the final (non-`NotLast`) frame is
        // ignored — the default program, for instance, has a dead trailing
        // `if W > 73` fragment that jxl_from_tree silently drops.
        let mut extra_frames: Vec<Frame> = Vec::new();
        let mut prev_not_last = header_has_notlast(&extra_headers);
        while prev_not_last {
            // Skip blanks between frames.
            while i < lines.len() && lines[i].trim().is_empty() {
                i += 1;
            }
            if i >= lines.len() {
                // `NotLast` promised a following frame that isn't there.
                return Err("NotLast frame is missing its following frame".to_string());
            }
            let (fheaders, fsplines) = parse_frame_header(&lines, &mut i)?;
            if i >= lines.len() {
                return Err("frame header with no tree body".to_string());
            }
            let froot = parse_tree_advancing(&lines, &mut i)?;
            prev_not_last = header_has_notlast(&fheaders);
            extra_frames.push(Frame {
                extra_headers: fheaders,
                splines: fsplines,
                root: froot,
            });
        }

        Ok(ImageProgram {
            width,
            height,
            bitdepth,
            channels,
            orientation,
            rct,
            extra_headers,
            splines,
            root,
            extra_frames,
        })
    }
}

/// Tokenise the body from line `*i` onward, parse exactly one complete tree, and
/// advance `*i` to the line of the next unconsumed token (or past the end). A
/// strict binary tree consumes exactly its own tokens, so for a well-formed
/// multi-frame program this stops precisely at the next frame's first line.
fn parse_tree_advancing(lines: &[&str], i: &mut usize) -> Result<Node, String> {
    let mut tokens: Vec<&str> = Vec::new();
    let mut token_line: Vec<usize> = Vec::new();
    for (off, line) in lines[*i..].iter().enumerate() {
        for tok in line.split_whitespace() {
            tokens.push(tok);
            token_line.push(*i + off);
        }
    }
    if tokens.is_empty() {
        return Err("program has no tree body".to_string());
    }
    let mut pos = 0usize;
    let root = parse_node(&tokens, &mut pos)?;
    *i = if pos < tokens.len() {
        token_line[pos]
    } else {
        lines.len()
    };
    Ok(root)
}

/// Collect one (non-first) frame's header block — verbatim directive lines and
/// `Spline` blocks — starting at line `*i`, stopping at the first body-starter
/// (`if`/`-`) line or end of input. Unlike frame 0, later frames keep every
/// directive (including `RCT`/`Width` overrides) verbatim; we only require they
/// be recognised so genuinely malformed input still errors.
fn parse_frame_header(lines: &[&str], i: &mut usize) -> Result<(Vec<String>, Vec<String>), String> {
    let mut extra_headers: Vec<String> = Vec::new();
    let mut splines: Vec<String> = Vec::new();
    while *i < lines.len() {
        let trimmed = lines[*i].trim();
        if trimmed.is_empty() {
            *i += 1;
            continue;
        }
        let key = trimmed.split_whitespace().next().unwrap_or("");
        if key == "if" || key == "-" {
            break;
        }
        if key == "Spline" {
            let mut block: Vec<&str> = vec![lines[*i]];
            *i += 1;
            while *i < lines.len() {
                let cur = lines[*i];
                block.push(cur);
                let has_end = cur.split_whitespace().any(|t| t == "EndSpline");
                *i += 1;
                if has_end {
                    break;
                }
            }
            splines.push(block.join("\n"));
            continue;
        }
        if !is_recognised_header_key(key) {
            return Err(format!("unknown directive '{}' in frame header", key));
        }
        extra_headers.push(trimmed.split_whitespace().collect::<Vec<_>>().join(" "));
        *i += 1;
    }
    Ok((extra_headers, splines))
}

/// Whether a frame's header directive lines include a `NotLast` (meaning another
/// frame follows it).
fn header_has_notlast(headers: &[String]) -> bool {
    headers
        .iter()
        .any(|h| h.split_whitespace().next() == Some("NotLast"))
}

/// Header keys jxl-art recognises: the structured globals plus the pass-through
/// `EXTRA_HEADER_KEYS`.
fn is_recognised_header_key(k: &str) -> bool {
    matches!(
        k,
        "Bitdepth" | "Width" | "Height" | "Channels" | "Orientation" | "RCT"
    ) || EXTRA_HEADER_KEYS.contains(&k)
}

/// Remove every `/* … */` block. Unterminated comments are kept verbatim so
/// bad input doesn't silently swallow code.
fn strip_block_comments(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    loop {
        match rest.find("/*") {
            Some(start) => {
                out.push_str(&rest[..start]);
                out.push(' '); // keep token boundary
                let after = &rest[start + 2..];
                match after.find("*/") {
                    Some(end) => {
                        rest = &after[end + 2..];
                    }
                    None => {
                        out.push_str("/*");
                        out.push_str(after);
                        break;
                    }
                }
            }
            None => {
                out.push_str(rest);
                break;
            }
        }
    }
    out
}

fn parse_u32(rest: &[&str], key: &str) -> Result<u32, String> {
    let v = rest
        .first()
        .ok_or_else(|| format!("expected value after '{}'", key))?;
    v.parse().map_err(|_| format!("bad {}: {}", key, v))
}

/// Recursively parse one node (if-branch or predict leaf) from the token stream.
fn parse_node(tokens: &[&str], pos: &mut usize) -> Result<Node, String> {
    let tok = *tokens
        .get(*pos)
        .ok_or_else(|| "unexpected end of input while parsing node".to_string())?;
    *pos += 1;

    match tok {
        "if" => {
            let var_str = *tokens
                .get(*pos)
                .ok_or("expected variable name after 'if'")?;
            *pos += 1;
            let op_str = *tokens.get(*pos).ok_or("expected operator after variable")?;
            *pos += 1;
            let thr_str = *tokens.get(*pos).ok_or("expected threshold value")?;
            *pos += 1;

            if op_str != ">" {
                return Err(format!("only '>' operator is supported, got '{}'", op_str));
            }
            let var = parse_var(var_str);
            let threshold: i64 = thr_str
                .parse()
                .map_err(|_| format!("bad threshold: '{}'", thr_str))?;
            let condition = Condition {
                var,
                op: Op::Gt,
                threshold,
            };

            let on_true = Box::new(parse_node(tokens, pos)?);
            let on_false = Box::new(parse_node(tokens, pos)?);
            Ok(Node::If {
                condition,
                on_true,
                on_false,
            })
        }
        "-" => {
            let name = *tokens
                .get(*pos)
                .ok_or("expected predictor name after '-'")?;
            *pos += 1;
            parse_predictor_tokens(name, tokens, pos)
        }
        other => Err(format!("expected 'if' or '-', got '{}'", other)),
    }
}

fn parse_predictor_tokens(name: &str, tokens: &[&str], pos: &mut usize) -> Result<Node, String> {
    if name == "Set" {
        let first = *tokens.get(*pos).ok_or("expected value after 'Set'")?;
        *pos += 1;
        let v: i64 = if first == "-" || first == "+" {
            let mag_str = *tokens
                .get(*pos)
                .ok_or_else(|| format!("expected magnitude after Set sign '{}'", first))?;
            *pos += 1;
            let mag: i64 = mag_str
                .parse()
                .map_err(|_| format!("bad Set magnitude: '{}'", mag_str))?;
            if first == "-" {
                -mag
            } else {
                mag
            }
        } else {
            first
                .parse()
                .map_err(|_| format!("bad Set value: '{}'", first))?
        };
        return Ok(Node::Predict(Predictor::Set(v)));
    }

    // Offset: "0" | "+ N" | "- N" | signed-int-literal ("+137", "-195", "42")
    let sign_or_zero = *tokens
        .get(*pos)
        .ok_or_else(|| format!("expected offset after '{}'", name))?;

    let (offset_val, offset_raw): (i64, String) = match sign_or_zero {
        "0" => {
            *pos += 1;
            (0, "0".to_string())
        }
        "+" => {
            *pos += 1;
            let mag = *tokens.get(*pos).ok_or("expected magnitude after '+'")?;
            *pos += 1;
            let n: i64 = mag
                .parse()
                .map_err(|_| format!("bad magnitude: '{}'", mag))?;
            (n, format!("+ {}", mag))
        }
        "-" => {
            *pos += 1;
            let mag = *tokens.get(*pos).ok_or("expected magnitude after '-'")?;
            *pos += 1;
            let n: i64 = mag
                .parse()
                .map_err(|_| format!("bad magnitude: '{}'", mag))?;
            (-n, format!("- {}", mag))
        }
        other if is_signed_int(other) => {
            *pos += 1;
            let n: i64 = other
                .parse()
                .map_err(|_| format!("bad offset: '{}'", other))?;
            (n, other.to_string())
        }
        other => {
            return Err(format!(
                "expected '0', '+', '-', or signed int for offset, got '{}'",
                other
            ))
        }
    };

    let pred = match name {
        "N" => Predictor::N(offset_val),
        "W" => Predictor::W(offset_val),
        "AvgN+NW" => Predictor::AvgNNW(offset_val),
        "AvgN+NE" => Predictor::AvgNNE(offset_val),
        "AvgW+NW" => Predictor::AvgWNW(offset_val),
        "Weighted" => Predictor::Weighted(offset_val),
        _ => Predictor::Other {
            name: name.to_string(),
            offset: offset_raw,
        },
    };
    Ok(Node::Predict(pred))
}

fn parse_var(s: &str) -> Var {
    match s {
        "x" => Var::X,
        "y" => Var::Y,
        "c" => Var::C,
        "W" => Var::W,
        "N" => Var::N,
        "WGH" => Var::WGH,
        _ => Var::Other(s.to_string()),
    }
}

fn is_signed_int(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let rest = if s.starts_with('+') || s.starts_with('-') {
        &s[1..]
    } else {
        s
    };
    !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit())
}
