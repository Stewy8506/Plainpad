//! Per-line intent detection (PRD L-1..L-3): deterministic rules, confidence
//! bands, human-readable "why" for every inference, and plain-mode that
//! short-circuits everything. Text is never changed here — we only annotate.

use crate::mathwrap::{self, Vars};
use crate::trigger_config::{TriggerCategory, TriggerConfig};
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

/// Analyze a note using default trigger configuration.
pub fn analyze(text: &str, plain: bool) -> Analysis {
    analyze_with_config(text, plain, &TriggerConfig::default())
}

/// Analyze a note with specific `TriggerConfig`.
/// `plain` = per-note or global plain mode: zero work, zero annotations (PRD L-3).
pub fn analyze_with_config(text: &str, plain: bool, config: &TriggerConfig) -> Analysis {
    let start = Instant::now();
    if plain {
        return Analysis {
            annotations: Vec::new(),
            code_mode: false,
            code_lang: String::new(),
            elapsed_us: 0,
        };
    }
    let lines: Vec<&str> = text.lines().collect();

    let first_nonempty_idx = lines.iter().position(|l| !l.trim().is_empty());

    // Explicit mode word on the first non-empty line: `math`, `list`, `code: bash`.
    let mut forced = ForcedMode::None;
    let mut forced_lang = String::new();
    if let Some(first_idx) = first_nonempty_idx {
        let t = lines[first_idx].trim().to_lowercase();
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

        // First non-empty line mode declaration skips annotation processing if mode word
        if Some(i) == first_nonempty_idx && is_first_line_mode_word(t) {
            continue;
        }

        // Check natural-language sentence triggers
        if !config.disable_all_keywords {
            if let Some((category, _kw, remaining_args)) = config.match_trigger(t) {
                match category {
                    TriggerCategory::Sum => {
                        let preceding = extract_preceding_block(&lines, i, config);
                        if let Some(formatted) = compute_block_sum(&lines, &preceding, &annotations, &vars) {
                            annotations.push(Annotation {
                                line: i,
                                kind: "math",
                                confidence: 0.95,
                                value: formatted,
                                chip: String::new(),
                                why: "sum of numbers and currencies in preceding paragraph or list".into(),
                            });
                            continue;
                        }
                    }
                    TriggerCategory::Avg => {
                        let preceding = extract_preceding_block(&lines, i, config);
                        if let Some(formatted) = compute_block_avg(&lines, &preceding, &annotations, &vars) {
                            annotations.push(Annotation {
                                line: i,
                                kind: "math",
                                confidence: 0.95,
                                value: formatted,
                                chip: String::new(),
                                why: "arithmetic mean of values in preceding paragraph or list".into(),
                            });
                            continue;
                        }
                    }
                    TriggerCategory::Count => {
                        let preceding = extract_preceding_block(&lines, i, config);
                        let count = compute_block_count(&preceding);
                        let label = if count == 1 { "1 item".to_string() } else { format!("{count} items") };
                        annotations.push(Annotation {
                            line: i,
                            kind: "stats",
                            confidence: 0.95,
                            value: label,
                            chip: String::new(),
                            why: "count of items in preceding list".into(),
                        });
                        continue;
                    }
                    TriggerCategory::Math => {
                        if !remaining_args.is_empty() {
                            if let Some((res, why)) = crate::currency::evaluate_currency(remaining_args) {
                                annotations.push(Annotation {
                                    line: i,
                                    kind: "math",
                                    confidence: 0.95,
                                    value: res,
                                    chip: String::new(),
                                    why,
                                });
                                continue;
                            }
                            if let Some((res, why)) = crate::units::parse_and_convert(remaining_args) {
                                annotations.push(Annotation {
                                    line: i,
                                    kind: "math",
                                    confidence: 0.95,
                                    value: res,
                                    chip: String::new(),
                                    why,
                                });
                                continue;
                            }
                            if let Some(res) = mathwrap::base_conversion(remaining_args) {
                                annotations.push(Annotation {
                                    line: i,
                                    kind: "math",
                                    confidence: 0.95,
                                    value: res,
                                    chip: String::new(),
                                    why: "base conversion".into(),
                                });
                                continue;
                            }
                            if let Some(out) = mathwrap::eval_line(remaining_args, &vars) {
                                annotations.push(Annotation {
                                    line: i,
                                    kind: "math",
                                    confidence: 0.95,
                                    value: out.result,
                                    chip: String::new(),
                                    why: "evaluated math expression".into(),
                                });
                                continue;
                            }
                            if let Ok(out) = mathwrap::evaluate(remaining_args) {
                                let res = out.result.trim().to_string();
                                if !res.is_empty() {
                                    annotations.push(Annotation {
                                        line: i,
                                        kind: "math",
                                        confidence: 0.95,
                                        value: res,
                                        chip: String::new(),
                                        why: "evaluated math expression".into(),
                                    });
                                    continue;
                                }
                            }
                        }
                    }
                    TriggerCategory::List => {
                        annotations.push(Annotation {
                            line: i,
                            kind: "list",
                            confidence: 0.85,
                            value: String::new(),
                            chip: "Convert to bullet list (Tab)".into(),
                            why: "list trigger keyword".into(),
                        });
                        continue;
                    }
                    TriggerCategory::Check => {
                        annotations.push(Annotation {
                            line: i,
                            kind: "checklist",
                            confidence: 0.85,
                            value: String::new(),
                            chip: "Convert to checklist (Tab)".into(),
                            why: "check trigger keyword".into(),
                        });
                        continue;
                    }
                    TriggerCategory::Code => {
                        annotations.push(Annotation {
                            line: i,
                            kind: "code",
                            confidence: 0.85,
                            value: remaining_args.trim().to_string(),
                            chip: "Format as code block (Tab)".into(),
                            why: "code trigger keyword".into(),
                        });
                        continue;
                    }
                    TriggerCategory::Paste => {
                        annotations.push(Annotation {
                            line: i,
                            kind: "paste",
                            confidence: 0.95,
                            value: String::new(),
                            chip: "Start AutoPaste (Enter)".into(),
                            why: "trigger for continuous clipboard capture".into(),
                        });
                        continue;
                    }
                    TriggerCategory::Timer => {
                        let phrase = if _kw.to_lowercase() == "timer" {
                            t.to_string()
                        } else if remaining_args.is_empty() {
                            "timer".to_string()
                        } else {
                            format!("timer {}", remaining_args)
                        };
                        if let Some(tp) = crate::timers::parse_timer_phrase(&phrase) {
                            match tp {
                                crate::timers::TimerPhrase::StartStopwatch => {
                                    annotations.push(Annotation {
                                        line: i,
                                        kind: "timer",
                                        confidence: 0.95,
                                        value: "00:00".into(),
                                        chip: "Start stopwatch (Enter)".into(),
                                        why: "starts a stopwatch".into(),
                                    });
                                }
                                crate::timers::TimerPhrase::StartCountdown { duration_ms, name } => {
                                    annotations.push(Annotation {
                                        line: i,
                                        kind: "timer",
                                        confidence: 0.95,
                                        value: format_duration(duration_ms),
                                        chip: format!("Start timer ({}) (Enter)", if name == "Timer" { format_duration(duration_ms) } else { name }),
                                        why: "countdown timer trigger".into(),
                                    });
                                }
                                crate::timers::TimerPhrase::StartPomodoro { work_ms, break_ms, .. } => {
                                    annotations.push(Annotation {
                                        line: i,
                                        kind: "timer",
                                        confidence: 0.95,
                                        value: format!("Pomodoro ({}m/{}m)", work_ms / 60_000, break_ms / 60_000),
                                        chip: "Start Pomodoro (Enter)".into(),
                                        why: "pomodoro trigger".into(),
                                    });
                                }
                                crate::timers::TimerPhrase::Control(crate::timers::TimerControlAction::PauseResume) => {
                                    annotations.push(Annotation {
                                        line: i,
                                        kind: "timer_control",
                                        confidence: 0.95,
                                        value: "pause".into(),
                                        chip: "Pause / Resume timer (Enter)".into(),
                                        why: "phrase to pause or resume active timer".into(),
                                    });
                                }
                                crate::timers::TimerPhrase::Control(crate::timers::TimerControlAction::Restart) => {
                                    annotations.push(Annotation {
                                        line: i,
                                        kind: "timer_control",
                                        confidence: 0.95,
                                        value: "restart".into(),
                                        chip: "Restart timer (Enter)".into(),
                                        why: "phrase to restart active timer".into(),
                                    });
                                }
                                crate::timers::TimerPhrase::Control(crate::timers::TimerControlAction::Stop) => {
                                    annotations.push(Annotation {
                                        line: i,
                                        kind: "timer_stop",
                                        confidence: 0.95,
                                        value: "stop".into(),
                                        chip: "Stop timer (Enter)".into(),
                                        why: "phrase to stop running timer".into(),
                                    });
                                }
                            }
                            continue;
                        }
                    }
                }
            }
        }

        // Timers (explicit phrases only — starting one by accident would be rude).
        if !config.disable_all_keywords {
            if let Some(tp) = crate::timers::parse_timer_phrase(t) {
                match tp {
                    crate::timers::TimerPhrase::StartStopwatch => {
                        annotations.push(Annotation {
                            line: i,
                            kind: "timer",
                            confidence: 0.95,
                            value: "00:00".into(),
                            chip: "Start stopwatch (Enter)".into(),
                            why: "starts a stopwatch".into(),
                        });
                        continue;
                    }
                    crate::timers::TimerPhrase::StartCountdown { duration_ms, name } => {
                        annotations.push(Annotation {
                            line: i,
                            kind: "timer",
                            confidence: 0.95,
                            value: format_duration(duration_ms),
                            chip: format!("Start timer ({}) (Enter)", if name == "Timer" { format_duration(duration_ms) } else { name }),
                            why: "starts with 'timer' or 'remind me in' followed by a duration".into(),
                        });
                        continue;
                    }
                    crate::timers::TimerPhrase::StartPomodoro { work_ms, break_ms, .. } => {
                        annotations.push(Annotation {
                            line: i,
                            kind: "timer",
                            confidence: 0.95,
                            value: format!("Pomodoro ({}m/{}m)", work_ms / 60_000, break_ms / 60_000),
                            chip: "Start Pomodoro (Enter)".into(),
                            why: "pomodoro trigger".into(),
                        });
                        continue;
                    }
                    crate::timers::TimerPhrase::Control(crate::timers::TimerControlAction::PauseResume) => {
                        annotations.push(Annotation {
                            line: i,
                            kind: "timer_control",
                            confidence: 0.95,
                            value: "pause".into(),
                            chip: "Pause / Resume timer (Enter)".into(),
                            why: "phrase to pause or resume active timer".into(),
                        });
                        continue;
                    }
                    crate::timers::TimerPhrase::Control(crate::timers::TimerControlAction::Restart) => {
                        annotations.push(Annotation {
                            line: i,
                            kind: "timer_control",
                            confidence: 0.95,
                            value: "restart".into(),
                            chip: "Restart timer (Enter)".into(),
                            why: "phrase to restart active timer".into(),
                        });
                        continue;
                    }
                    crate::timers::TimerPhrase::Control(crate::timers::TimerControlAction::Stop) => {
                        annotations.push(Annotation {
                            line: i,
                            kind: "timer_stop",
                            confidence: 0.95,
                            value: "stop".into(),
                            chip: "Stop timer (Enter)".into(),
                            why: "phrase to stop running timer".into(),
                        });
                        continue;
                    }
                }
            }
        }

        // Checklists: explicit prefix or todo phrase.
        let is_checkbox_marker = t.starts_with("[ ] ") || t.starts_with("[x] ") || t.starts_with("[X] ")
            || t.starts_with("- [ ] ") || t.starts_with("- [x] ") || t.starts_with("- [X] ")
            || t.starts_with("* [ ] ") || t.starts_with("* [x] ") || t.starts_with("* [X] ")
            || t.starts_with("+ [ ] ") || t.starts_with("+ [x] ") || t.starts_with("+ [X] ");
        if is_checkbox_marker {
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

        // Physical unit conversion (Distance, Area, Mass, Volume, Temperature)
        if let Some((res, why)) = crate::units::parse_and_convert(t) {
            annotations.push(Annotation {
                line: i,
                kind: "conversion",
                confidence: 0.95,
                chip: format!("Convert ({res})"),
                value: res,
                why,
            });
            continue;
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

fn is_first_line_mode_word(t: &str) -> bool {
    let l = t.to_lowercase();
    l == "math" || l == "calcs"
}

/// Extracts preceding block lines within the same paragraph/list scope.
/// Scans upward from `trigger_idx - 1` until reaching an empty line (after entering the block),
/// a markdown heading (`#`), or another trigger line. Returns lines in top-to-bottom order with indices.
pub fn extract_preceding_block<'a>(
    lines: &[&'a str],
    trigger_idx: usize,
    config: &TriggerConfig,
) -> Vec<(usize, &'a str)> {
    if trigger_idx == 0 {
        return Vec::new();
    }
    let mut block = Vec::new();
    let mut entered_block = false;

    for j in (0..trigger_idx).rev() {
        let raw = lines[j];
        let t = raw.trim();
        if t.is_empty() {
            if entered_block {
                break;
            }
            continue;
        }
        if t.starts_with('#') {
            break;
        }
        if !config.disable_all_keywords && config.match_trigger(t).is_some() {
            break;
        }
        if t.starts_with("//") {
            continue;
        }
        entered_block = true;
        block.push((j, raw));
    }
    block.reverse();
    block
}

fn extract_currency_symbol(s: &str) -> Option<String> {
    for sym in ["$", "€", "£", "₹", "¥", "₩", "₿"] {
        if s.contains(sym) {
            return Some(sym.to_string());
        }
    }
    for code in ["USD", "EUR", "GBP", "INR", "JPY", "CAD", "AUD", "NZD", "CHF", "CNY"] {
        if s.contains(code) {
            let sym = match code {
                "USD" | "CAD" | "AUD" | "NZD" => "$",
                "EUR" => "€",
                "GBP" => "£",
                "INR" => "₹",
                "JPY" | "CNY" => "¥",
                _ => return Some(code.to_string()),
            };
            return Some(sym.to_string());
        }
    }
    None
}

fn parse_line_amount_and_currency(line: &str) -> Option<(f64, Option<String>, bool)> {
    let t = line.trim();
    if t.is_empty() || t.starts_with("//") {
        return None;
    }

    let line_sym = extract_currency_symbol(t);
    let tokens: Vec<&str> = t.split_whitespace().collect();

    let mut candidates = Vec::new();
    for token in tokens {
        let cleaned_punct = token.trim_matches(|c: char| c == ':' || c == ';' || c == ',');
        if cleaned_punct.is_empty() {
            continue;
        }
        let token_sym = extract_currency_symbol(cleaned_punct);
        let num_str = cleaned_punct.replace(['$', '€', '£', '₹', '¥', '₩', '₿', ','], "");
        if let Ok(v) = num_str.parse::<f64>() {
            let has_dec = num_str.contains('.');
            let sym = token_sym.or_else(|| line_sym.clone());
            candidates.push((v, sym, has_dec));
        }
    }

    if let Some(c) = candidates.iter().find(|(_, s, _)| s.is_some()) {
        return Some(c.clone());
    }
    candidates.last().cloned()
}

fn compute_block_sum(
    _lines: &[&str],
    preceding: &[(usize, &str)],
    annotations: &[Annotation],
    vars: &Vars,
) -> Option<String> {
    if preceding.is_empty() {
        return None;
    }
    let mut total = 0.0;
    let mut count = 0usize;
    let mut currencies: Vec<String> = Vec::new();
    let mut any_decimals = false;

    for &(idx, raw_line) in preceding {
        let line = raw_line.trim();

        // 1. Line already evaluated as math or conversion
        if let Some(anno) = annotations.iter().find(|a| a.line == idx && (a.kind == "math" || a.kind == "conversion")) {
            let val = &anno.value;
            let sym = extract_currency_symbol(val);
            let cleaned = val.replace(['$', '€', '£', '₹', '¥', '₩', '₿', ','], "");
            if let Ok(v) = cleaned.trim().parse::<f64>() {
                total += v;
                count += 1;
                if val.contains('.') {
                    any_decimals = true;
                }
                if let Some(s) = sym {
                    currencies.push(s);
                }
                continue;
            }
        }

        // 2. Math evaluation of line
        if let Some(out) = mathwrap::eval_line(line, vars) {
            let sym = extract_currency_symbol(&out.result).or_else(|| extract_currency_symbol(line));
            let cleaned = out.result.replace(['$', '€', '£', '₹', '¥', '₩', '₿', ','], "");
            if let Ok(v) = cleaned.trim().parse::<f64>() {
                total += v;
                count += 1;
                if out.result.contains('.') {
                    any_decimals = true;
                }
                if let Some(s) = sym {
                    currencies.push(s);
                }
                continue;
            }
        }

        // 3. Fallback: parse amount and currency
        if let Some((amount, sym, has_dec)) = parse_line_amount_and_currency(line) {
            total += amount;
            count += 1;
            if has_dec {
                any_decimals = true;
            }
            if let Some(s) = sym {
                currencies.push(s);
            }
        }
    }

    if count == 0 {
        return None;
    }

    let common_sym = if !currencies.is_empty() && currencies.iter().all(|c| c == &currencies[0]) {
        currencies[0].as_str()
    } else {
        ""
    };

    let formatted = if any_decimals || total.fract().abs() > 1e-9 {
        format!("{}{:.2}", common_sym, total)
    } else {
        format!("{}{}", common_sym, total.round() as i64)
    };

    Some(formatted)
}

fn compute_block_avg(
    _lines: &[&str],
    preceding: &[(usize, &str)],
    annotations: &[Annotation],
    vars: &Vars,
) -> Option<String> {
    if preceding.is_empty() {
        return None;
    }
    let mut total = 0.0;
    let mut count = 0usize;
    let mut currencies: Vec<String> = Vec::new();

    for &(idx, raw_line) in preceding {
        let line = raw_line.trim();

        if let Some(anno) = annotations.iter().find(|a| a.line == idx && (a.kind == "math" || a.kind == "conversion")) {
            let val = &anno.value;
            let sym = extract_currency_symbol(val);
            let cleaned = val.replace(['$', '€', '£', '₹', '¥', '₩', '₿', ','], "");
            if let Ok(v) = cleaned.trim().parse::<f64>() {
                total += v;
                count += 1;
                if let Some(s) = sym {
                    currencies.push(s);
                }
                continue;
            }
        }

        if let Some(out) = mathwrap::eval_line(line, vars) {
            let sym = extract_currency_symbol(&out.result).or_else(|| extract_currency_symbol(line));
            let cleaned = out.result.replace(['$', '€', '£', '₹', '¥', '₩', '₿', ','], "");
            if let Ok(v) = cleaned.trim().parse::<f64>() {
                total += v;
                count += 1;
                if let Some(s) = sym {
                    currencies.push(s);
                }
                continue;
            }
        }

        if let Some((amount, sym, _)) = parse_line_amount_and_currency(line) {
            total += amount;
            count += 1;
            if let Some(s) = sym {
                currencies.push(s);
            }
        }
    }

    if count == 0 {
        return None;
    }

    let avg = total / count as f64;
    let common_sym = if !currencies.is_empty() && currencies.iter().all(|c| c == &currencies[0]) {
        currencies[0].as_str()
    } else {
        ""
    };

    let formatted = if avg.fract().abs() < 1e-9 {
        format!("{}{}", common_sym, avg.round() as i64)
    } else {
        format!("{}{:.2}", common_sym, avg)
    };

    Some(formatted)
}

fn compute_block_count(preceding: &[(usize, &str)]) -> usize {
    if preceding.is_empty() {
        return 0;
    }
    if preceding.len() == 1 {
        let single = preceding[0].1.trim();
        let items: Vec<&str> = single.split(',').map(|s| s.trim()).filter(|s| !s.is_empty()).collect();
        if items.len() > 1 {
            return items.len();
        }
        return 1;
    }
    preceding.len()
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

    // Physical unit conversion (`5 cuerdas in acres`, `14 st in kg`, `2 uk pint in ml`).
    if let Some((res, why)) = crate::units::parse_and_convert(t) {
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

        let a = analyze("- [ ] buy milk\n* [x] done chore\n[ ] bracket task", false);
        let checklist_annos: Vec<_> = a.annotations.iter().filter(|x| x.kind == "checklist").collect();
        assert_eq!(checklist_annos.len(), 3);
        assert_eq!(checklist_annos[0].line, 0);
        assert_eq!(checklist_annos[1].line, 1);
        assert_eq!(checklist_annos[2].line, 2);
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

    #[test]
    fn physical_unit_conversions_annotated() {
        let text = "5 cuerdas in acres\n14 st in kg\n2 uk pint in ml\n32 fahrenheit in celsius";
        let a = analyze(text, false);
        assert_eq!(a.annotations.len(), 4);
        assert_eq!(a.annotations[0].kind, "conversion");
        assert!(a.annotations[0].value.contains("4.856"));
        assert!(a.annotations[0].chip.contains("Convert"));
        assert_eq!(a.annotations[1].kind, "conversion");
        assert!(a.annotations[1].value.contains("88.9"));
        assert_eq!(a.annotations[2].kind, "conversion");
        assert!(a.annotations[2].value.contains("1136.5"));
        assert_eq!(a.annotations[3].kind, "conversion");
        assert_eq!(a.annotations[3].value, "0 °C");
    }
}
