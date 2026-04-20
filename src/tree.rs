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
    /// The body is tokenised with `split_whitespace` (identical to
    /// `jxl_from_tree`'s `*f >> out` tokeniser), so indentation is entirely
    /// cosmetic and ignored.
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
