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

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    /// Any other jxl_from_tree-accepted leaf we don't model structurally
    /// (`NE`, `NW`, `WW`, `NN`, `NWW`, `AvgW+N`, `AvgAll`, `Gradient`,
    /// `Select`). `offset` stores the raw source offset text so
    /// `to_text` re-emits verbatim (`"0"`, `"+ 5"`, `"- 12"`, `"+137"`).
    Other { name: String, offset: String },
}

impl Predictor {
    pub fn label(&self) -> String {
        fn fmt_offset(o: i64) -> String {
            if o >= 0 { format!("+ {}", o) } else { format!("- {}", o.abs()) }
        }
        fn fmt_pred(name: &str, o: i64) -> String {
            if o == 0 { format!("{} 0", name) } else { format!("{} {}", name, fmt_offset(o)) }
        }
        match self {
            Predictor::Set(v)      => format!("Set {}", v),
            Predictor::N(o)        => fmt_pred("N", *o),
            Predictor::W(o)        => fmt_pred("W", *o),
            Predictor::AvgNNW(o)   => fmt_pred("AvgN+NW", *o),
            Predictor::AvgNNE(o)   => fmt_pred("AvgN+NE", *o),
            Predictor::AvgWNW(o)   => fmt_pred("AvgW+NW", *o),
            Predictor::Weighted(o) => fmt_pred("Weighted", *o),
            Predictor::Other { name, offset } => format!("{} {}", name, offset),
        }
    }
}

// ── Nodes ────────────────────────────────────────────────────────────────────

/// Binary decision tree matching `jxl_from_tree`'s `ParseNode` shape: every
/// `If` has exactly one `on_true` and one `on_false`, every path ends at a
/// `Predict` leaf.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Node {
    If {
        condition: Condition,
        on_true:  Box<Node>,
        on_false: Box<Node>,
    },
    Predict(Predictor),
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

/// Directives we don't model structurally but recognise so the header loop
/// doesn't mistake them for body tokens. Passed through verbatim via
/// `extra_headers`.
const EXTRA_HEADER_KEYS: &[&str] = &[
    "Squeeze", "DeltaPalette", "Gaborish", "XYB", "Alpha", "NotLast",
    "EPF", "Upsample", "HiddenChannel",
    "Rec2100", "Noise", "FramePos",
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
        let all_lines: Vec<&str> = stripped.lines().collect();
        // A leading comment like `/* title */` becomes a blank line after
        // stripping; drop those up front so the header loop doesn't treat
        // that blank line as the end of headers.
        let start_idx = all_lines.iter()
            .position(|l| !l.trim().is_empty())
            .unwrap_or(all_lines.len());
        let lines: &[&str] = &all_lines[start_idx..];

        let mut bitdepth: u32 = 8;
        let mut width:    u32 = 1024;
        let mut height:   u32 = 1024;
        let mut channels: u32 = 3;
        let mut orientation: Option<u32> = None;
        let mut rct:         Option<u32> = None;
        let mut extra_headers: Vec<String> = Vec::new();

        // Phase 1: headers. Ends at first blank line OR first unknown first-token.
        let mut body_start = lines.len();
        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                body_start = i + 1;
                break;
            }
            let mut it = trimmed.split_whitespace();
            let key = it.next().unwrap_or("");
            let rest: Vec<&str> = it.collect();
            match key {
                "Bitdepth"    => bitdepth    = parse_u32(&rest, "Bitdepth")?,
                "Width"       => width       = parse_u32(&rest, "Width")?,
                "Height"      => height      = parse_u32(&rest, "Height")?,
                "Channels"    => channels    = parse_u32(&rest, "Channels")?,
                "Orientation" => orientation = Some(parse_u32(&rest, "Orientation")?),
                "RCT"         => rct         = Some(parse_u32(&rest, "RCT")?),
                k if EXTRA_HEADER_KEYS.contains(&k) => {
                    // Canonicalise whitespace: key + args joined by single spaces.
                    let mut line = String::from(k);
                    for a in &rest {
                        line.push(' ');
                        line.push_str(a);
                    }
                    extra_headers.push(line);
                }
                _ => {
                    // Unknown first token → treat as start of body.
                    body_start = i;
                    break;
                }
            }
        }

        // Phase 2: extract Spline…EndSpline blocks; the rest is tree body.
        let mut splines: Vec<String> = Vec::new();
        let mut body_lines: Vec<&str> = Vec::new();
        let mut j = body_start;
        while j < lines.len() {
            let first_tok = lines[j].split_whitespace().next().unwrap_or("");
            if first_tok == "Spline" {
                let mut block: Vec<&str> = vec![lines[j]];
                j += 1;
                while j < lines.len() {
                    let cur = lines[j];
                    block.push(cur);
                    let has_end = cur.split_whitespace().any(|t| t == "EndSpline");
                    j += 1;
                    if has_end {
                        break;
                    }
                }
                splines.push(block.join("\n"));
            } else {
                body_lines.push(lines[j]);
                j += 1;
            }
        }

        // Phase 3: tokenise body and walk the tree.
        let tokens: Vec<&str> = body_lines.iter()
            .flat_map(|l| l.split_whitespace())
            .collect();

        if tokens.is_empty() {
            return Err("program has no tree body".to_string());
        }

        let mut pos = 0usize;
        let root = parse_node(&tokens, &mut pos)?;

        Ok(ImageProgram {
            width, height, bitdepth, channels, orientation, rct,
            extra_headers, splines, root,
        })
    }
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
                    Some(end) => { rest = &after[end + 2..]; }
                    None => {
                        out.push_str("/*");
                        out.push_str(after);
                        break;
                    }
                }
            }
            None => { out.push_str(rest); break; }
        }
    }
    out
}

fn parse_u32(rest: &[&str], key: &str) -> Result<u32, String> {
    let v = rest.first().ok_or_else(|| format!("expected value after '{}'", key))?;
    v.parse().map_err(|_| format!("bad {}: {}", key, v))
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
            let var = parse_var(var_str);
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
        return Ok(Node::Predict(Predictor::Set(v)));
    }

    // Offset: "0" | "+ N" | "- N" | signed-int-literal ("+137", "-195", "42")
    let sign_or_zero = *tokens.get(*pos)
        .ok_or_else(|| format!("expected offset after '{}'", name))?;

    let (offset_val, offset_raw): (i64, String) = match sign_or_zero {
        "0" => { *pos += 1; (0, "0".to_string()) }
        "+" => {
            *pos += 1;
            let mag = *tokens.get(*pos).ok_or("expected magnitude after '+'")?;
            *pos += 1;
            let n: i64 = mag.parse().map_err(|_| format!("bad magnitude: '{}'", mag))?;
            (n, format!("+ {}", mag))
        }
        "-" => {
            *pos += 1;
            let mag = *tokens.get(*pos).ok_or("expected magnitude after '-'")?;
            *pos += 1;
            let n: i64 = mag.parse().map_err(|_| format!("bad magnitude: '{}'", mag))?;
            (-n, format!("- {}", mag))
        }
        other if is_signed_int(other) => {
            *pos += 1;
            let n: i64 = other.parse().map_err(|_| format!("bad offset: '{}'", other))?;
            (n, other.to_string())
        }
        other => return Err(format!("expected '0', '+', '-', or signed int for offset, got '{}'", other)),
    };

    let pred = match name {
        "N"        => Predictor::N(offset_val),
        "W"        => Predictor::W(offset_val),
        "AvgN+NW"  => Predictor::AvgNNW(offset_val),
        "AvgN+NE"  => Predictor::AvgNNE(offset_val),
        "AvgW+NW"  => Predictor::AvgWNW(offset_val),
        "Weighted" => Predictor::Weighted(offset_val),
        _          => Predictor::Other { name: name.to_string(), offset: offset_raw },
    };
    Ok(Node::Predict(pred))
}

fn parse_var(s: &str) -> Var {
    match s {
        "x"   => Var::X,
        "y"   => Var::Y,
        "c"   => Var::C,
        "W"   => Var::W,
        "N"   => Var::N,
        "WGH" => Var::WGH,
        _     => Var::Other(s.to_string()),
    }
}

fn is_signed_int(s: &str) -> bool {
    if s.is_empty() { return false; }
    let rest = if s.starts_with('+') || s.starts_with('-') { &s[1..] } else { s };
    !rest.is_empty() && rest.bytes().all(|b| b.is_ascii_digit())
}
