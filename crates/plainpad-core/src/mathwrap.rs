//! Math layer: fend-backed evaluation plus deterministic pre-processing for the
//! scratchpad cases fend does not handle natively (labelled sums, percentages,
//! base conversion). Network is never touched: currency conversion is out of
//! scope here (PRD M-5 is P1 and requires an explicit opt-in shell hook).

use fend_core::Context;
use std::collections::BTreeMap;

/// Outcome of evaluating one expression.
#[derive(Debug, Clone, PartialEq)]
pub struct MathOutcome {
    /// The computed result, formatted for display.
    pub result: String,
    /// True when fend (or the built-in paths) resolved this as a quantity/unit.
    pub unit_aware: bool,
}

/// Evaluate an expression with fend. Deterministic: RNG disabled, fixed "current
/// time", terminal output mode, no network handlers installed.
/// A tiny in-memory cache keeps per-keystroke re-analysis well under the 30 ms
/// budget (PRD §6): repeated identical expressions return instantly.
pub fn evaluate(expr: &str) -> Result<MathOutcome, String> {
    use std::sync::{Mutex, OnceLock};
    type EvalCache = Mutex<Vec<(String, Result<MathOutcome, String>)>>;
    static CACHE: OnceLock<EvalCache> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(Vec::with_capacity(64)));
    let key = expr.trim().to_string();
    if let Some(hit) = cache.lock().unwrap().iter().find(|(k, _)| *k == key) {
        return hit.1.clone();
    }
    let outcome = evaluate_uncached(&key);
    {
        let mut c = cache.lock().unwrap();
        if c.len() >= 64 {
            let _ = c.remove(0);
        }
        c.push((key, outcome.clone()));
    }
    outcome
}

fn evaluate_uncached(expr: &str) -> Result<MathOutcome, String> {
    let mut ctx = Context::new();
    ctx.disable_rng();
    ctx.set_current_time_v1(0, 0);
    ctx.set_output_mode_terminal();
    let res = fend_core::evaluate(expr, &mut ctx)?;
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

/// Try the full evaluation stack for a note line: base conversion → percent
/// rewrite + variable substitution → raw fend. Returns None when nothing parses.
pub fn eval_line(expr: &str, vars: &Vars) -> Option<MathOutcome> {
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
    if let Ok(m) = evaluate(&prepared) {
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

/// C-style bitwise evaluation for integer operands: & | ^ << >>.
fn eval_bitwise(expr: &str) -> Option<String> {
    for (pat, op) in [("<<", 0u8), (">>", 1), ("&", 2), ("|", 3), ("^", 4)] {
        if let Some(pos) = expr.find(pat) {
            let lhs = expr[..pos].trim();
            let rhs = expr[pos + pat.len()..].trim();
            let parse = |s: &str| -> Option<i128> {
                let s = s.trim();
                if let Some(h) = s.strip_prefix("0x") {
                    i128::from_str_radix(h, 16).ok()
                } else if let Some(o) = s.strip_prefix("0o") {
                    i128::from_str_radix(o, 8).ok()
                } else if let Some(b) = s.strip_prefix("0b") {
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
}
