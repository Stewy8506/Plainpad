//! Currency & crypto conversion engine with offline seed rates and live refresh.
//!
//! Supports natural language phrases such as:
//! - `$32 USD to JPY =`
//! - `100 usd in inr`
//! - `50 eur to gbp`
//! - `₹5,000 in usd`
//! - `0.5 btc to usd`
//! - `100 dollars in euros`
//! - `5000 yen to usd`
//! - `50 usd into eur`
//! - `50 usd = inr`

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{OnceLock, RwLock};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RatesSnapshot {
    pub result: Option<String>,
    pub time_last_update_unix: Option<u64>,
    pub time_last_update_utc: Option<String>,
    pub base_code: Option<String>,
    pub rates: HashMap<String, f64>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct RatesInfo {
    pub last_updated_unix: u64,
    pub last_updated_utc: String,
    pub currency_count: usize,
    pub is_seed: bool,
}

pub struct CurrencyEngine {
    rates: HashMap<String, f64>,
    last_updated_unix: u64,
    last_updated_utc: String,
    is_seed: bool,
}

impl CurrencyEngine {
    pub fn new() -> Self {
        // Load bundled seed rates
        let seed_json = include_str!("seed_rates.json");
        Self::from_json(seed_json, true).unwrap_or_else(|| Self {
            rates: Self::fallback_rates(),
            last_updated_unix: 0,
            last_updated_utc: "Seed fallback".into(),
            is_seed: true,
        })
    }

    fn fallback_rates() -> HashMap<String, f64> {
        let mut r = HashMap::new();
        r.insert("USD".into(), 1.0);
        r.insert("EUR".into(), 0.92);
        r.insert("GBP".into(), 0.79);
        r.insert("INR".into(), 83.5);
        r.insert("JPY".into(), 150.0);
        r.insert("CAD".into(), 1.36);
        r.insert("AUD".into(), 1.52);
        r.insert("BTC".into(), 1.0 / 85000.0);
        r.insert("ETH".into(), 1.0 / 2700.0);
        r.insert("SOL".into(), 1.0 / 120.0);
        r
    }

    pub fn from_json(json_str: &str, is_seed: bool) -> Option<Self> {
        let snap: RatesSnapshot = serde_json::from_str(json_str).ok()?;
        let mut rates = snap.rates;
        // Ensure USD is present as base
        rates.entry("USD".into()).or_insert(1.0);

        Some(Self {
            rates,
            last_updated_unix: snap.time_last_update_unix.unwrap_or(0),
            last_updated_utc: snap.time_last_update_utc.unwrap_or_else(|| "Unknown".into()),
            is_seed,
        })
    }

    pub fn update_from_json(&mut self, json_str: &str) -> bool {
        if let Some(loaded) = Self::from_json(json_str, false) {
            self.rates = loaded.rates;
            self.last_updated_unix = loaded.last_updated_unix;
            self.last_updated_utc = loaded.last_updated_utc;
            self.is_seed = false;
            true
        } else {
            false
        }
    }

    pub fn info(&self) -> RatesInfo {
        RatesInfo {
            last_updated_unix: self.last_updated_unix,
            last_updated_utc: self.last_updated_utc.clone(),
            currency_count: self.rates.len(),
            is_seed: self.is_seed,
        }
    }

    pub fn get_rate(&self, code: &str) -> Option<f64> {
        self.rates.get(&code.to_uppercase()).copied()
    }

    pub fn has_currency(&self, code: &str) -> bool {
        self.rates.contains_key(&code.to_uppercase())
    }

    /// Convert an amount from one currency to another using the USD pivot.
    pub fn convert(&self, amount: f64, from: &str, to: &str) -> Option<f64> {
        let from_rate = self.get_rate(from)?;
        let to_rate = self.get_rate(to)?;
        if from_rate <= 0.0 || to_rate <= 0.0 {
            return None;
        }
        let usd_val = amount / from_rate;
        Some(usd_val * to_rate)
    }
}

static GLOBAL_ENGINE: OnceLock<RwLock<CurrencyEngine>> = OnceLock::new();

pub fn global_engine() -> &'static RwLock<CurrencyEngine> {
    GLOBAL_ENGINE.get_or_init(|| RwLock::new(CurrencyEngine::new()))
}

/// Update global currency rates from a JSON string.
pub fn update_rates(json_str: &str) -> bool {
    let engine = global_engine();
    if let Ok(mut lock) = engine.write() {
        lock.update_from_json(json_str)
    } else {
        false
    }
}

/// Get metadata about the currently loaded rates.
pub fn get_rates_info() -> RatesInfo {
    let engine = global_engine();
    let lock = engine.read().unwrap();
    lock.info()
}

/// Format a converted amount with appropriate precision and symbols.
pub fn format_converted(amount: f64, target_code: &str) -> String {
    let upper = target_code.to_uppercase();
    let symbol = get_currency_symbol(&upper);

    if upper == "BTC" || upper == "ETH" || upper == "SOL" {
        if amount < 0.0001 {
            return format!("{:.8} {}", amount, upper);
        } else if amount < 1.0 {
            return format!("{:.6} {}", amount, upper);
        } else {
            return format!("{:.4} {}", amount, upper);
        }
    }

    // Zero-decimal currencies
    if upper == "JPY" || upper == "KRW" || upper == "VND" || upper == "IDR" || upper == "HUF" || upper == "CLP" {
        let rounded = amount.round();
        let formatted_num = format_with_commas(rounded as i64);
        if let Some(sym) = symbol {
            return format!("{}{}", sym, formatted_num);
        } else {
            return format!("{} {}", formatted_num, upper);
        }
    }

    // Standard 2-decimal fiat currencies
    let int_part = amount.trunc() as i64;
    let fract = (amount.fract().abs() * 100.0).round() as i64;
    let formatted_num = if fract >= 100 {
        format!("{}.00", format_with_commas(int_part + 1))
    } else {
        format!("{}.{:02}", format_with_commas(int_part), fract)
    };

    if let Some(sym) = symbol {
        format!("{}{}", sym, formatted_num)
    } else {
        format!("{} {}", formatted_num, upper)
    }
}

fn format_with_commas(mut n: i64) -> String {
    if n == 0 {
        return "0".to_string();
    }
    let is_neg = n < 0;
    if is_neg {
        n = -n;
    }
    let s = n.to_string();
    let mut out = String::new();
    let chars: Vec<char> = s.chars().collect();
    let len = chars.len();

    for (i, c) in chars.iter().enumerate() {
        out.push(*c);
        let rem = len - 1 - i;
        if rem > 0 && rem % 3 == 0 {
            out.push(',');
        }
    }
    if is_neg {
        format!("-{}", out)
    } else {
        out
    }
}

/// Symbol for display formatting.
fn get_currency_symbol(code: &str) -> Option<&'static str> {
    match code {
        "USD" => Some("$"),
        "EUR" => Some("€"),
        "GBP" => Some("£"),
        "INR" => Some("₹"),
        "JPY" => Some("¥"),
        "CAD" => Some("C$"),
        "AUD" => Some("A$"),
        "KRW" => Some("₩"),
        "BRL" => Some("R$"),
        "RUB" => Some("₽"),
        "TRY" => Some("₺"),
        "PLN" => Some("zł"),
        "THB" => Some("฿"),
        "ILS" => Some("₪"),
        "VND" => Some("₫"),
        "PHP" => Some("₱"),
        _ => None,
    }
}

/// Map colloquial names, symbols, or ISO codes to a canonical 3-letter currency code.
pub fn canonicalize_currency(raw: &str) -> Option<&'static str> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }

    // Direct symbol matches
    match trimmed {
        "$" => return Some("USD"),
        "€" => return Some("EUR"),
        "£" => return Some("GBP"),
        "₹" | "Rs" | "rs" | "Rs." | "rs." => return Some("INR"),
        "¥" => return Some("JPY"),
        "₩" => return Some("KRW"),
        "฿" => return Some("THB"),
        "₽" => return Some("RUB"),
        "₺" => return Some("TRY"),
        "zł" => return Some("PLN"),
        "₪" => return Some("ILS"),
        "₫" => return Some("VND"),
        "₱" => return Some("PHP"),
        "C$" | "c$" | "CA$" | "ca$" => return Some("CAD"),
        "A$" | "a$" | "AU$" | "au$" => return Some("AUD"),
        "NZ$" | "nz$" => return Some("NZD"),
        "HK$" | "hk$" => return Some("HKD"),
        "SG$" | "sg$" => return Some("SGD"),
        "R$" | "r$" => return Some("BRL"),
        _ => {}
    }

    let lower = trimmed.to_lowercase();
    match lower.as_str() {
        // USD
        "usd" | "dollar" | "dollars" | "buck" | "bucks" | "us dollar" | "us dollars" => Some("USD"),
        // EUR
        "eur" | "euro" | "euros" => Some("EUR"),
        // GBP
        "gbp" | "pound" | "pounds" | "quid" | "sterling" | "british pound" | "british pounds" => Some("GBP"),
        // INR
        "inr" | "rupee" | "rupees" => Some("INR"),
        // JPY
        "jpy" | "yen" | "japanese yen" => Some("JPY"),
        // CAD
        "cad" | "canadian dollar" | "canadian dollars" => Some("CAD"),
        // AUD
        "aud" | "australian dollar" | "australian dollars" | "aussie dollar" | "aussie dollars" => Some("AUD"),
        // CNY
        "cny" | "rmb" | "yuan" | "chinese yuan" => Some("CNY"),
        // CHF
        "chf" | "swiss franc" | "swiss francs" | "franc" | "francs" => Some("CHF"),
        // SGD
        "sgd" | "singapore dollar" | "singapore dollars" => Some("SGD"),
        // HKD
        "hkd" | "hong kong dollar" | "hong kong dollars" => Some("HKD"),
        // NZD
        "nzd" | "new zealand dollar" | "kiwi dollar" => Some("NZD"),
        // KRW
        "krw" | "won" | "korean won" => Some("KRW"),
        // BRL
        "brl" | "real" | "reais" | "brazilian real" => Some("BRL"),
        // RUB
        "rub" | "ruble" | "rubles" | "russian ruble" => Some("RUB"),
        // TRY
        "try" | "lira" | "turkish lira" => Some("TRY"),
        // ZAR
        "zar" | "rand" | "south african rand" => Some("ZAR"),
        // SEK
        "sek" | "krona" | "kronor" | "swedish krona" => Some("SEK"),
        // NOK
        "nok" | "norwegian krone" | "norwegian kroner" => Some("NOK"),
        // DKK
        "dkk" | "danish krone" | "danish kroner" => Some("DKK"),
        // PLN
        "pln" | "zloty" | "polish zloty" => Some("PLN"),
        // THB
        "thb" | "baht" | "thai baht" => Some("THB"),
        // IDR
        "idr" | "rupiah" | "indonesian rupiah" => Some("IDR"),
        // MYR
        "myr" | "ringgit" | "malaysian ringgit" => Some("MYR"),
        // PHP
        "php" | "peso" | "pesos" | "philippine peso" => Some("PHP"),
        // MXN
        "mxn" | "mexican peso" | "mexican pesos" => Some("MXN"),
        // AED
        "aed" | "dirham" | "dirhams" | "uae dirham" => Some("AED"),
        // SAR
        "sar" | "riyal" | "riyals" | "saudi riyal" => Some("SAR"),
        // ILS
        "ils" | "shekel" | "shekels" | "israeli shekel" => Some("ILS"),
        // Crypto
        "btc" | "bitcoin" | "bitcoins" => Some("BTC"),
        "eth" | "ether" | "ethereum" => Some("ETH"),
        "sol" | "solana" => Some("SOL"),
        _ => {
            // Check if uppercase matches a known currency in our engine
            let upper = trimmed.to_uppercase();
            if upper.len() == 3 {
                let engine = global_engine();
                if let Ok(lock) = engine.read() {
                    if lock.has_currency(&upper) {
                        return Some(Box::leak(upper.into_boxed_str()));
                    }
                }
            }
            None
        }
    }
}

/// Parse source amount and currency from string like `$32 USD`, `₹5,000`, `100 usd`, `50€`.
fn parse_amount_and_source(s: &str) -> Option<(f64, &'static str)> {
    let trimmed = s.trim();
    if trimmed.is_empty() {
        return None;
    }

    let mut candidate_from_symbol: Option<&'static str> = None;
    let mut working = trimmed;

    // Check multi-char symbol prefix first
    for prefix in &["C$", "c$", "CA$", "ca$", "A$", "a$", "AU$", "au$", "NZ$", "nz$", "HK$", "hk$", "SG$", "sg$", "R$", "r$", "Rs.", "rs.", "Rs", "rs"] {
        if working.starts_with(prefix) {
            candidate_from_symbol = canonicalize_currency(prefix);
            working = working[prefix.len()..].trim();
            break;
        }
    }

    // Single-char symbol prefix
    if candidate_from_symbol.is_none() {
        if let Some(first_char) = working.chars().next() {
            if let Some(canonical) = canonicalize_currency(&first_char.to_string()) {
                candidate_from_symbol = Some(canonical);
                working = working[first_char.len_utf8()..].trim();
            }
        }
    }

    // Trailing symbol
    if candidate_from_symbol.is_none() {
        if let Some(last_char) = working.chars().next_back() {
            if let Some(canonical) = canonicalize_currency(&last_char.to_string()) {
                candidate_from_symbol = Some(canonical);
                let cut_len = working.len() - last_char.len_utf8();
                working = working[..cut_len].trim();
            }
        }
    }

    // Now split `working` into tokens: find the numeric part and any currency word
    let parts: Vec<&str> = working.split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    let mut amount: Option<f64> = None;
    let mut word_curr: Option<&'static str> = None;

    for part in parts {
        let clean_num = part.replace(',', "");
        if let Ok(v) = clean_num.parse::<f64>() {
            if v >= 0.0 && amount.is_none() {
                amount = Some(v);
                continue;
            }
        }

        // Not a number; check if it's a currency alias
        if let Some(c) = canonicalize_currency(part) {
            word_curr = Some(c);
        }
    }

    let amt = amount?;
    // Word currency takes precedence over symbol if both exist, otherwise symbol
    let from_curr = word_curr.or(candidate_from_symbol)?;

    Some((amt, from_curr))
}

/// Parse target currency from string like `JPY`, `euros`, `usd`, `₹`.
fn parse_target_currency(s: &str) -> Option<&'static str> {
    let mut trimmed = s.trim().trim_end_matches(['?', '=', '.', ' ', '\t']);
    if trimmed.is_empty() {
        return None;
    }

    // If starts with "to " or "in " (e.g. from split edge cases)
    if let Some(rest) = trimmed.strip_prefix("to ") {
        trimmed = rest.trim();
    } else if let Some(rest) = trimmed.strip_prefix("in ") {
        trimmed = rest.trim();
    }

    canonicalize_currency(trimmed)
}

/// Parse a natural language currency conversion query.
/// Returns `(amount, from_currency_code, to_currency_code)` if matched.
pub fn parse_currency_phrase(line: &str) -> Option<(f64, &'static str, &'static str)> {
    let mut t = line.trim();
    // Strip trailing equals/question
    t = t.trim_end_matches(['=', '?', ' ', '\t']);

    // Strip leading "convert "
    if t.to_lowercase().starts_with("convert ") {
        t = t[8..].trim();
    }

    // Don't process empty or single token
    if t.is_empty() || !t.contains(char::is_whitespace) && !t.contains('=') {
        return None;
    }

    // Separators to try in priority order
    let separators = [" to ", " in ", " into ", " as ", " = "];
    for sep in separators {
        if let Some(pos) = t.to_lowercase().find(sep) {
            let left = &t[..pos];
            let right = &t[pos + sep.len()..];

            if let Some((amount, from_curr)) = parse_amount_and_source(left) {
                if let Some(to_curr) = parse_target_currency(right) {
                    return Some((amount, from_curr, to_curr));
                }
            }
        }
    }

    // Fallback: check " = " without spaces or trailing "=" format e.g. "50 usd = inr"
    if let Some((left, right)) = t.split_once('=') {
        if let Some((amount, from_curr)) = parse_amount_and_source(left) {
            if let Some(to_curr) = parse_target_currency(right) {
                return Some((amount, from_curr, to_curr));
            }
        }
    }

    // Fallback: check 3-word or 4-word space patterns: "100 usd inr", "$50 jpy", "100$ eur"
    let tokens: Vec<&str> = t.split_whitespace().collect();
    if tokens.len() == 2 {
        // e.g. "$50 jpy" or "50$ eur"
        if let Some((amount, from_curr)) = parse_amount_and_source(tokens[0]) {
            if let Some(to_curr) = parse_target_currency(tokens[1]) {
                return Some((amount, from_curr, to_curr));
            }
        }
    } else if tokens.len() == 3 {
        // e.g. "100 usd inr"
        let left = format!("{} {}", tokens[0], tokens[1]);
        if let Some((amount, from_curr)) = parse_amount_and_source(&left) {
            if let Some(to_curr) = parse_target_currency(tokens[2]) {
                return Some((amount, from_curr, to_curr));
            }
        }
    }

    None
}

/// Evaluates a line for currency conversion.
/// If valid, returns `Some((formatted_result, explanation))`.
pub fn evaluate_currency(line: &str) -> Option<(String, String)> {
    let (amount, from_curr, to_curr) = parse_currency_phrase(line)?;
    let engine = global_engine();
    let lock = engine.read().ok()?;

    let converted = lock.convert(amount, from_curr, to_curr)?;
    let formatted = format_converted(converted, to_curr);
    let why = format!("converted {} {} to {}", amount, from_curr, to_curr);

    Some((formatted, why))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canonicalize() {
        assert_eq!(canonicalize_currency("$"), Some("USD"));
        assert_eq!(canonicalize_currency("dollars"), Some("USD"));
        assert_eq!(canonicalize_currency("EUR"), Some("EUR"));
        assert_eq!(canonicalize_currency("euros"), Some("EUR"));
        assert_eq!(canonicalize_currency("₹"), Some("INR"));
        assert_eq!(canonicalize_currency("rupees"), Some("INR"));
        assert_eq!(canonicalize_currency("yen"), Some("JPY"));
        assert_eq!(canonicalize_currency("btc"), Some("BTC"));
        assert_eq!(canonicalize_currency("bitcoin"), Some("BTC"));
    }

    #[test]
    fn test_parse_phrases() {
        // $32 USD to JPY =
        let p1 = parse_currency_phrase("$32 USD to JPY =");
        assert_eq!(p1, Some((32.0, "USD", "JPY")));

        // 100 usd in inr
        let p2 = parse_currency_phrase("100 usd in inr");
        assert_eq!(p2, Some((100.0, "USD", "INR")));

        // 50 eur to gbp
        let p3 = parse_currency_phrase("50 eur to gbp");
        assert_eq!(p3, Some((50.0, "EUR", "GBP")));

        // ₹5,000 in usd
        let p4 = parse_currency_phrase("₹5,000 in usd");
        assert_eq!(p4, Some((5000.0, "INR", "USD")));

        // 0.5 btc to usd
        let p5 = parse_currency_phrase("0.5 btc to usd");
        assert_eq!(p5, Some((0.5, "BTC", "USD")));

        // 100 dollars in euros
        let p6 = parse_currency_phrase("100 dollars in euros");
        assert_eq!(p6, Some((100.0, "USD", "EUR")));

        // 50 usd = inr
        let p7 = parse_currency_phrase("50 usd = inr");
        assert_eq!(p7, Some((50.0, "USD", "INR")));

        // Non-currency line should be None
        assert_eq!(parse_currency_phrase("50 to 100"), None);
        assert_eq!(parse_currency_phrase("goto main"), None);
    }

    #[test]
    fn test_evaluation() {
        let res = evaluate_currency("100 usd in eur");
        assert!(res.is_some());
        let (val, why) = res.unwrap();
        assert!(val.starts_with('€') || val.ends_with("EUR"));
        assert!(why.contains("converted 100 USD to EUR"));
    }
}
