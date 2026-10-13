//! Physical unit conversion engine covering 5 dimensions:
//! Distance, Area, Mass/Weight, Volume, and Temperature.
//!
//! Includes Antinote regional aliases:
//! - Area: `cuerda`/`cuerdas`, `dunam`/`dunams`, `bigha`/`bighas`
//! - Mass: `stones`/`stone`/`st`, `long ton`/`imperial ton`/`uk ton`
//! - Volume: UK vs US variants (`ukTeaspoon`, `ukTablespoon`, `uk pint`, `imperial gallon`, `ukFluidOunce`, etc.)
//! - Temperature: Affine conversions between Celsius, Fahrenheit, and Kelvin.

/// Physical dimensions supported by the conversion engine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dimension {
    Distance,
    Area,
    Mass,
    Volume,
    Temperature,
}

/// Information about a recognized physical unit.
#[derive(Debug, Clone)]
pub struct UnitInfo {
    pub dimension: Dimension,
    /// Multiplicative factor relative to canonical base unit for linear dimensions.
    pub factor: f64,
    /// Canonical display abbreviation or name.
    pub canonical_display: &'static str,
}

/// Temperature scale identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TempScale {
    Celsius,
    Fahrenheit,
    Kelvin,
}

/// Resolve a unit name or alias into its `Dimension`, `UnitInfo`, and optional `TempScale`.
pub fn resolve_unit(name: &str) -> Option<UnitInfo> {
    let norm = normalize_unit_str(name);
    if norm.is_empty() {
        return None;
    }

    // 1. Temperature scales
    if let Some(scale) = resolve_temp_scale(&norm) {
        let (canonical, _) = match scale {
            TempScale::Celsius => ("°C", "celsius"),
            TempScale::Fahrenheit => ("°F", "fahrenheit"),
            TempScale::Kelvin => ("K", "kelvin"),
        };
        return Some(UnitInfo {
            dimension: Dimension::Temperature,
            factor: 1.0,
            canonical_display: canonical,
        });
    }

    // 2. Linear dimensions: Distance, Area, Mass, Volume
    resolve_linear_unit(&norm)
}

fn resolve_temp_scale(norm: &str) -> Option<TempScale> {
    match norm {
        "celsius" | "deg c" | "degc" | "degrees c" | "degrees celsius" | "c" | "centigrade" => {
            Some(TempScale::Celsius)
        }
        "fahrenheit" | "deg f" | "degf" | "degrees f" | "degrees fahrenheit" | "f" => {
            Some(TempScale::Fahrenheit)
        }
        "kelvin" | "deg k" | "degk" | "degrees k" | "degrees kelvin" | "k" => {
            Some(TempScale::Kelvin)
        }
        _ => None,
    }
}

fn resolve_linear_unit(norm: &str) -> Option<UnitInfo> {
    // --- DISTANCE (base: meter 'm') ---
    match norm {
        "mm" | "millimeter" | "millimeters" | "millimetre" | "millimetres" => Some(UnitInfo {
            dimension: Dimension::Distance,
            factor: 0.001,
            canonical_display: "mm",
        }),
        "cm" | "centimeter" | "centimeters" | "centimetre" | "centimetres" => Some(UnitInfo {
            dimension: Dimension::Distance,
            factor: 0.01,
            canonical_display: "cm",
        }),
        "m" | "meter" | "meters" | "metre" | "metres" => Some(UnitInfo {
            dimension: Dimension::Distance,
            factor: 1.0,
            canonical_display: "m",
        }),
        "km" | "kilometer" | "kilometers" | "kilometre" | "kilometres" => Some(UnitInfo {
            dimension: Dimension::Distance,
            factor: 1000.0,
            canonical_display: "km",
        }),
        "in" | "inch" | "inches" | "\"" => Some(UnitInfo {
            dimension: Dimension::Distance,
            factor: 0.0254,
            canonical_display: "in",
        }),
        "ft" | "foot" | "feet" | "'" => Some(UnitInfo {
            dimension: Dimension::Distance,
            factor: 0.3048,
            canonical_display: "ft",
        }),
        "yd" | "yard" | "yards" => Some(UnitInfo {
            dimension: Dimension::Distance,
            factor: 0.9144,
            canonical_display: "yd",
        }),
        "mi" | "mile" | "miles" => Some(UnitInfo {
            dimension: Dimension::Distance,
            factor: 1609.344,
            canonical_display: "mi",
        }),

        // --- AREA (base: square meter 'sq m') ---
        "sq mm" | "sqmm" | "square millimeter" | "square millimeters" | "square millimetre"
        | "square millimetres" | "mm2" => Some(UnitInfo {
            dimension: Dimension::Area,
            factor: 1e-6,
            canonical_display: "sq mm",
        }),
        "sq cm" | "sqcm" | "square centimeter" | "square centimeters" | "square centimetre"
        | "square centimetres" | "cm2" => Some(UnitInfo {
            dimension: Dimension::Area,
            factor: 1e-4,
            canonical_display: "sq cm",
        }),
        "sq m" | "sqm" | "square meter" | "square meters" | "square metre" | "square metres"
        | "m2" => Some(UnitInfo {
            dimension: Dimension::Area,
            factor: 1.0,
            canonical_display: "sq m",
        }),
        "sq ft" | "sqft" | "square foot" | "square feet" | "ft2" => Some(UnitInfo {
            dimension: Dimension::Area,
            factor: 0.09290304,
            canonical_display: "sq ft",
        }),
        "sq yd" | "sqyd" | "square yard" | "square yards" | "yd2" => Some(UnitInfo {
            dimension: Dimension::Area,
            factor: 0.83612736,
            canonical_display: "sq yd",
        }),
        "sq km" | "sqkm" | "square kilometer" | "square kilometers" | "square kilometre"
        | "square kilometres" | "km2" => Some(UnitInfo {
            dimension: Dimension::Area,
            factor: 1e6,
            canonical_display: "sq km",
        }),
        "acre" | "acres" => Some(UnitInfo {
            dimension: Dimension::Area,
            factor: 4046.8564224,
            canonical_display: "acres",
        }),
        "hectare" | "hectares" | "ha" => Some(UnitInfo {
            dimension: Dimension::Area,
            factor: 10000.0,
            canonical_display: "hectares",
        }),
        // Regional area units from Antinote:
        "cuerda" | "cuerdas" => Some(UnitInfo {
            dimension: Dimension::Area,
            factor: 3930.395625,
            canonical_display: "cuerdas",
        }),
        "dunam" | "dunams" => Some(UnitInfo {
            dimension: Dimension::Area,
            factor: 1000.0,
            canonical_display: "dunams",
        }),
        "bigha" | "bighas" => Some(UnitInfo {
            dimension: Dimension::Area,
            factor: 2500.0,
            canonical_display: "bighas",
        }),

        // --- MASS / WEIGHT (base: kilogram 'kg') ---
        "mg" | "milligram" | "milligrams" | "milligramme" | "milligrammes" => Some(UnitInfo {
            dimension: Dimension::Mass,
            factor: 1e-6,
            canonical_display: "mg",
        }),
        "g" | "gram" | "grams" | "gramme" | "grammes" => Some(UnitInfo {
            dimension: Dimension::Mass,
            factor: 0.001,
            canonical_display: "g",
        }),
        "kg" | "kilogram" | "kilograms" | "kilogramme" | "kilogrammes" | "kilo" | "kilos" => {
            Some(UnitInfo {
                dimension: Dimension::Mass,
                factor: 1.0,
                canonical_display: "kg",
            })
        }
        "tonne" | "tonnes" | "metric ton" | "metric tons" | "t" => Some(UnitInfo {
            dimension: Dimension::Mass,
            factor: 1000.0,
            canonical_display: "tonnes",
        }),
        "lb" | "lbs" | "pound" | "pounds" => Some(UnitInfo {
            dimension: Dimension::Mass,
            factor: 0.45359237,
            canonical_display: "lbs",
        }),
        "oz" | "ounce" | "ounces" => Some(UnitInfo {
            dimension: Dimension::Mass,
            factor: 0.028349523125,
            canonical_display: "oz",
        }),
        // Regional mass units from Antinote:
        "st" | "stone" | "stones" => Some(UnitInfo {
            dimension: Dimension::Mass,
            factor: 6.35029318, // 14 lb
            canonical_display: "stones",
        }),
        "long ton" | "long tons" | "imperial ton" | "imperial tons" | "uk ton" | "uk tons" => {
            Some(UnitInfo {
                dimension: Dimension::Mass,
                factor: 1016.0469088, // 2240 lb
                canonical_display: "long tons",
            })
        }

        // --- VOLUME (base: liter 'l') ---
        "ml" | "milliliter" | "milliliters" | "millilitre" | "millilitres" => Some(UnitInfo {
            dimension: Dimension::Volume,
            factor: 0.001,
            canonical_display: "ml",
        }),
        "l" | "liter" | "liters" | "litre" | "litres" => Some(UnitInfo {
            dimension: Dimension::Volume,
            factor: 1.0,
            canonical_display: "l",
        }),
        "cu mm" | "cubic millimeter" | "cubic millimeters" | "cubic millimetre"
        | "cubic millimetres" | "mm3" => Some(UnitInfo {
            dimension: Dimension::Volume,
            factor: 1e-6,
            canonical_display: "cu mm",
        }),
        "cu m" | "cubic meter" | "cubic meters" | "cubic metre" | "cubic metres" | "m3" => {
            Some(UnitInfo {
                dimension: Dimension::Volume,
                factor: 1000.0,
                canonical_display: "cu m",
            })
        }
        "cu ft" | "cubic foot" | "cubic feet" | "ft3" => Some(UnitInfo {
            dimension: Dimension::Volume,
            factor: 28.316846592,
            canonical_display: "cu ft",
        }),
        // US vs UK regional volume variants from Antinote:
        "tsp" | "tsps" | "teaspoon" | "teaspoons" | "us teaspoon" | "us teaspoons" | "us tsp" => {
            Some(UnitInfo {
                dimension: Dimension::Volume,
                factor: 0.00492892159375,
                canonical_display: "tsp",
            })
        }
        "ukteaspoon" | "uk teaspoon" | "uk teaspoons" | "uk tsp" | "uk tsps"
        | "imperial teaspoon" | "imperial teaspoons" => Some(UnitInfo {
            dimension: Dimension::Volume,
            factor: 0.00591939,
            canonical_display: "ukTeaspoon",
        }),
        "tbsp" | "tbsps" | "tablespoon" | "tablespoons" | "us tablespoon" | "us tablespoons"
        | "us tbsp" => Some(UnitInfo {
            dimension: Dimension::Volume,
            factor: 0.01478676478125,
            canonical_display: "tbsp",
        }),
        "uktablespoon" | "uk tablespoon" | "uk tablespoons" | "uk tbsp" | "uk tbsps"
        | "imperial tablespoon" | "imperial tablespoons" => Some(UnitInfo {
            dimension: Dimension::Volume,
            factor: 0.0177582,
            canonical_display: "ukTablespoon",
        }),
        "pt" | "pint" | "pints" | "us pint" | "us pints" | "us pt" => Some(UnitInfo {
            dimension: Dimension::Volume,
            factor: 0.473176473,
            canonical_display: "pt",
        }),
        "uk pint" | "uk pints" | "ukpint" | "ukpints" | "imperial pint" | "imperial pints"
        | "uk pt" => Some(UnitInfo {
            dimension: Dimension::Volume,
            factor: 0.56826125,
            canonical_display: "uk pint",
        }),
        "gal" | "gallon" | "gallons" | "us gallon" | "us gallons" | "us gal" => Some(UnitInfo {
            dimension: Dimension::Volume,
            factor: 3.785411784,
            canonical_display: "gal",
        }),
        "imperial gallon" | "imperial gallons" | "uk gallon" | "uk gallons" | "uk gal" => {
            Some(UnitInfo {
                dimension: Dimension::Volume,
                factor: 4.54609,
                canonical_display: "imperial gallon",
            })
        }
        "fl oz" | "fl oz." | "fluid ounce" | "fluid ounces" | "us fl oz" | "us fluid ounce"
        | "us fluid ounces" => Some(UnitInfo {
            dimension: Dimension::Volume,
            factor: 0.0295735295625,
            canonical_display: "fl oz",
        }),
        "ukfluidounce" | "uk fluid ounce" | "uk fluid ounces" | "uk fl oz" | "uk fl oz."
        | "imperial fluid ounce" | "imperial fluid ounces" | "imperial fl oz" => Some(UnitInfo {
            dimension: Dimension::Volume,
            factor: 0.0284130625,
            canonical_display: "ukFluidOunce",
        }),

        _ => None,
    }
}

/// Normalize unit strings: lowercases, removes degree symbols, periods, and collapses whitespace.
pub fn normalize_unit_str(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.trim().chars() {
        if c == '°' || c == '.' {
            continue;
        } else if c == '²' {
            out.push('2');
        } else if c == '³' {
            out.push('3');
        } else {
            out.push(c.to_ascii_lowercase());
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Determine the preferred target unit display string.
/// Preserves user styling where appropriate (e.g. `lbs` vs `lb`, `miles` vs `mi`),
/// while ensuring temperature always formats with standard symbols (`°C`, `°F`, `K`).
fn determine_target_display(to_unit_raw: &str, info: &UnitInfo) -> String {
    let norm = normalize_unit_str(to_unit_raw);
    match info.dimension {
        Dimension::Temperature => info.canonical_display.to_string(),
        Dimension::Distance => {
            if norm == "miles" {
                "miles".to_string()
            } else if norm == "feet" {
                "feet".to_string()
            } else {
                info.canonical_display.to_string()
            }
        }
        Dimension::Mass => {
            if norm == "lb" {
                "lb".to_string()
            } else if norm == "lbs" {
                "lbs".to_string()
            } else {
                info.canonical_display.to_string()
            }
        }
        Dimension::Area => {
            if norm == "sq m" || norm == "square meters" || norm == "square meter" {
                "sq m".to_string()
            } else {
                info.canonical_display.to_string()
            }
        }
        Dimension::Volume => {
            if norm == "litres" || norm == "liters" || norm == "l" {
                "l".to_string()
            } else {
                info.canonical_display.to_string()
            }
        }
    }
}

/// Convert a numeric quantity `val` from `from_unit` to `to_unit`.
///
/// Returns `Some((converted_value, target_unit_display))` if both units are valid and
/// belong to the same physical dimension. Returns `None` on dimension mismatch or unrecognized unit.
pub fn convert(val: f64, from_unit: &str, to_unit: &str) -> Option<(f64, String)> {
    let from_info = resolve_unit(from_unit)?;
    let to_info = resolve_unit(to_unit)?;

    if from_info.dimension != to_info.dimension {
        return None;
    }

    let target_display = determine_target_display(to_unit, &to_info);

    match from_info.dimension {
        Dimension::Temperature => {
            let from_scale = resolve_temp_scale(&normalize_unit_str(from_unit))?;
            let to_scale = resolve_temp_scale(&normalize_unit_str(to_unit))?;

            // Convert from source to Celsius
            let celsius = match from_scale {
                TempScale::Celsius => val,
                TempScale::Fahrenheit => (val - 32.0) * 5.0 / 9.0,
                TempScale::Kelvin => val - 273.15,
            };

            // Convert from Celsius to destination
            let converted = match to_scale {
                TempScale::Celsius => celsius,
                TempScale::Fahrenheit => celsius * 9.0 / 5.0 + 32.0,
                TempScale::Kelvin => celsius + 273.15,
            };

            Some((converted, target_display))
        }
        _ => {
            let base_val = val * from_info.factor;
            let converted = base_val / to_info.factor;
            Some((converted, target_display))
        }
    }
}

/// Format a converted floating-point number cleanly:
/// - Exact integers print without decimal fraction (e.g. `1`, `196`, `1000`)
/// - Floats format up to 4-6 decimal places, trimming trailing zeros.
pub fn format_value(val: f64) -> String {
    if (val.round() - val).abs() < 1e-9 {
        return format!("{}", val.round() as i64);
    }
    let abs_val = val.abs();
    let s = if abs_val >= 1000.0 {
        format!("{:.4}", val)
    } else if abs_val >= 0.001 {
        format!("{:.4}", val)
    } else {
        format!("{:.6}", val)
    };
    let trimmed = s.trim_end_matches('0').trim_end_matches('.');
    trimmed.to_string()
}

/// Natural-language conversion parser.
/// Matches lines of the structure: `VALUE [FROM_UNIT] (in|to|into|as|=) [TO_UNIT]`.
/// Returns `Some((formatted_result, explanation_string))`.
pub fn parse_and_convert(line: &str) -> Option<(String, String)> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    // Normalize punctuation
    let s = trimmed.trim_end_matches(['?', '=', ';']).trim();
    if s.is_empty() {
        return None;
    }

    const SEPARATORS: &[&str] = &[" in ", " to ", " into ", " as ", " = ", "="];

    for &sep in SEPARATORS {
        let mut start_idx = 0;
        while let Some(rel) = s[start_idx..].find(sep) {
            let sep_pos = start_idx + rel;
            let lhs = s[..sep_pos].trim();
            let rhs = s[sep_pos + sep.len()..].trim();
            start_idx = sep_pos + sep.len();

            if lhs.is_empty() || rhs.is_empty() {
                continue;
            }

            // RHS must resolve to a valid unit
            let _to_info = match resolve_unit(rhs) {
                Some(info) => info,
                None => continue,
            };

            // LHS must parse into leading numeric value and from_unit
            let (val, from_unit_raw) = match parse_leading_val_and_unit(lhs) {
                Some(parsed) => parsed,
                None => continue,
            };

            if let Some((converted, target_display)) = convert(val, from_unit_raw, rhs) {
                let formatted_val = format_value(converted);
                let formatted_result = format!("{formatted_val} {target_display}");
                let explanation = format!(
                    "converted {} {} to {}",
                    format_value(val),
                    from_unit_raw,
                    rhs
                );
                return Some((formatted_result, explanation));
            }
        }
    }

    None
}

/// Split LHS into `(val, unit_str)`.
/// Handles leading positive or negative numbers, floats, commas, and suffixes like `"` or `'`.
fn parse_leading_val_and_unit(lhs: &str) -> Option<(f64, &str)> {
    let t = lhs.trim();
    if t.is_empty() {
        return None;
    }

    let mut end_num = 0;
    let chars: Vec<(usize, char)> = t.char_indices().collect();
    let len = chars.len();
    let mut i = 0;

    // Check optional leading +/-
    if i < len && (chars[i].1 == '+' || chars[i].1 == '-') {
        i += 1;
    }

    let mut seen_digit = false;
    let mut seen_dot = false;

    while i < len {
        let c = chars[i].1;
        if c.is_ascii_digit() {
            seen_digit = true;
            end_num = chars[i].0 + c.len_utf8();
            i += 1;
        } else if c == '.' && !seen_dot {
            seen_dot = true;
            end_num = chars[i].0 + c.len_utf8();
            i += 1;
        } else if c == ',' || c == '_' {
            // Thousands separator
            end_num = chars[i].0 + c.len_utf8();
            i += 1;
        } else {
            break;
        }
    }

    if !seen_digit {
        return None;
    }

    let num_str = &t[..end_num].replace([',', '_'], "");
    let val: f64 = num_str.parse().ok()?;
    let unit_str = t[end_num..].trim();

    if unit_str.is_empty() {
        return None;
    }

    Some((val, unit_str))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_distance_conversions() {
        let (v, u) = convert(1000.0, "mm", "m").unwrap();
        assert!((v - 1.0).abs() < 1e-6);
        assert_eq!(u, "m");

        let (v, _) = convert(5.0, "km", "miles").unwrap();
        assert!((v - 3.106855).abs() < 1e-4);

        let (v, u) = convert(12.0, "in", "cm").unwrap();
        assert!((v - 30.48).abs() < 1e-6);
        assert_eq!(u, "cm");

        let (v, u) = convert(6.0, "ft", "m").unwrap();
        assert!((v - 1.8288).abs() < 1e-6);
        assert_eq!(u, "m");

        let (v, _) = convert(3.0, "yd", "feet").unwrap();
        assert!((v - 9.0).abs() < 1e-6);
    }

    #[test]
    fn test_area_conversions_and_regional() {
        let (v, u) = convert(100.0, "sq ft", "sq m").unwrap();
        assert!((v - 9.2903).abs() < 1e-3);
        assert_eq!(u, "sq m");

        // Regional area: cuerdas (3930.395625 m²) -> acres (4046.8564224 m²)
        let (v, _) = convert(5.0, "cuerdas", "acres").unwrap();
        assert!((v - 4.8561).abs() < 1e-3);

        // Regional area: dunam (1000 m²) -> sq m
        let (v, _) = convert(1.0, "dunam", "sq m").unwrap();
        assert!((v - 1000.0).abs() < 1e-6);

        // Regional area: bigha (2500 m²) -> sq m
        let (v, _) = convert(1.0, "bigha", "sq m").unwrap();
        assert!((v - 2500.0).abs() < 1e-6);

        // Hectare (10000 m²) -> acres
        let (v, _) = convert(1.0, "hectare", "acres").unwrap();
        assert!((v - 2.471).abs() < 1e-3);
    }

    #[test]
    fn test_mass_conversions_and_regional() {
        let (v, u) = convert(1000.0, "g", "kg").unwrap();
        assert!((v - 1.0).abs() < 1e-6);
        assert_eq!(u, "kg");

        // Regional mass: stones (14 lb) -> lbs
        let (v, _) = convert(14.0, "stones", "lbs").unwrap();
        assert!((v - 196.0).abs() < 1e-6);

        let (v, _) = convert(1.0, "stone", "kg").unwrap();
        assert!((v - 6.3503).abs() < 1e-3);

        // Regional mass: long ton (2240 lb) -> lbs
        let (v, _) = convert(1.0, "long ton", "lbs").unwrap();
        assert!((v - 2240.0).abs() < 1e-6);

        let (v, _) = convert(1.0, "imperial ton", "kg").unwrap();
        assert!((v - 1016.05).abs() < 1e-1);

        let (v, _) = convert(16.0, "oz", "lb").unwrap();
        assert!((v - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_volume_conversions_and_uk_us_variants() {
        let (v, u) = convert(1000.0, "ml", "l").unwrap();
        assert!((v - 1.0).abs() < 1e-6);
        assert_eq!(u, "l");

        let (v, _) = convert(1.0, "us gallon", "litres").unwrap();
        assert!((v - 3.7854).abs() < 1e-3);

        let (v, _) = convert(1.0, "imperial gallon", "litres").unwrap();
        assert!((v - 4.5461).abs() < 1e-3);

        let (v, _) = convert(1.0, "us pint", "ml").unwrap();
        assert!((v - 473.18).abs() < 1e-1);

        let (v, _) = convert(1.0, "uk pint", "ml").unwrap();
        assert!((v - 568.26).abs() < 1e-1);

        let (v, _) = convert(1.0, "tbsp", "tsp").unwrap();
        assert!((v - 3.0).abs() < 1e-6);

        let (v, _) = convert(1.0, "ukTeaspoon", "ml").unwrap();
        assert!((v - 5.92).abs() < 1e-2);

        let (v, _) = convert(1.0, "ukTablespoon", "ml").unwrap();
        assert!((v - 17.76).abs() < 1e-2);

        let (v, _) = convert(1.0, "ukFluidOunce", "ml").unwrap();
        assert!((v - 28.41).abs() < 1e-2);

        let (v, _) = convert(1.0, "fl oz", "ml").unwrap();
        assert!((v - 29.57).abs() < 1e-2);
    }

    #[test]
    fn test_temperature_affine_conversions() {
        let (v, u) = convert(32.0, "fahrenheit", "celsius").unwrap();
        assert!((v - 0.0).abs() < 1e-6);
        assert_eq!(u, "°C");

        let (v, _) = convert(212.0, "fahrenheit", "celsius").unwrap();
        assert!((v - 100.0).abs() < 1e-6);

        let (v, u) = convert(0.0, "celsius", "fahrenheit").unwrap();
        assert!((v - 32.0).abs() < 1e-6);
        assert_eq!(u, "°F");

        let (v, _) = convert(100.0, "celsius", "fahrenheit").unwrap();
        assert!((v - 212.0).abs() < 1e-6);

        let (v, u) = convert(0.0, "celsius", "kelvin").unwrap();
        assert!((v - 273.15).abs() < 1e-6);
        assert_eq!(u, "K");

        let (v, _) = convert(-40.0, "fahrenheit", "celsius").unwrap();
        assert!((v - (-40.0)).abs() < 1e-6);
    }

    #[test]
    fn test_mismatched_dimensions_fail() {
        assert!(convert(10.0, "m", "kg").is_none());
        assert!(convert(10.0, "celsius", "l").is_none());
        assert!(convert(5.0, "acre", "ft").is_none());
    }

    #[test]
    fn test_parse_and_convert_natural_language() {
        let (res, why) = parse_and_convert("5 cuerdas in acres").unwrap();
        assert!(res.contains("4.856"));
        assert!(res.contains("acres"));
        assert!(why.contains("cuerdas"));

        let (res, _) = parse_and_convert("14 st in kg").unwrap();
        assert!(res.contains("88.9"));
        assert!(res.contains("kg"));

        let (res, _) = parse_and_convert("2 uk pint in ml").unwrap();
        assert!(res.contains("1136.5"));
        assert!(res.contains("ml"));

        let (res, _) = parse_and_convert("32 fahrenheit in celsius =").unwrap();
        assert_eq!(res, "0 °C");

        let (res, _) = parse_and_convert("-40 fahrenheit to celsius?").unwrap();
        assert_eq!(res, "-40 °C");
    }
}
