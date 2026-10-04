//! Per-line intent detection (PRD L-1..L-3): deterministic rules, confidence
//! bands, human-readable "why" for every inference, and plain-mode that
//! short-circuits everything. Text is never changed here — we only annotate.

use crate::mathwrap::{self, Vars};
use serde::Serialize;
use std::time::Instant;

/// What the UI should render for one line.
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Annotation {
    /// 0-based line index in the note.
    pub line: usize,
    /// Kind of intent: math | conversion | stats | timer | checklist | list | code | title.
    pub kind: &'static str,
    /// High (≥0.85): render inline. Medium (0.5..0.85): suggestion chip.
    pub confidence: f32,
    /// Display value for high confidence (e.g. the computed result).
    pub value: String,
    /// Chip label for medium confidence (e.g. "Make checklist (Tab)").
    pub chip: String,
    /// Human-readable explanation of why this inference was made (PRD L-2).
    pub why: String,
}

/// Result of analyzing a whole note.
#[derive(Serialize, Clone, Debug)]
pub struct Analysis {
    pub annotations: Vec<Annotation>,
    /// True when ≥60 % of non-empty lines look like source code (UI: monospace tint).
    pub code_mode: bool,
    /// Detected language tag when code_mode ("rust", "bash", …).
    pub code_lang: String,
    /// How long the analysis took (UI/monitoring for the <30 ms budget).
    pub elapsed_us: u128,
}

/// Analyze a note. `plain` = per-note or global plain mode: zero work, zero
/// annotations (PRD L-3). Never mutates text; runs in microseconds for
/// scratchpad-sized notes so it can run on every keystroke off the input path.
pub fn analyze(text: &str, plain: bool) -> Analysis {
    let start = Instant::now();
    if plain {
        return Analysis { annotations: Vec::new(), code_mode: false, code_lang: String::new(), elapsed_us: 0 };
    }
    let lines: Vec<&str> = text.lines().collect();

    // Explicit mode word on the first non-empty line: `math`, `list`, `code: bash`.
    let mut forced = ForcedMode::None;
    let mut forced_lang = String::new();
    if let Some(first) = lines.iter().find(|l| !l.trim().is_empty()) {
        let t = first.trim().to_lowercase();
        if t == "math" || t == "calcs" || t == "sum" {
            forced = ForcedMode::Math;
        } else if t == "list" {
            forced = ForcedMode::List;
        } else if let Some(lang) = t.strip_prefix("code:") {
            forced = ForcedMode::Code;
            forced_lang = lang.trim().to_string();
        } else if t == "code" {
            forced = ForcedMode::Code;
        }
    }

    let vars = mathwrap::extract_variables(&lines);
    let mut annotations = Vec::new();
    let mut code_lines = 0usize;
    let mut nonempty = 0usize;

    for (i, line) in lines.iter().enumerate() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        nonempty += 1;

        if is_mode_word(t) {
            continue;
        }

        // Timers (explicit phrases only — starting one by accident would be rude).
        if crate::timers::is_timer_stop_phrase(t) {
            annotations.push(Annotation {
                line: i,
                kind: "timer_stop",
                confidence: 0.95,
                value: String::new(),
                chip: "Stop timer (Enter)".into(),
                why: "phrase to stop running timer".into(),
            });
            continue;
        }

        if let Some((name, ms)) = crate::timers::parse_timer_phrase(t) {
            annotations.push(Annotation {
                line: i, kind: "timer", confidence: 0.95,
                value: format_duration(ms),
                chip: format!("Start timer ({})", if name == "Timer" { format_duration(ms) } else { name.clone() }),
                why: "starts with 'timer' or 'remind me in' followed by a duration".into(),
            });
            continue;
        }

        // Checklists: explicit prefix or todo phrase.
        if t.starts_with("[ ] ") || t.starts_with("[x] ") || t.starts_with("[X] ") {
            annotations.push(Annotation {
                line: i, kind: "checklist", confidence: 1.0, value: String::new(), chip: String::new(),
                why: "line starts with a checkbox marker".into(),
            });
            continue;
        }
        let lower = t.to_lowercase();
        if lower.starts_with("todo ") {
            annotations.push(Annotation {
                line: i, kind: "checklist", confidence: 0.95, value: String::new(),
                chip: "Make checklist (Tab)".into(),
                why: "starts with the word 'todo'".into(),
            });
            continue;
        }

        // Running totals: `total` or `sum` over previous lines (Antinote feature)
        if (lower == "total" || lower == "sum") && i > 0 {
            if let Some(s) = compute_running_total(&lines, i, &annotations, &vars) {
                let formatted = if s.fract() == 0.0 {
                    format!("{}", s as i64)
                } else {
                    format!("{:.2}", s)
                };
                annotations.push(Annotation {
                    line: i,
                    kind: "math",
                    confidence: 0.95,
                    value: formatted,
                    chip: String::new(),
                    why: "sum of numbers and evaluated results on preceding lines".into(),
                });
                continue;
            }
        }

        // Math / conversions / variables.
        if let Some((kind, conf, value, why)) = classify_math(t, &vars, forced == ForcedMode::Math) {
            annotations.push(Annotation {
                line: i, kind, confidence: conf, value, chip: String::new(), why,
            });
            continue;
        }

        // Code-likeness heuristic.
        if forced == ForcedMode::Code {
            code_lines += 1;
            annotations.push(Annotation {
                line: i, kind: "code", confidence: 0.9, value: forced_lang.clone(), chip: String::new(),
                why: "note mode is set to code".into(),
            });
        } else if looks_like_code(t) {
            code_lines += 1;
        }
    }

    // Checklist suggestion chip: a comma list of 3+ short items (medium confidence).
    for (i, line) in lines.iter().enumerate() {
        let t = line.trim();
        if t.is_empty() || annotations.iter().any(|a| a.line == i) {
            continue;
        }
        let items: Vec<&str> = t.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
        let wordy = items.iter().all(|s| s.chars().count() <= 40 && !s.contains(char::is_whitespace) || s.split_whitespace().count() <= 4);
        if items.len() >= 3 && wordy {
            annotations.push(Annotation {
                line: i, kind: "checklist", confidence: 0.6, value: String::new(),
                chip: format!("Make checklist ({}) (Tab)", items.len()),
                why: "comma-separated list of short items".into(),
            });
        }
    }

    let code_mode = forced == ForcedMode::Code || (nonempty > 0 && code_lines * 10 >= nonempty * 6);
    Analysis {
        annotations,
        code_mode,
        code_lang: forced_lang,
        elapsed_us: start.elapsed().as_micros(),
    }
}

#[derive(PartialEq)]
enum ForcedMode {
    None,
    Math,
    List,
    Code,
}

fn is_mode_word(t: &str) -> bool {
    let l = t.to_lowercase();
    l == "math"
        || l == "calcs"
        || l == "sum"
        || l == "list"
        || l == "code"
        || l == "paste"
        || l.starts_with("code:")
}

fn compute_running_total(
    lines: &[&str],
    current_idx: usize,
    annotations: &[Annotation],
    vars: &Vars,
) -> Option<f64> {
    let mut total = 0.0;
    let mut count = 0usize;

    for j in 0..current_idx {
        let line = lines[j].trim();
        if line.is_empty() || line.starts_with("//") {
            continue;
        }
        let lower = line.to_lowercase();
        if is_mode_word(line) || lower == "total" || lower == "sum" {
            continue;
        }

        // 1. If line j produced an evaluated math annotation, use its calculated result!
        // E.g. "200 * 120050 =" produced value "24,010,000" or "24010000"
        if let Some(anno) = annotations.iter().find(|a| a.line == j && a.kind == "math") {
            let cleaned = anno.value.replace(',', "").trim().to_string();
            if let Ok(v) = cleaned.parse::<f64>() {
                total += v;
                count += 1;
                continue;
            }
        }

        // 2. Or try evaluating line j as math directly
        if let Some(out) = mathwrap::eval_line(line, vars) {
            let cleaned = out.result.replace(',', "").trim().to_string();
            if let Ok(v) = cleaned.parse::<f64>() {
                total += v;
                count += 1;
                continue;
            }
        }

        // 3. Fallback: parse numbers / currency from labelled line e.g. `$10 books`, `food 4500`
        let nums = mathwrap::parse_leading_number(line);
        if let Some(&n) = nums.first() {
            total += n;
            count += 1;
        }
    }

    if count > 0 {
        Some(total)
    } else {
        None
    }
}

/// Classify a line as math/conversion. Returns (kind, confidence, value, why).
fn classify_math(t: &str, vars: &Vars, forced_math: bool) -> Option<(&'static str, f32, String, String)> {
    let lower = t.to_lowercase();
    let has_digit = t.chars().any(|c| c.is_ascii_digit());
    let has_math_op = t.contains(['+', '*', '/', '^', '%', '&', '|', '='])
        || t.contains(" x ")
        || t.contains(" X ")
        || (t.contains('-') && !t.starts_with("--"));
    let has_func = ["sqrt", "cbrt", "abs", "sin", "cos", "tan", "log", "ln", "exp", "floor", "ceil", "round", "min", "max"]
        .iter()
        .any(|&f| lower.contains(f) && lower.contains('('));
    let mentions_var = vars.keys().any(|k| contains_word(t, k));

    if !has_digit && !has_math_op && !has_func && !mentions_var && !forced_math {
        return None;
    }
    // Reject sentences: too many words that are not part of math.
    let words = t.split_whitespace().count();

    // Currency & crypto conversion (`$32 USD to JPY =`, `100 usd in inr`, `0.5 btc to usd`, `₹5,000 in usd`).
    if let Some((res, why)) = crate::currency::evaluate_currency(t) {
        return Some(("conversion", 0.95, res, why));
    }

    // Base conversion (255 in binary) is checked first so fend doesn't strip the base prefix.
    if let Some(res) = mathwrap::base_conversion(t) {
        return Some(("conversion", 0.92, res, "recognised `<number> in <base>`".into()));
    }

    // Unit conversion `5 km in miles`, `72 f to c` — try fend directly.
    if (lower.contains(" in ") || lower.contains(" to ")) && words <= 6 {
        if let Ok(m) = mathwrap::evaluate(t) {
            let main = m.result.trim().to_string();
            if !main.is_empty() {
                return Some(("conversion", 0.9, main, "recognised `<quantity> <unit> in <unit>`".into()));
            }
        }
    }

    // Percent forms.
    if lower.contains('%') {
        if let Some(out) = mathwrap::eval_line(t, vars) {
            return Some(("math", 0.9, out.result, "percentage arithmetic".into()));
        }
    }

    // Expressions, functions, variables, labelled sums.
    if has_math_op || has_func || mentions_var || forced_math {
        // `x` used as multiplication (Antinote-style `trees x o2 per tree`)
        let normalized = if t.contains(" x ") || t.ends_with(" x") || t.contains(" X ") {
            t.replace(" x ", " * ").replace(" X ", " * ")
        } else {
            t.to_string()
        };
        if let Some(out) = mathwrap::eval_line(&normalized, vars) {
            let numeric = out.result.chars().next().is_some_and(|c| c.is_ascii_digit() || c == '-' || c == '+');
            if numeric {
                let conf = if forced_math { 0.9 } else { 0.88 };
                let why: &str = if mentions_var {
                    "uses your named values on earlier lines"
                } else if has_func {
                    "mathematical function evaluation"
                } else {
                    "line is an arithmetic expression"
                };
                return Some(("math", conf, out.result, why.to_string()));
            }
        }
    }
    None
}

fn contains_word(hay: &str, word: &str) -> bool {
    let Some(pos) = hay.to_lowercase().find(&word.to_lowercase()) else { return false };
    let before_ok = hay[..pos].chars().next_back().is_none_or(|c| !(c.is_alphanumeric() || c == '_'));
    let after_ok = hay[pos + word.len()..].chars().next().is_none_or(|c| !(c.is_alphanumeric() || c == '_'));
    before_ok && after_ok
}

fn looks_like_code(t: &str) -> bool {
    let markers = ["; ", "{", "}", "(", ")", "=>", "::", "==", "&&", "||", "->", "#!", "$(", "];"];
    markers.iter().any(|m| t.contains(m))
        || t.starts_with("//")
        || t.starts_with('#')
        || t.starts_with("def ")
        || t.starts_with("fn ")
        || t.starts_with("let ")
        || t.starts_with("const ")
        || t.starts_with("import ")
        || t.starts_with("if ")
        || t.starts_with("for ")
}

/// Format a duration for display: 25 min → "25 min", 90 s → "1 min 30 s".
pub fn format_duration(ms: u64) -> String {
    let s = ms / 1000;
    if s < 60 {
        return format!("{s} s");
    }
    let m = s / 60;
    let rem = s % 60;
    if m < 60 {
        return if rem == 0 { format!("{m} min") } else { format!("{m} min {rem} s") };
    }
    let h = m / 60;
    let rem_m = m % 60;
    if rem_m == 0 { format!("{h} h") } else { format!("{h} h {rem_m} min") }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn math_lines_annotate() {
        let a = analyze("45 * 12", false);
        let m = a.annotations.iter().find(|x| x.kind == "math").expect("math annotation");
        assert!(m.confidence >= 0.85, "high confidence, got {}", m.confidence);
        assert_eq!(m.value.replace(',', ""), "540");
        assert!(!m.why.is_empty());
    }

    #[test]
    fn labelled_sum_with_bindings() {
        let text = "rent 12000\nfood 4500\ntransport 1800\nrent + food + transport";
        let a = analyze(text, false);
        let m = a.annotations.last().unwrap();
        assert_eq!(m.kind, "math");
        assert_eq!(m.value.replace(',', ""), "18300");
        assert!(m.why.contains("named values"));
    }

    #[test]
    fn percent_and_units_and_bases() {
        let a = analyze("20% of 850", false);
        assert_eq!(a.annotations[0].value.replace(',', ""), "170");

        let a = analyze("5 km in miles", false);
        assert_eq!(a.annotations[0].kind, "conversion");
        assert!(a.annotations[0].value.contains("mile"));

        let a = analyze("255 in binary", false);
        assert_eq!(a.annotations[0].value, "0b11111111");
    }

    #[test]
    fn checklist_and_todo_and_chip() {
        let a = analyze("todo call dentist", false);
        let c = a.annotations.iter().find(|x| x.kind == "checklist").unwrap();
        assert!(c.chip.contains("Tab"));

        let a = analyze("buy milk, eggs, bread", false);
        let c = a.annotations.iter().find(|x| x.chip.contains("Tab")).expect("chip");
        assert_eq!(c.kind, "checklist");
        assert!(c.confidence < 0.85, "medium confidence chip");
    }

    #[test]
    fn timer_phrase_detected() {
        let a = analyze("timer 25 min", false);
        let t = a.annotations.iter().find(|x| x.kind == "timer").unwrap();
        assert_eq!(t.value, "25 min");
    }

    #[test]
    fn plain_mode_does_nothing() {
        let a = analyze("45 * 12\ntimer 5 min", true);
        assert!(a.annotations.is_empty());
        assert_eq!(a.elapsed_us, 0);
    }

    #[test]
    fn code_mode_via_forced_word() {
        let text = "code: bash\nmkdir -p \"$BACKUP_DIR\"\nrsync -avh \"$SRC/\" \"$BACKUP_DIR/\"";
        let a = analyze(text, false);
        assert!(a.code_mode);
        assert_eq!(a.code_lang, "bash");
        assert!(a.annotations.iter().any(|x| x.kind == "code"));
    }

    #[test]
    fn prose_is_not_math() {
        let a = analyze("call mom about the 3 tickets", false);
        assert!(a.annotations.iter().all(|x| x.kind != "math"), "{:?}", a.annotations);
    }

    #[test]
    fn latency_budget_small_notes() {
        // PRD: intent-detection latency per keystroke < 30 ms. A typical scratchpad
        // note is well under 100 lines; measure the p95 over 200 runs.
        let text = "math\nrent 12000\nfood 4500\nrent + food\nbuy milk, eggs, bread\ntimer 10 min\n45 * 12\n5 km in miles\n".repeat(10);
        let mut worst = 0u128;
        for _ in 0..200 {
            let a = analyze(&text, false);
            worst = worst.max(a.elapsed_us);
        }
        assert!(worst < 30_000, "worst run took {worst}µs (budget 30_000µs)");
    }

    #[test]
    fn math_with_trailing_equals() {
        let text = "math\n\n200 * 120050 =";
        let a = analyze(text, false);
        println!("Annotations: {:?}", a.annotations);
        assert_eq!(a.annotations.len(), 1);
        assert_eq!(a.annotations[0].value.replace(',', ""), "24010000");
    }

    #[test]
    fn math_functions_and_powers() {
        let text = "calcs\nsqrt(16)\n2^8\nabs(-42)\n-5 + 20";
        let a = analyze(text, false);
        assert_eq!(a.annotations.len(), 4);
        assert_eq!(a.annotations[0].value, "4");
        assert_eq!(a.annotations[1].value, "256");
        assert_eq!(a.annotations[2].value, "42");
        assert_eq!(a.annotations[3].value, "15");
    }

    #[test]
    fn total_sums_evaluated_expressions() {
        let text = "math\n\n200 * 120050 =\n\ntotal";
        let a = analyze(text, false);
        let total_anno = a.annotations.iter().find(|x| x.line == 4).expect("total annotation");
        assert_eq!(total_anno.value.replace(',', ""), "24010000");
    }

    #[test]
    fn currency_conversions_annotated() {
        let text = "$32 USD to JPY =\n100 usd in inr\n50 eur to gbp\n₹5,000 in usd\n0.5 btc to usd";
        let a = analyze(text, false);
        assert_eq!(a.annotations.len(), 5);
        for anno in &a.annotations {
            assert_eq!(anno.kind, "conversion");
            assert!(anno.confidence >= 0.9);
            assert!(!anno.value.is_empty());
        }
    }
}
