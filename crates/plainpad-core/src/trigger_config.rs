//! Natural-language trigger configuration, alias registry, and matching engine.
//! Supports all 9 Antinote trigger categories:
//! sum, avg, math, count, list, check, code, paste, timer.

use serde::{Deserialize, Serialize};

/// The 9 natural-language trigger categories defined in Antinote SPEC §5.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TriggerCategory {
    Sum,
    Avg,
    Math,
    Count,
    List,
    Check,
    Code,
    Paste,
    Timer,
}

/// Configuration and alias mapping for sentence-level triggers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TriggerConfig {
    /// Master toggle to disable all trigger processing.
    pub disable_all_keywords: bool,
    /// Whether inline trailing arguments are permitted after the trigger keyword.
    pub allow_arguments_after_keyword: bool,
    /// Aliases for Sum triggers (default: sum, total, add).
    pub sum_triggers: Vec<String>,
    /// Aliases for Average triggers (default: avg, average).
    pub avg_triggers: Vec<String>,
    /// Aliases for Math triggers (default: math, convert, model).
    pub math_triggers: Vec<String>,
    /// Aliases for Count triggers (default: count).
    pub count_triggers: Vec<String>,
    /// Aliases for List triggers (default: list).
    pub list_triggers: Vec<String>,
    /// Aliases for Check / Checklist triggers (default: check, todo).
    pub check_triggers: Vec<String>,
    /// Aliases for Code block triggers (default: code).
    pub code_triggers: Vec<String>,
    /// Aliases for Paste / AutoPaste triggers (default: paste).
    pub paste_triggers: Vec<String>,
    /// Aliases for Timer triggers (default: timer).
    pub timer_triggers: Vec<String>,
    /// Optional custom keyword label.
    pub custom_keyword_label: Option<String>,
    /// Optional custom keyword description.
    pub custom_keyword_description: Option<String>,
}

impl Default for TriggerConfig {
    fn default() -> Self {
        Self {
            disable_all_keywords: false,
            allow_arguments_after_keyword: true,
            sum_triggers: vec!["sum".into(), "total".into(), "add".into()],
            avg_triggers: vec!["avg".into(), "average".into()],
            math_triggers: vec!["math".into(), "convert".into(), "model".into()],
            count_triggers: vec!["count".into()],
            list_triggers: vec!["list".into()],
            check_triggers: vec!["check".into(), "todo".into()],
            code_triggers: vec!["code".into()],
            paste_triggers: vec!["paste".into()],
            timer_triggers: vec!["timer".into()],
            custom_keyword_label: None,
            custom_keyword_description: None,
        }
    }
}

impl TriggerConfig {
    /// Matches a line of text against registered trigger categories and aliases.
    ///
    /// Returns `Some((category, matched_keyword, remaining_args))` if a trigger is identified.
    /// If `disable_all_keywords` is true, always returns `None`.
    /// If `allow_arguments_after_keyword` is false, only matches standalone keyword lines.
    pub fn match_trigger<'a>(&self, line: &'a str) -> Option<(TriggerCategory, &'a str, &'a str)> {
        if self.disable_all_keywords {
            return None;
        }

        let trimmed_start = line.trim_start();
        if trimmed_start.is_empty() {
            return None;
        }
        let leading_spaces = line.len() - trimmed_start.len();
        let trimmed = trimmed_start.trim_end();

        let categories: [(TriggerCategory, &[String]); 9] = [
            (TriggerCategory::Sum, &self.sum_triggers),
            (TriggerCategory::Avg, &self.avg_triggers),
            (TriggerCategory::Count, &self.count_triggers),
            (TriggerCategory::Math, &self.math_triggers),
            (TriggerCategory::List, &self.list_triggers),
            (TriggerCategory::Check, &self.check_triggers),
            (TriggerCategory::Code, &self.code_triggers),
            (TriggerCategory::Paste, &self.paste_triggers),
            (TriggerCategory::Timer, &self.timer_triggers),
        ];

        for (cat, triggers) in categories {
            for kw in triggers {
                let kw_trimmed = kw.trim();
                if kw_trimmed.is_empty() {
                    continue;
                }
                let kw_len = kw_trimmed.len();

                // Case 1: line trimmed exactly equals kw
                if trimmed.eq_ignore_ascii_case(kw_trimmed) {
                    let matched_kw = &line[leading_spaces..leading_spaces + kw_len];
                    return Some((cat, matched_kw, ""));
                }

                // Case 2: line starts with kw followed by space, colon, or punctuation
                if self.allow_arguments_after_keyword
                    && trimmed.len() > kw_len
                    && trimmed[..kw_len].eq_ignore_ascii_case(kw_trimmed)
                {
                    let next_char = trimmed[kw_len..].chars().next().unwrap();
                    if next_char.is_whitespace() || next_char == ':' {
                        let matched_kw = &line[leading_spaces..leading_spaces + kw_len];
                        let remainder = trimmed[kw_len..].trim_start();
                        let remainder = if let Some(stripped) = remainder.strip_prefix(':') {
                            stripped.trim_start()
                        } else {
                            remainder
                        };
                        return Some((cat, matched_kw, remainder));
                    }
                }
            }
        }

        None
    }

    /// Hydrates `TriggerConfig` from persisted SQLite settings map matching SPEC §5 keys.
    pub fn from_settings(map: &serde_json::Map<String, serde_json::Value>) -> Self {
        let defaults = Self::default();

        let disable_all_keywords = map
            .get("disableAllKeywords")
            .and_then(|v| v.as_bool())
            .unwrap_or(defaults.disable_all_keywords);

        let allow_arguments_after_keyword = map
            .get("allowArgumentsAfterKeyword")
            .and_then(|v| v.as_bool())
            .unwrap_or(defaults.allow_arguments_after_keyword);

        let sum_triggers = parse_triggers(map, "sumTriggers", "sumMainTrigger", &defaults.sum_triggers);
        let avg_triggers = parse_triggers(map, "avgTriggers", "avgMainTrigger", &defaults.avg_triggers);
        let math_triggers = parse_triggers(map, "mathTriggers", "mathMainTrigger", &defaults.math_triggers);
        let count_triggers = parse_triggers(map, "countTriggers", "countMainTrigger", &defaults.count_triggers);
        let list_triggers = parse_triggers(map, "listTriggers", "listMainTrigger", &defaults.list_triggers);
        let check_triggers = parse_triggers(map, "checkTriggers", "checkMainTrigger", &defaults.check_triggers);
        let code_triggers = parse_triggers(map, "codeTriggers", "codeMainTrigger", &defaults.code_triggers);
        let paste_triggers = parse_triggers(map, "pasteTriggers", "pasteMainTrigger", &defaults.paste_triggers);
        let timer_triggers = parse_triggers(map, "timerTriggers", "timerMainTrigger", &defaults.timer_triggers);

        let custom_keyword_label = map
            .get("customKeywordLabel")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let custom_keyword_description = map
            .get("customKeywordDescription")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Self {
            disable_all_keywords,
            allow_arguments_after_keyword,
            sum_triggers,
            avg_triggers,
            math_triggers,
            count_triggers,
            list_triggers,
            check_triggers,
            code_triggers,
            paste_triggers,
            timer_triggers,
            custom_keyword_label,
            custom_keyword_description,
        }
    }

    /// Serializes `TriggerConfig` to settings key-value entries according to SPEC §5.
    pub fn to_settings(&self) -> serde_json::Map<String, serde_json::Value> {
        let mut map = serde_json::Map::new();

        map.insert(
            "disableAllKeywords".into(),
            serde_json::Value::Bool(self.disable_all_keywords),
        );
        map.insert(
            "allowArgumentsAfterKeyword".into(),
            serde_json::Value::Bool(self.allow_arguments_after_keyword),
        );

        let cat_entries = [
            ("sumTriggers", "sumMainTrigger", &self.sum_triggers),
            ("avgTriggers", "avgMainTrigger", &self.avg_triggers),
            ("mathTriggers", "mathMainTrigger", &self.math_triggers),
            ("countTriggers", "countMainTrigger", &self.count_triggers),
            ("listTriggers", "listMainTrigger", &self.list_triggers),
            ("checkTriggers", "checkMainTrigger", &self.check_triggers),
            ("codeTriggers", "codeMainTrigger", &self.code_triggers),
            ("pasteTriggers", "pasteMainTrigger", &self.paste_triggers),
            ("timerTriggers", "timerMainTrigger", &self.timer_triggers),
        ];

        for (arr_key, main_key, list) in cat_entries {
            let arr_val = serde_json::Value::Array(
                list.iter()
                    .map(|s| serde_json::Value::String(s.clone()))
                    .collect(),
            );
            map.insert(arr_key.into(), arr_val);
            let main_val = list.first().map(|s| s.as_str()).unwrap_or("");
            map.insert(main_key.into(), serde_json::Value::String(main_val.to_string()));
        }

        if let Some(ref l) = self.custom_keyword_label {
            map.insert(
                "customKeywordLabel".into(),
                serde_json::Value::String(l.clone()),
            );
        }
        if let Some(ref d) = self.custom_keyword_description {
            map.insert(
                "customKeywordDescription".into(),
                serde_json::Value::String(d.clone()),
            );
        }

        map
    }
}

fn parse_triggers(
    map: &serde_json::Map<String, serde_json::Value>,
    array_key: &str,
    main_key: &str,
    defaults: &[String],
) -> Vec<String> {
    if let Some(val) = map.get(array_key) {
        match val {
            serde_json::Value::Array(arr) => {
                let items: Vec<String> = arr
                    .iter()
                    .filter_map(|v| v.as_str())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                if !items.is_empty() {
                    return items;
                }
            }
            serde_json::Value::String(s) => {
                let items: Vec<String> = s
                    .split(',')
                    .map(|item| item.trim().to_string())
                    .filter(|item| !item.is_empty())
                    .collect();
                if !items.is_empty() {
                    return items;
                }
            }
            _ => {}
        }
    }

    if let Some(val) = map.get(main_key) {
        if let Some(s) = val.as_str() {
            let trimmed = s.trim().to_string();
            if !trimmed.is_empty() {
                let mut v = vec![trimmed];
                for d in defaults {
                    if !v.contains(d) {
                        v.push(d.clone());
                    }
                }
                return v;
            }
        }
    }

    defaults.to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_triggers() {
        let cfg = TriggerConfig::default();
        assert!(!cfg.disable_all_keywords);
        assert!(cfg.allow_arguments_after_keyword);

        assert_eq!(
            cfg.match_trigger("sum"),
            Some((TriggerCategory::Sum, "sum", ""))
        );
        assert_eq!(
            cfg.match_trigger("total"),
            Some((TriggerCategory::Sum, "total", ""))
        );
        assert_eq!(
            cfg.match_trigger("ADD"),
            Some((TriggerCategory::Sum, "ADD", ""))
        );
        assert_eq!(
            cfg.match_trigger("avg"),
            Some((TriggerCategory::Avg, "avg", ""))
        );
        assert_eq!(
            cfg.match_trigger("average"),
            Some((TriggerCategory::Avg, "average", ""))
        );
        assert_eq!(
            cfg.match_trigger("count"),
            Some((TriggerCategory::Count, "count", ""))
        );
        assert_eq!(
            cfg.match_trigger("math"),
            Some((TriggerCategory::Math, "math", ""))
        );
        assert_eq!(
            cfg.match_trigger("convert"),
            Some((TriggerCategory::Math, "convert", ""))
        );
        assert_eq!(
            cfg.match_trigger("list"),
            Some((TriggerCategory::List, "list", ""))
        );
        assert_eq!(
            cfg.match_trigger("check"),
            Some((TriggerCategory::Check, "check", ""))
        );
        assert_eq!(
            cfg.match_trigger("todo"),
            Some((TriggerCategory::Check, "todo", ""))
        );
        assert_eq!(
            cfg.match_trigger("code"),
            Some((TriggerCategory::Code, "code", ""))
        );
        assert_eq!(
            cfg.match_trigger("paste"),
            Some((TriggerCategory::Paste, "paste", ""))
        );
        assert_eq!(
            cfg.match_trigger("timer"),
            Some((TriggerCategory::Timer, "timer", ""))
        );
    }

    #[test]
    fn test_arguments_after_keyword() {
        let cfg = TriggerConfig::default();

        assert_eq!(
            cfg.match_trigger("math 12 * 4"),
            Some((TriggerCategory::Math, "math", "12 * 4"))
        );
        assert_eq!(
            cfg.match_trigger("math: 12 * 4"),
            Some((TriggerCategory::Math, "math", "12 * 4"))
        );
        assert_eq!(
            cfg.match_trigger("code: rust"),
            Some((TriggerCategory::Code, "code", "rust"))
        );
        assert_eq!(
            cfg.match_trigger("timer 5m"),
            Some((TriggerCategory::Timer, "timer", "5m"))
        );
        assert_eq!(
            cfg.match_trigger("count items"),
            Some((TriggerCategory::Count, "count", "items"))
        );
        // Non-boundary words should NOT match
        assert_eq!(cfg.match_trigger("summary"), None);
        assert_eq!(cfg.match_trigger("addition"), None);
    }

    #[test]
    fn test_allow_arguments_disabled() {
        let mut cfg = TriggerConfig::default();
        cfg.allow_arguments_after_keyword = false;

        assert_eq!(
            cfg.match_trigger("sum"),
            Some((TriggerCategory::Sum, "sum", ""))
        );
        assert_eq!(cfg.match_trigger("sum 123"), None);
        assert_eq!(cfg.match_trigger("math 12 * 4"), None);
    }

    #[test]
    fn test_disable_all_keywords() {
        let mut cfg = TriggerConfig::default();
        cfg.disable_all_keywords = true;

        assert_eq!(cfg.match_trigger("sum"), None);
        assert_eq!(cfg.match_trigger("total"), None);
        assert_eq!(cfg.match_trigger("math"), None);
        assert_eq!(cfg.match_trigger("timer"), None);
    }

    #[test]
    fn test_custom_aliases_and_serialization() {
        let mut cfg = TriggerConfig::default();
        cfg.sum_triggers.push("tally".into());
        cfg.custom_keyword_label = Some("Tally".into());

        assert_eq!(
            cfg.match_trigger("tally"),
            Some((TriggerCategory::Sum, "tally", ""))
        );

        let settings = cfg.to_settings();
        let restored = TriggerConfig::from_settings(&settings);

        assert_eq!(restored.sum_triggers, cfg.sum_triggers);
        assert_eq!(restored.custom_keyword_label, Some("Tally".into()));
        assert_eq!(
            restored.match_trigger("tally"),
            Some((TriggerCategory::Sum, "tally", ""))
        );
    }
}
