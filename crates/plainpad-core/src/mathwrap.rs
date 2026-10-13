//! Math layer: fend-backed evaluation plus deterministic pre-processing for the
//! scratchpad cases fend does not handle natively (labelled sums, percentages,
//! base conversion). Network is never touched: currency conversion is out of
//! scope here (PRD M-5 is P1 and requires an explicit opt-in shell hook).

use fend_core::Context;
use std::collections::BTreeMap;

/// Configurable angle measurement mode for trigonometric calculations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum AngleMode {
    #[serde(rename = "degrees")]
    Degrees,
    #[serde(rename = "radians")]
    Radians,
}

impl Default for AngleMode {
    fn default() -> Self {
        AngleMode::Degrees
    }
}

impl std::str::FromStr for AngleMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_lowercase().as_str() {
            "degrees" | "deg" => Ok(AngleMode::Degrees),
            "radians" | "rad" => Ok(AngleMode::Radians),
            other => Err(format!("Unknown angle mode '{other}'; expected 'degrees' or 'radians'")),
        }
    }
}

/// Outcome of evaluating one expression.
#[derive(Debug, Clone, PartialEq)]
pub struct MathOutcome {
    /// The computed result, formatted for display.
    pub result: String,
    /// True when fend (or the built-in paths) resolved this as a quantity/unit.
    pub unit_aware: bool,
}

/// Evaluate an expression with default Degrees angle mode.
pub fn evaluate(expr: &str) -> Result<MathOutcome, String> {
    evaluate_with_mode(expr, AngleMode::Degrees)
}

/// Evaluate an expression with a specific angle mode (Degrees vs Radians).
/// Deterministic: RNG disabled, fixed "current time", terminal output mode, no network handlers.
/// In-memory cache keeps per-keystroke re-analysis well under the 30 ms budget.
pub fn evaluate_with_mode(expr: &str, mode: AngleMode) -> Result<MathOutcome, String> {
    use std::sync::{Mutex, OnceLock};
    type EvalCache = Mutex<Vec<(String, Result<MathOutcome, String>)>>;
    static CACHE: OnceLock<EvalCache> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(Vec::with_capacity(64)));
    let mode_prefix = match mode {
        AngleMode::Degrees => "deg",
        AngleMode::Radians => "rad",
    };
    let trimmed = expr.trim();
    let key = format!("{}:{}", mode_prefix, trimmed);
    if let Some(hit) = cache.lock().unwrap().iter().find(|(k, _)| *k == key) {
        return hit.1.clone();
    }
    let outcome = evaluate_uncached_with_mode(trimmed, mode);
    {
        let mut c = cache.lock().unwrap();
        if c.len() >= 64 {
            let _ = c.remove(0);
        }
        c.push((key, outcome.clone()));
    }
    outcome
}

fn evaluate_uncached_with_mode(expr: &str, mode: AngleMode) -> Result<MathOutcome, String> {
    let prepared = preprocess_math_grammar(expr);
    let prepared = match mode {
        AngleMode::Degrees => apply_degrees_mode(&prepared),
        AngleMode::Radians => prepared,
    };
    let mut ctx = Context::new();
    ctx.disable_rng();
    ctx.set_current_time_v1(0, 0);
    ctx.set_output_mode_terminal();
    let res = fend_core::evaluate(&prepared, &mut ctx)?;
    Ok(MathOutcome { result: res.get_main_result().to_string(), unit_aware: !res.output_is_empty() })
}

/// Named-variable table extracted from a note (see `extract_variables`).
pub type Vars = BTreeMap<String, String>;

/// Extract variable bindings from note lines of the shapes:
///   `rent = 12000`          (plain assignment)
///   `rent: 12000`           (label: value — also a binding)
///   `rent 12000`            (label space value, label ≤ 3 words, no trailing op)
/// The stored value is the raw right-hand side; it may itself reference other
/// variables and is resolved lazily during `evaluate_with_vars`.
pub fn extract_variables(lines: &[&str]) -> Vars {
    let mut vars = Vars::new();
    for line in lines {
        let t = line.trim();
        if t.is_empty() || is_checklist_line(t) {
            continue;
        }
        if let Some((name, rhs)) = split_assignment(t, '=') {
            if valid_name(&name) {
                vars.insert(name.to_lowercase(), rhs.trim().to_string());
            }
        } else if let Some((name, rhs)) = split_assignment(t, ':') {
            if valid_name(&name) && looks_like_value(rhs) {
                vars.insert(name.to_lowercase(), rhs.trim().to_string());
            }
        } else if let Some((name, rhs)) = split_label_value(t) {
            if valid_name(&name) && looks_like_value(rhs) {
                vars.insert(name.to_lowercase(), rhs.trim().to_string());
            }
        }
    }
    vars
}

fn is_checklist_line(t: &str) -> bool {
    t.starts_with("[ ] ") || t.starts_with("[x] ") || t.starts_with("[X] ")
}

fn split_assignment(t: &str, sep: char) -> Option<(String, &str)> {
    let pos = t.find(sep)?;
    let name = t[..pos].trim();
    let rhs = &t[pos + sep.len_utf8()..];
    if name.is_empty() || rhs.trim().is_empty() {
        return None;
    }
    Some((name.to_string(), rhs))
}

/// `label value` split: last whitespace run before a trailing pure number/expression.
fn split_label_value(t: &str) -> Option<(String, &str)> {
    let idx = t.rfind(char::is_whitespace)?;
    let (name, rhs) = (t[..idx].trim(), t[idx..].trim());
    if name.is_empty() || rhs.is_empty() {
        return None;
    }
    // The right side must start like a number/quantity for this to be a binding.
    let first = rhs.chars().next()?;
    if first.is_ascii_digit() || first == '-' || first == '+' || first == '(' {
        Some((name.to_string(), rhs))
    } else {
        None
    }
}

fn valid_name(name: &str) -> bool {
    let n = name.trim();
    if n.is_empty() || n.chars().count() > 24 {
        return false;
    }
    // Names may contain letters, digits (not leading), spaces (≤ 3 words) and _-.
    let words = n.split_whitespace().count();
    if words > 3 {
        return false;
    }
    n.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-' || c.is_whitespace())
        && n.chars().next().is_some_and(|c| c.is_alphabetic() || c == '_')
}

fn looks_like_value(rhs: &str) -> bool {
    let t = rhs.trim().trim_end_matches(['.', ';']);
    if t.is_empty() {
        return false;
    }
    let first = t.chars().next().unwrap();
    first.is_ascii_digit() || first == '-' || first == '+' || first == '('
}

/// Substitute variables into an expression, longest-name-first, whole-word only.
/// Repeats (up to 5 passes) let variables reference other variables.
pub fn substitute_vars(expr: &str, vars: &Vars) -> String {
    let mut out = expr.to_string();
    for _ in 0..5 {
        let before = out.clone();
        let mut sorted: Vec<(&String, &String)> = vars.iter().collect();
        sorted.sort_by_key(|(k, _)| std::cmp::Reverse(k.chars().count()));
        for (name, value) in sorted {
            out = replace_word(&out, name, value);
        }
        if out == before {
            break;
        }
    }
    out
}

fn replace_word(hay: &str, word: &str, replacement: &str) -> String {
    if word.is_empty() {
        return hay.to_string();
    }
    let mut result = String::with_capacity(hay.len());
    let mut rest = hay;
    let lower_hay = hay.to_lowercase();
    let lower_word = word.to_lowercase();
    let mut search_from = 0usize;
    while let Some(rel) = lower_hay[search_from..].find(&lower_word) {
        let start = search_from + rel;
        let end = start + lower_word.len();
        let boundary_before =
            hay[..start].chars().next_back().is_none_or(|c| !(c.is_alphanumeric() || c == '_'));
        let boundary_after =
            hay[end..].chars().next().is_none_or(|c| !(c.is_alphanumeric() || c == '_'));
        if boundary_before && boundary_after {
            result.push_str(&hay[search_from..start]);
            result.push_str(replacement);
        } else {
            result.push_str(&hay[search_from..start]);
            result.push_str(&hay[start..end]);
        }
        search_from = end;
        rest = &hay[search_from.min(hay.len())..];
    }
    result.push_str(rest);
    result
}

/// Rewrite scratchpad percentages into fend-friendly arithmetic:
///   `20% of 850` → `20/100 * 850`
///   `850 + 15%`  → `850 * (1 + 15/100)`  (also −)
pub fn rewrite_percent(expr: &str) -> String {
    let t = expr.trim();
    let lower = t.to_lowercase();
    if let Some(pos) = lower.find("% of ") {
        let pct = t[..pos].trim();
        let rest = t[pos + 5..].trim();
        if pct.ends_with('%') {
            let pct = pct.trim_end_matches('%').trim();
            if is_numberish(pct) {
                return format!("({pct}/100) * ({rest})");
            }
        }
    }
    // `A + B%` / `A - B%`
    for op in ['+', '-'] {
        if let Some(pos) = t.rfind(op) {
            let (lhs, rhs) = (t[..pos].trim(), t[pos + 1..].trim());
            if let Some(pct) = rhs.strip_suffix('%') {
                let pct = pct.trim();
                if is_numberish(pct) && !lhs.is_empty() && is_numberish_or_expr(lhs) {
                    return match op {
                        '+' => format!("({lhs}) * (1 + {pct}/100)"),
                        _ => format!("({lhs}) * (1 - {pct}/100)"),
                    };
                }
            }
        }
    }
    t.to_string()
}

fn is_numberish(s: &str) -> bool {
    !s.is_empty() && s.chars().next().is_some_and(|c| c.is_ascii_digit() || c == '-' || c == '+')
}

fn is_numberish_or_expr(s: &str) -> bool {
    is_numberish(s) || s.contains(char::is_whitespace) || s.contains(['+', '-', '*', '/', '(', ')'])
}

/// Programmer base conversion: `255 in binary` / `0xFF in decimal` / `12 in hex`.
/// Returns the formatted answer, or None if the line is not a base conversion.
pub fn base_conversion(expr: &str) -> Option<String> {
    let lower = expr.trim().to_lowercase();
    let (num_part, base_name) = lower.rsplit_once(" in ")?;
    let base = match base_name.trim() {
        "binary" | "bin" | "base 2" => 2,
        "octal" | "oct" | "base 8" => 8,
        "hex" | "hexadecimal" | "base 16" => 16,
        "decimal" | "dec" | "base 10" => 10,
        _ => return None,
    };
    let value = if let Some(h) = num_part.trim().strip_prefix("0x") {
        i128::from_str_radix(h, 16).ok()?
    } else if let Some(o) = num_part.trim().strip_prefix("0o") {
        i128::from_str_radix(o, 8).ok()?
    } else if let Some(b) = num_part.trim().strip_prefix("0b") {
        i128::from_str_radix(b, 2).ok()?
    } else {
        num_part.trim().parse::<i128>().ok()?
    };
    let formatted = match base {
        2 => format!("0b{value:b}"),
        8 => format!("0o{value:o}"),
        16 => format!("0x{value:X}"),
        _ => format!("{value}"),
    };
    Some(formatted)
}

/// Block statistics over a set of lines (PRD M-8).
/// Lines starting with `//` are excluded from counts by design.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct BlockStats {
    pub sum: Option<f64>,
    pub average: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub items: usize,
    pub lines: usize,
    pub words: usize,
    pub chars: usize,
}

pub fn block_stats(lines: &[&str]) -> BlockStats {
    let mut nums: Vec<f64> = Vec::new();
    let mut counted_lines = 0usize;
    let mut words = 0usize;
    let mut chars = 0usize;
    for l in lines {
        let t = l.trim();
        chars += l.chars().count();
        if t.starts_with("//") {
            continue; // excluded from all counts by the // marker
        }
        counted_lines += 1;
        words += t.split_whitespace().count();
        if !t.is_empty() {
            nums.extend(parse_leading_number(t));
        }
    }
    let sum = if nums.is_empty() { None } else { Some(nums.iter().sum::<f64>()) };
    BlockStats {
        sum,
        average: sum.map(|s| s / nums.len() as f64),
        min: nums.iter().copied().reduce(f64::min),
        max: nums.iter().copied().reduce(f64::max),
        items: nums.len(),
        lines: counted_lines,
        words,
        chars,
    }
}

/// Parse a leading number like `4500`, `3.5`, `-12`, `1_000` from a line
/// (a label may precede it, e.g. `food 4500`).
pub fn parse_leading_number(t: &str) -> Vec<f64> {
    let mut cleaned = String::with_capacity(t.len());
    let mut started = false;
    for c in t.chars() {
        if c.is_ascii_digit() || (!started && (c == '-' || c == '+')) {
            started = true;
            cleaned.push(c);
        } else if c == ',' || c == '_' {
            // thousands separator — skip
        } else if c == '.' && started {
            cleaned.push(c);
        } else if started {
            break;
        } else if c.is_whitespace() {
            continue;
        } else {
            break; // label prefix that isn't followed by a number
        }
    }
    let mut v = Vec::new();
    if !cleaned.is_empty() {
        if let Ok(n) = cleaned.parse::<f64>() {
            v.push(n);
        }
    }
    v
}

/// Try the full evaluation stack for a note line with default Degrees mode.
pub fn eval_line(expr: &str, vars: &Vars) -> Option<MathOutcome> {
    eval_line_with_mode(expr, vars, AngleMode::Degrees)
}

/// Try the full evaluation stack for a note line with a specific angle mode:
/// base conversion → percent rewrite + variable substitution → grammar preprocessing → fend. Returns None when nothing parses.
pub fn eval_line_with_mode(expr: &str, vars: &Vars, mode: AngleMode) -> Option<MathOutcome> {
    let raw = expr.trim().trim_end_matches(['=', '?']).trim();
    if raw.is_empty() {
        return None;
    }
    let t = if let Some((_, rhs)) = raw.split_once(':') {
        let r = rhs.trim();
        if r.chars().any(|c| c.is_ascii_digit()) { r } else { raw }
    } else {
        raw
    };
    if let Some(b) = base_conversion(t) {
        return Some(MathOutcome { result: b, unit_aware: false });
    }
    let prepared = rewrite_percent(&substitute_vars(t, vars));
    // Antinote-style `x` multiplication (`trees x o2 per tree`).
    let prepared = if prepared.contains(" x ") {
        prepared.replace(" x ", " * ")
    } else {
        prepared
    };
    let prepared = preprocess_math_grammar(&prepared);
    if is_bitwise_expr(&prepared) {
        if let Some(r) = eval_bitwise(&prepared) {
            return Some(MathOutcome { result: r, unit_aware: false });
        }
    }
    if let Ok(m) = evaluate_with_mode(&prepared, mode) {
        let main = m.result.trim().to_string();
        if !main.is_empty() {
            return Some(MathOutcome { result: main, unit_aware: m.unit_aware });
        }
    }
    // Bitwise ops in a programmer-math fallback (fend focuses on units/math).
    if let Some(r) = eval_bitwise(&prepared) {
        return Some(MathOutcome { result: r, unit_aware: false });
    }
    None
}

fn is_bitwise_expr(expr: &str) -> bool {
    if expr.contains("<<") || expr.contains(">>") || expr.contains('&') || expr.contains('|') {
        return true;
    }
    if let Some(pos) = expr.find('^') {
        let lhs = expr[..pos].trim();
        let rhs = expr[pos + 1..].trim();
        let is_programmer_num = |s: &str| {
            let s = s.trim().to_lowercase();
            s.starts_with("0x") || s.starts_with("0b") || s.starts_with("0o")
        };
        if is_programmer_num(lhs) || is_programmer_num(rhs) {
            return true;
        }
    }
    false
}

/// C-style bitwise evaluation for integer operands: & | ^ << >>.
fn eval_bitwise(expr: &str) -> Option<String> {
    for (pat, op) in [("<<", 0u8), (">>", 1), ("&", 2), ("|", 3), ("^", 4)] {
        if let Some(pos) = expr.find(pat) {
            let lhs = expr[..pos].trim();
            let rhs = expr[pos + pat.len()..].trim();
            let parse = |s: &str| -> Option<i128> {
                let s = s.trim();
                let lower = s.to_lowercase();
                if let Some(h) = lower.strip_prefix("0x") {
                    i128::from_str_radix(h, 16).ok()
                } else if let Some(o) = lower.strip_prefix("0o") {
                    i128::from_str_radix(o, 8).ok()
                } else if let Some(b) = lower.strip_prefix("0b") {
                    i128::from_str_radix(b, 2).ok()
                } else {
                    s.parse::<i128>().ok()
                }
            };
            if let (Some(a), Some(b)) = (parse(lhs), parse(rhs)) {
                let r = match op {
                    0 => a << b,
                    1 => a >> b,
                    2 => a & b,
                    3 => a | b,
                    _ => a ^ b,
                };
                return Some(format!("{r}"));
            }
        }
    }
    None
}

/// Rewrite trigonometric functions for degrees mode:
/// - Circular trig `sin`, `cos`, `tan` arguments receive `((arg) deg)` when lacking explicit angular units
/// - Inverse trig `asin`, `acos`, `atan` receive ` in degrees` when lacking explicit target units
/// - Hyperbolic functions `sinh`, `cosh`, `tanh`, `asinh`, `acosh`, `atanh` remain untouched (dimensionless)
pub fn apply_degrees_mode(expr: &str) -> String {
    let mut s = rewrite_direct_trig_degrees(expr);
    s = rewrite_inverse_trig_degrees(&s);
    s
}

fn rewrite_direct_trig_degrees(expr: &str) -> String {
    const DIRECT_TRIG: &[&str] = &["sin", "cos", "tan"];
    let chars: Vec<char> = expr.chars().collect();
    let len = chars.len();
    let mut out = String::with_capacity(len + 16);
    let mut i = 0;

    while i < len {
        let prev_char = if i == 0 { None } else { Some(chars[i - 1]) };
        let valid_prefix = prev_char.is_none_or(|p| !p.is_alphanumeric() && p != '_');

        if valid_prefix {
            let mut matched = None;
            for &func in DIRECT_TRIG {
                let func_len = func.len();
                if i + func_len <= len {
                    let candidate: String = chars[i..i + func_len].iter().collect();
                    if candidate.eq_ignore_ascii_case(func) {
                        let after_idx = i + func_len;
                        let valid_suffix = after_idx >= len || (!chars[after_idx].is_alphabetic() && chars[after_idx] != '_');
                        if valid_suffix {
                            matched = Some((func, func_len));
                            break;
                        }
                    }
                }
            }

            if let Some((func_name, func_len)) = matched {
                let mut paren_start = i + func_len;
                while paren_start < len && (chars[paren_start] == ' ' || chars[paren_start] == '\t') {
                    paren_start += 1;
                }

                if paren_start < len && chars[paren_start] == '(' {
                    let mut depth = 1;
                    let mut paren_end = paren_start + 1;
                    while paren_end < len && depth > 0 {
                        if chars[paren_end] == '(' {
                            depth += 1;
                        } else if chars[paren_end] == ')' {
                            depth -= 1;
                        }
                        paren_end += 1;
                    }

                    if depth == 0 {
                        let arg: String = chars[paren_start + 1..paren_end - 1].iter().collect();
                        let arg_lower = arg.to_lowercase();
                        let already_has_unit = arg_lower.contains("deg")
                            || arg_lower.contains("degree")
                            || arg_lower.contains("rad")
                            || arg_lower.contains("radian")
                            || arg_lower.contains('°');

                        let processed_arg = rewrite_direct_trig_degrees(&arg);
                        out.push_str(func_name);
                        out.push('(');
                        if already_has_unit {
                            out.push_str(&processed_arg);
                        } else {
                            out.push_str(&format!("(({processed_arg}) deg)"));
                        }
                        out.push(')');
                        i = paren_end;
                        continue;
                    }
                }
            }
        }

        out.push(chars[i]);
        i += 1;
    }

    out
}

fn rewrite_inverse_trig_degrees(expr: &str) -> String {
    let lower = expr.to_lowercase();
    let has_inverse = lower.contains("asin") || lower.contains("acos") || lower.contains("atan");
    if !has_inverse {
        return expr.to_string();
    }
    if lower.contains(" in ") || lower.contains(" to ") {
        return expr.to_string();
    }
    format!("{expr} in degrees")
}

/// Preprocess natural-language mathematical grammar replicating Antinote DDMathParser rules:
/// - Maps roots like `cuberoot(x)` to `cbrt(x)`
/// - Expands implicit multiplication: `4(12+3)` -> `4 * (12+3)`, `(2+3)(4+5)` -> `(2+3) * (4+5)`, `(2+3) 4` -> `(2+3) * 4`
/// - Rewrites mixed fractions `A B/C` into `((A) + ((B)/(C)))`
pub fn preprocess_math_grammar(expr: &str) -> String {
    let s = rewrite_cuberoot(expr);
    let s = rewrite_implicit_multiplication(&s);
    let s = rewrite_mixed_fractions(&s);
    s
}

fn rewrite_cuberoot(expr: &str) -> String {
    let lower = expr.to_lowercase();
    let mut out = String::with_capacity(expr.len());
    let mut search_from = 0;

    while let Some(rel) = lower[search_from..].find("cuberoot") {
        let start = search_from + rel;
        let end = start + "cuberoot".len();

        let boundary_before =
            expr[..start].chars().next_back().is_none_or(|c| !c.is_alphanumeric() && c != '_');
        if boundary_before {
            out.push_str(&expr[search_from..start]);
            out.push_str("cbrt");
            search_from = end;
        } else {
            out.push_str(&expr[search_from..end]);
            search_from = end;
        }
    }
    out.push_str(&expr[search_from..]);
    out
}

fn rewrite_mixed_fractions(expr: &str) -> String {
    let chars: Vec<char> = expr.chars().collect();
    let len = chars.len();
    let mut out = String::with_capacity(expr.len() + 16);
    let mut i = 0;

    while i < len {
        let mut start_idx = i;
        let mut has_unary_minus = false;

        // Check for unary minus: either start of expr or preceded by operator / open paren
        if chars[start_idx] == '-' {
            let is_unary = if start_idx == 0 {
                true
            } else {
                let prev_non_ws = chars[..start_idx].iter().rev().find(|c| !c.is_whitespace());
                match prev_non_ws {
                    None => true,
                    Some(&c) => {
                        c == '(' || c == '+' || c == '-' || c == '*' || c == '/' || c == '=' || c == ','
                    }
                }
            };
            if is_unary {
                let mut next = start_idx + 1;
                while next < len && chars[next].is_whitespace() {
                    next += 1;
                }
                if next < len && chars[next].is_ascii_digit() {
                    has_unary_minus = true;
                    start_idx = next;
                }
            }
        }

        // At start_idx, check if we have a standalone integer A
        let prev_char = if start_idx == 0 {
            None
        } else {
            Some(chars[start_idx - 1])
        };
        let valid_prefix = match prev_char {
            None => true,
            Some(c) => !c.is_alphanumeric() && c != '_' && c != '.' && c != '/',
        };

        if valid_prefix && start_idx < len && chars[start_idx].is_ascii_digit() {
            let a_start = start_idx;
            let mut a_end = a_start;
            while a_end < len && chars[a_end].is_ascii_digit() {
                a_end += 1;
            }

            // After A, must have at least one whitespace
            let mut ws_end = a_end;
            while ws_end < len && (chars[ws_end] == ' ' || chars[ws_end] == '\t') {
                ws_end += 1;
            }

            if ws_end > a_end && ws_end < len && chars[ws_end].is_ascii_digit() {
                let b_start = ws_end;
                let mut b_end = b_start;
                while b_end < len && chars[b_end].is_ascii_digit() {
                    b_end += 1;
                }

                // After B, must have '/'
                if b_end < len && chars[b_end] == '/' {
                    let c_start = b_end + 1;
                    if c_start < len && chars[c_start].is_ascii_digit() {
                        let mut c_end = c_start;
                        while c_end < len && chars[c_end].is_ascii_digit() {
                            c_end += 1;
                        }

                        // After C, must NOT be digit, '.', or '/'
                        let valid_suffix = if c_end >= len {
                            true
                        } else {
                            let next_c = chars[c_end];
                            !next_c.is_ascii_digit() && next_c != '.' && next_c != '/'
                        };

                        if valid_suffix {
                            let a_str: String = chars[a_start..a_end].iter().collect();
                            let b_str: String = chars[b_start..b_end].iter().collect();
                            let c_str: String = chars[c_start..c_end].iter().collect();

                            if has_unary_minus {
                                out.push_str(&format!("-(({a_str}) + (({b_str})/({c_str})))"));
                            } else {
                                out.push_str(&format!("(({a_str}) + (({b_str})/({c_str})))"));
                            }
                            i = c_end;
                            continue;
                        }
                    }
                }
            }
        }

        out.push(chars[i]);
        i += 1;
    }

    out
}

fn rewrite_implicit_multiplication(expr: &str) -> String {
    const KNOWN_FUNCS: &[&str] = &[
        "sin", "cos", "tan", "asin", "acos", "atan",
        "sinh", "cosh", "tanh", "asinh", "acosh", "atanh",
        "sqrt", "cbrt", "cuberoot",
    ];

    let chars: Vec<char> = expr.chars().collect();
    let len = chars.len();
    let mut out = String::with_capacity(len + 16);
    let mut i = 0;

    while i < len {
        let c = chars[i];

        // Case 1: `)` followed by `(` -> `) * (`
        if c == ')' {
            out.push(')');
            let mut next = i + 1;
            while next < len && (chars[next] == ' ' || chars[next] == '\t') {
                next += 1;
            }
            if next < len && chars[next] == '(' {
                out.push_str(" * ");
                i = next;
                continue;
            }
            // Case 2: `)` followed by a number -> `) * $num`
            if next < len && chars[next].is_ascii_digit() {
                out.push_str(" * ");
                i = next;
                continue;
            }
            // Case 3: `)` followed by function name -> `) * $func`
            let remaining: String = chars[next..].iter().collect();
            let mut matched_func = false;
            for func in KNOWN_FUNCS {
                if remaining.starts_with(func) {
                    let after_func = next + func.len();
                    let boundary = after_func >= len || !chars[after_func].is_alphanumeric();
                    if boundary {
                        out.push_str(" * ");
                        i = next;
                        matched_func = true;
                        break;
                    }
                }
            }
            if matched_func {
                continue;
            }
            i += 1;
            continue;
        }

        // Case 4: Standalone number before `(` or function name
        let prev_char = if i == 0 { None } else { Some(chars[i - 1]) };
        let is_standalone_num_start = c.is_ascii_digit()
            && prev_char.is_none_or(|p| !p.is_alphanumeric() && p != '_' && p != '.');

        if is_standalone_num_start {
            let num_start = i;
            let mut num_end = num_start;
            let mut seen_dot = false;
            while num_end < len {
                if chars[num_end].is_ascii_digit() {
                    num_end += 1;
                } else if chars[num_end] == '.' && !seen_dot {
                    if num_end + 1 < len && chars[num_end + 1].is_ascii_digit() {
                        seen_dot = true;
                        num_end += 2;
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }

            // Check what follows the number
            let mut next = num_end;
            while next < len && (chars[next] == ' ' || chars[next] == '\t') {
                next += 1;
            }

            if next < len {
                if chars[next] == '(' {
                    // Number followed by `(` -> insert ` * `
                    let num_str: String = chars[num_start..num_end].iter().collect();
                    out.push_str(&num_str);
                    out.push_str(" * ");
                    i = next;
                    continue;
                } else {
                    // Check if followed by known function name
                    let remaining: String = chars[next..].iter().collect();
                    let mut matched_func = false;
                    for func in KNOWN_FUNCS {
                        if remaining.starts_with(func) {
                            let after_func = next + func.len();
                            let boundary = after_func >= len || !chars[after_func].is_alphanumeric();
                            if boundary {
                                let num_str: String = chars[num_start..num_end].iter().collect();
                                out.push_str(&num_str);
                                out.push_str(" * ");
                                i = next;
                                matched_func = true;
                                break;
                            }
                        }
                    }
                    if matched_func {
                        continue;
                    }
                }
            }

            let num_str: String = chars[num_start..num_end].iter().collect();
            out.push_str(&num_str);
            i = num_end;
            continue;
        }

        out.push(c);
        i += 1;
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fend_arithmetic_and_units() {
        let m = evaluate("45 * 12").unwrap();
        assert_eq!(m.result.replace(',', ""), "540");
        let m = evaluate("5 km in miles").unwrap();
        assert!(m.result.contains("mile"), "got: {}", m.result);
        // Temperature: fend expects Kelvin casing; accept both spellings.
        let m = evaluate("72 fahrenheit to celsius").or_else(|_| evaluate("72 degF to degC"))
            .unwrap();
        assert!(
            m.result.to_lowercase().contains("celsius")
                || m.result.contains('°')
                || m.result.contains("deg")
                || m.result.contains('C'),
            "temp: {}",
            m.result
        );
    }

    #[test]
    fn math_grammar_evaluation() {
        // Mixed fractions
        let m = evaluate("1 1/2").unwrap();
        assert_eq!(m.result, "1.5");
        let m = evaluate("3 1/4 + 2").unwrap();
        assert_eq!(m.result, "5.25");
        let m = evaluate("2 1/3 + 1 2/3").unwrap();
        assert_eq!(m.result, "4");
        let m = evaluate("-1 1/2").unwrap();
        assert_eq!(m.result, "-1.5");

        // Exponents & scientific notation
        let m = evaluate("2^8").unwrap();
        assert_eq!(m.result, "256");
        let m = evaluate("2**8").unwrap();
        assert_eq!(m.result, "256");
        let m = evaluate("1e6").unwrap();
        assert_eq!(m.result.replace(',', ""), "1000000");

        // Implicit multiplication
        let m = evaluate("4(12+3)").unwrap();
        assert_eq!(m.result, "60");
        let m = evaluate("(2+3)(4+5)").unwrap();
        assert_eq!(m.result, "45");
        let m = evaluate("(2+3) 4").unwrap();
        assert_eq!(m.result, "20");
        let m = evaluate("5(2)").unwrap();
        assert_eq!(m.result, "10");
        let m = evaluate("3(4 + 2(1 + 1))").unwrap();
        assert_eq!(m.result, "24");

        // Roots
        let m = evaluate("cuberoot(27)").unwrap();
        assert_eq!(m.result, "3");
        let m = evaluate("sqrt(144)").unwrap();
        assert_eq!(m.result, "12");

        // Bitwise vs exponent
        let vars = Vars::new();
        let m = eval_line("0xFF ^ 0x0F", &vars).unwrap();
        assert_eq!(m.result, "240");
        let m = eval_line("2^8", &vars).unwrap();
        assert_eq!(m.result, "256");
    }

    #[test]
    fn variables_react() {
        let lines = vec!["rent = 12000", "food: 4500", "total rent * 12"];
        let vars = extract_variables(&lines);
        assert_eq!(vars.get("rent").unwrap(), "12000");
        assert_eq!(vars.get("food").unwrap(), "4500");
        let out = eval_line("rent * 12", &vars).unwrap();
        assert_eq!(out.result.replace(',', ""), "144000");
    }

    #[test]
    fn labelled_sum_uses_bindings() {
        let lines = vec!["rent 12000", "food 4500", "transport 1800"];
        let vars = extract_variables(&lines);
        let out = eval_line("rent + food + transport", &vars).unwrap();
        assert_eq!(out.result.replace(',', ""), "18300");
    }

    #[test]
    fn percent_forms() {
        let vars = Vars::new();
        let a = eval_line("20% of 850", &vars).unwrap();
        assert_eq!(a.result.replace(',', ""), "170");
        let b = eval_line("850 + 15%", &vars).unwrap();
        assert_eq!(b.result.replace(',', ""), "977.5");
    }

    #[test]
    fn base_conversions() {
        assert_eq!(base_conversion("0xFF & 0x0F").is_none(), true); // not a conversion
        assert_eq!(base_conversion("255 in binary").unwrap(), "0b11111111");
        assert_eq!(base_conversion("0xFF in decimal").unwrap(), "255");
        assert_eq!(base_conversion("12 in hex").unwrap(), "0xC");
    }

    #[test]
    fn bitwise_via_fend() {
        let vars = Vars::new();
        let out = eval_line("0xFF & 0x0F", &vars).unwrap();
        assert!(out.result == "15" || out.result == "0xf", "got: {}", out.result);
    }

    #[test]
    fn stats_respect_comment_marker() {
        let lines = vec!["12000", "4500", "// secret 999999", "buy milk"];
        let s = block_stats(&lines);
        assert_eq!(s.sum.unwrap(), 16500.0);
        assert_eq!(s.lines, 3); // excluded comment line
        assert_eq!(s.words, 4); // 12000 | 4500 | "buy milk" = 2 words; secret excluded
        assert_eq!(s.chars, lines.iter().map(|l| l.chars().count()).sum::<usize>());
    }

    #[test]
    fn substitute_handles_multiword_labels() {
        let lines = vec!["trees: 23405", "o2 per tree: 220"];
        let vars = extract_variables(&lines);
        let out = eval_line("trees x o2 per tree", &vars).unwrap();
        assert_eq!(out.result.replace(',', ""), "5149100");
    }

    #[test]
    fn eval_line_garbage_is_none() {
        assert!(eval_line("hello world", &Vars::new()).is_none());
        assert!(eval_line("", &Vars::new()).is_none());
    }

    #[test]
    fn angle_modes_and_trig() {
        // Degrees mode (default)
        let m = evaluate_with_mode("sin(90)", AngleMode::Degrees).unwrap();
        assert_eq!(m.result, "1");
        let m = evaluate_with_mode("cos(0)", AngleMode::Degrees).unwrap();
        assert_eq!(m.result, "1");
        let m = evaluate_with_mode("cos(90)", AngleMode::Degrees).unwrap();
        assert_eq!(m.result, "0");
        let m = evaluate_with_mode("tan(45)", AngleMode::Degrees).unwrap();
        assert!(m.result.contains('1'), "tan(45) deg: {}", m.result);
        let m = evaluate_with_mode("asin(1)", AngleMode::Degrees).unwrap();
        assert!(m.result.contains("90") && m.result.contains("deg"), "asin(1) deg: {}", m.result);

        // Radians mode
        let m = evaluate_with_mode("sin(pi/2)", AngleMode::Radians).unwrap();
        assert_eq!(m.result, "1");
        let m = evaluate_with_mode("cos(pi)", AngleMode::Radians).unwrap();
        assert_eq!(m.result, "-1");
        let m = evaluate_with_mode("tan(pi/4)", AngleMode::Radians).unwrap();
        assert!(m.result.contains('1'), "tan(pi/4) rad: {}", m.result);

        // Hyperbolic functions (unscaled)
        let m = evaluate_with_mode("sinh(0)", AngleMode::Degrees).unwrap();
        assert!(m.result.contains('0'), "sinh(0): {}", m.result);
        let m = evaluate_with_mode("cosh(0)", AngleMode::Degrees).unwrap();
        assert!(m.result.contains('1'), "cosh(0): {}", m.result);
        let m = evaluate_with_mode("tanh(0)", AngleMode::Degrees).unwrap();
        assert!(m.result.contains('0'), "tanh(0): {}", m.result);

        // eval_line_with_mode
        let vars = Vars::new();
        let m = eval_line_with_mode("sin(90)", &vars, AngleMode::Degrees).unwrap();
        assert_eq!(m.result, "1");
        let m = eval_line_with_mode("sin(pi/2)", &vars, AngleMode::Radians).unwrap();
        assert_eq!(m.result, "1");
    }
}
