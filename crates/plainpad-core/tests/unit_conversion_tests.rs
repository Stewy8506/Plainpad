//! Integration tests for 5-Domain Physical Unit Conversion Engine (REQ-10):
//! Distance, Area (including regional cuerda, dunam, bigha),
//! Mass (including stones, long ton), Volume (UK vs US variants),
//! and Temperature (affine conversions).

use plainpad_core::intent;
use plainpad_core::mathwrap::{self, Vars};
use plainpad_core::units::{self, convert, parse_and_convert};

#[test]
fn test_distance_dimension_conversions() {
    let vars = Vars::new();

    // 1000 mm in m -> 1 m
    let (v, u) = convert(1000.0, "mm", "m").expect("1000 mm to m");
    assert!((v - 1.0).abs() < 1e-6);
    assert_eq!(u, "m");
    let (res, _) = parse_and_convert("1000 mm in m").expect("parse 1000 mm in m");
    assert_eq!(res, "1 m");
    let m = mathwrap::eval_line("1000 mm in m", &vars).expect("eval 1000 mm in m");
    assert_eq!(m.result, "1 m");

    // 5 km in miles -> ~3.1068 mi
    let (v, _) = convert(5.0, "km", "miles").expect("5 km to miles");
    assert!((v - 3.106856).abs() < 1e-3);
    let (res, _) = parse_and_convert("5 km in miles").expect("parse 5 km in miles");
    assert!(res.contains("3.106") || res.contains("3.107"));
    assert!(res.contains("mi") || res.contains("miles"));

    // 12 in in cm -> 30.48 cm
    let (v, u) = convert(12.0, "in", "cm").expect("12 in to cm");
    assert!((v - 30.48).abs() < 1e-6);
    assert_eq!(u, "cm");
    let (res, _) = parse_and_convert("12 in in cm").expect("parse 12 in in cm");
    assert_eq!(res, "30.48 cm");

    // 6 ft in m -> ~1.8288 m
    let (v, u) = convert(6.0, "ft", "m").expect("6 ft to m");
    assert!((v - 1.8288).abs() < 1e-6);
    assert_eq!(u, "m");
    let (res, _) = parse_and_convert("6 ft in m").expect("parse 6 ft in m");
    assert_eq!(res, "1.8288 m");

    // 3 yd in feet -> 9 ft
    let (v, u) = convert(3.0, "yd", "feet").expect("3 yd to feet");
    assert!((v - 9.0).abs() < 1e-6);
    assert_eq!(u, "ft");
    let (res, _) = parse_and_convert("3 yd in feet").expect("parse 3 yd in feet");
    assert_eq!(res, "9 ft");
}

#[test]
fn test_area_dimension_conversions_and_regional() {
    let vars = Vars::new();

    // 100 sq ft in sq m -> ~9.29 sq m
    let (v, u) = convert(100.0, "sq ft", "sq m").expect("100 sq ft to sq m");
    assert!((v - 9.2903).abs() < 1e-2);
    assert_eq!(u, "sq m");
    let (res, _) = parse_and_convert("100 sq ft in sq m").expect("parse 100 sq ft in sq m");
    assert!(res.contains("9.29"));
    assert!(res.contains("sq m"));

    // 5 cuerdas in acres -> ~4.856 acres (Antinote Puerto Rican cuerda = 3930.395625 m²)
    let (v, u) = convert(5.0, "cuerdas", "acres").expect("5 cuerdas to acres");
    assert!((v - 4.8561).abs() < 1e-3);
    assert_eq!(u, "acres");
    let (res, why) = parse_and_convert("5 cuerdas in acres").expect("parse 5 cuerdas in acres");
    assert!(res.contains("4.856"));
    assert!(res.contains("acres"));
    assert!(why.contains("cuerdas"));
    let m = mathwrap::eval_line("5 cuerdas in acres", &vars).expect("eval 5 cuerdas in acres");
    assert!(m.result.contains("4.856"));

    // 1 dunam in sq m -> 1000 sq m (metric dunam = 1000 m²)
    let (v, u) = convert(1.0, "dunam", "sq m").expect("1 dunam to sq m");
    assert!((v - 1000.0).abs() < 1e-6);
    assert_eq!(u, "sq m");
    let (res, _) = parse_and_convert("1 dunam in sq m").expect("parse 1 dunam in sq m");
    assert_eq!(res, "1000 sq m");

    // 1 bigha in sq m -> 2500 sq m (standard bigha = 2500 m²)
    let (v, u) = convert(1.0, "bigha", "sq m").expect("1 bigha to sq m");
    assert!((v - 2500.0).abs() < 1e-6);
    assert_eq!(u, "sq m");
    let (res, _) = parse_and_convert("1 bigha in sq m").expect("parse 1 bigha in sq m");
    assert_eq!(res, "2500 sq m");

    // 1 hectare in acres -> ~2.471 acres (10,000 m²)
    let (v, u) = convert(1.0, "hectare", "acres").expect("1 hectare to acres");
    assert!((v - 2.47105).abs() < 1e-3);
    assert_eq!(u, "acres");
    let (res, _) = parse_and_convert("1 hectare in acres").expect("parse 1 hectare in acres");
    assert!(res.contains("2.471"));
}

#[test]
fn test_mass_dimension_conversions_and_regional() {
    let vars = Vars::new();

    // 1000 g in kg -> 1 kg
    let (v, u) = convert(1000.0, "g", "kg").expect("1000 g to kg");
    assert!((v - 1.0).abs() < 1e-6);
    assert_eq!(u, "kg");
    let (res, _) = parse_and_convert("1000 g in kg").expect("parse 1000 g in kg");
    assert_eq!(res, "1 kg");

    // 14 stones in lbs -> 196 lbs (Antinote 1 stone = 14 lb)
    let (v, u) = convert(14.0, "stones", "lbs").expect("14 stones to lbs");
    assert!((v - 196.0).abs() < 1e-6);
    assert_eq!(u, "lbs");
    let (res, _) = parse_and_convert("14 stones in lbs").expect("parse 14 stones in lbs");
    assert_eq!(res, "196 lbs");
    let m = mathwrap::eval_line("14 stones in lbs", &vars).expect("eval 14 stones in lbs");
    assert_eq!(m.result, "196 lbs");

    // 1 stone in kg -> ~6.3503 kg
    let (v, u) = convert(1.0, "stone", "kg").expect("1 stone to kg");
    assert!((v - 6.35029).abs() < 1e-3);
    assert_eq!(u, "kg");
    let (res, _) = parse_and_convert("1 stone in kg").expect("parse 1 stone in kg");
    assert!(res.contains("6.3503"));
    assert!(res.contains("kg"));

    // 1 long ton in lbs -> 2240 lbs (Antinote long ton = 2240 lb)
    let (v, u) = convert(1.0, "long ton", "lbs").expect("1 long ton to lbs");
    assert!((v - 2240.0).abs() < 1e-6);
    assert_eq!(u, "lbs");
    let (res, _) = parse_and_convert("1 long ton in lbs").expect("parse 1 long ton in lbs");
    assert_eq!(res, "2240 lbs");

    // 1 imperial ton in kg -> ~1016.05 kg (1016.0469088 kg)
    let (v, u) = convert(1.0, "imperial ton", "kg").expect("1 imperial ton to kg");
    assert!((v - 1016.0469).abs() < 1e-1);
    assert_eq!(u, "kg");
    let (res, _) = parse_and_convert("1 imperial ton in kg").expect("parse 1 imperial ton in kg");
    assert!(res.contains("1016.05") || res.contains("1016.04"));
    assert!(res.contains("kg"));

    // 16 oz in lb -> 1 lb
    let (v, u) = convert(16.0, "oz", "lb").expect("16 oz to lb");
    assert!((v - 1.0).abs() < 1e-6);
    assert_eq!(u, "lb");
    let (res, _) = parse_and_convert("16 oz in lb").expect("parse 16 oz in lb");
    assert_eq!(res, "1 lb");
}

#[test]
fn test_volume_dimension_conversions_and_regional_variants() {
    let vars = Vars::new();

    // 1000 ml in l -> 1 l
    let (v, u) = convert(1000.0, "ml", "l").expect("1000 ml to l");
    assert!((v - 1.0).abs() < 1e-6);
    assert_eq!(u, "l");
    let (res, _) = parse_and_convert("1000 ml in l").expect("parse 1000 ml in l");
    assert_eq!(res, "1 l");

    // 1 us gallon in litres -> ~3.7854 l
    let (v, u) = convert(1.0, "us gallon", "litres").expect("1 us gallon to litres");
    assert!((v - 3.78541).abs() < 1e-3);
    assert_eq!(u, "l");
    let (res, _) = parse_and_convert("1 us gallon in litres").expect("parse 1 us gallon in litres");
    assert!(res.contains("3.7854"));
    assert!(res.contains("l"));

    // 1 imperial gallon in litres -> ~4.5461 l
    let (v, u) = convert(1.0, "imperial gallon", "litres").expect("1 imperial gallon to litres");
    assert!((v - 4.54609).abs() < 1e-3);
    assert_eq!(u, "l");
    let (res, _) = parse_and_convert("1 imperial gallon in litres").expect("parse 1 imperial gallon");
    assert!(res.contains("4.5461"));

    // 1 us pint in ml -> ~473.18 ml
    let (v, u) = convert(1.0, "us pint", "ml").expect("1 us pint to ml");
    assert!((v - 473.176).abs() < 1e-1);
    assert_eq!(u, "ml");
    let (res, _) = parse_and_convert("1 us pint in ml").expect("parse 1 us pint in ml");
    assert!(res.contains("473.1"));

    // 1 uk pint in ml -> ~568.26 ml
    let (v, u) = convert(1.0, "uk pint", "ml").expect("1 uk pint to ml");
    assert!((v - 568.261).abs() < 1e-1);
    assert_eq!(u, "ml");
    let (res, _) = parse_and_convert("1 uk pint in ml").expect("parse 1 uk pint in ml");
    assert!(res.contains("568.2"));

    // 1 tbsp in tsp -> 3 tsp
    let (v, u) = convert(1.0, "tbsp", "tsp").expect("1 tbsp to tsp");
    assert!((v - 3.0).abs() < 1e-6);
    assert_eq!(u, "tsp");
    let (res, _) = parse_and_convert("1 tbsp in tsp").expect("parse 1 tbsp in tsp");
    assert_eq!(res, "3 tsp");

    // 1 ukTeaspoon in ml -> ~5.92 ml
    let (v, u) = convert(1.0, "ukTeaspoon", "ml").expect("1 ukTeaspoon to ml");
    assert!((v - 5.91939).abs() < 1e-2);
    assert_eq!(u, "ml");
    let (res, _) = parse_and_convert("1 ukTeaspoon in ml").expect("parse 1 ukTeaspoon in ml");
    assert!(res.contains("5.92") || res.contains("5.919"));

    // 1 ukTablespoon in ml -> ~17.76 ml
    let (v, u) = convert(1.0, "ukTablespoon", "ml").expect("1 ukTablespoon to ml");
    assert!((v - 17.7582).abs() < 1e-2);
    assert_eq!(u, "ml");
    let (res, _) = parse_and_convert("1 ukTablespoon in ml").expect("parse 1 ukTablespoon in ml");
    assert!(res.contains("17.76") || res.contains("17.758"));

    // 1 ukFluidOunce in ml -> ~28.41 ml
    let (v, u) = convert(1.0, "ukFluidOunce", "ml").expect("1 ukFluidOunce to ml");
    assert!((v - 28.41306).abs() < 1e-2);
    assert_eq!(u, "ml");
    let (res, _) = parse_and_convert("1 ukFluidOunce in ml").expect("parse 1 ukFluidOunce in ml");
    assert!(res.contains("28.41"));

    // 1 fl oz in ml -> ~29.57 ml
    let (v, u) = convert(1.0, "fl oz", "ml").expect("1 fl oz to ml");
    assert!((v - 29.5735).abs() < 1e-2);
    assert_eq!(u, "ml");
    let (res, _) = parse_and_convert("1 fl oz in ml").expect("parse 1 fl oz in ml");
    assert!(res.contains("29.57"));

    let m = mathwrap::eval_line("2 uk pint in ml", &vars).expect("eval 2 uk pint in ml");
    assert!(m.result.contains("1136.5"));
}

#[test]
fn test_temperature_affine_conversions() {
    let vars = Vars::new();

    // 32 fahrenheit in celsius -> 0 °C
    let (v, u) = convert(32.0, "fahrenheit", "celsius").expect("32 f to c");
    assert!((v - 0.0).abs() < 1e-6);
    assert_eq!(u, "°C");
    let (res, _) = parse_and_convert("32 fahrenheit in celsius").expect("parse 32 f in c");
    assert_eq!(res, "0 °C");
    let m = mathwrap::eval_line("32 fahrenheit in celsius", &vars).expect("eval 32 f in c");
    assert_eq!(m.result, "0 °C");

    // 212 fahrenheit in celsius -> 100 °C
    let (v, u) = convert(212.0, "fahrenheit", "celsius").expect("212 f to c");
    assert!((v - 100.0).abs() < 1e-6);
    assert_eq!(u, "°C");
    let (res, _) = parse_and_convert("212 fahrenheit in celsius").expect("parse 212 f in c");
    assert_eq!(res, "100 °C");

    // 0 celsius in fahrenheit -> 32 °F
    let (v, u) = convert(0.0, "celsius", "fahrenheit").expect("0 c to f");
    assert!((v - 32.0).abs() < 1e-6);
    assert_eq!(u, "°F");
    let (res, _) = parse_and_convert("0 celsius in fahrenheit").expect("parse 0 c in f");
    assert_eq!(res, "32 °F");

    // 100 celsius in fahrenheit -> 212 °F
    let (v, u) = convert(100.0, "celsius", "fahrenheit").expect("100 c to f");
    assert!((v - 212.0).abs() < 1e-6);
    assert_eq!(u, "°F");
    let (res, _) = parse_and_convert("100 celsius in fahrenheit").expect("parse 100 c in f");
    assert_eq!(res, "212 °F");

    // 0 celsius in kelvin -> 273.15 K
    let (v, u) = convert(0.0, "celsius", "kelvin").expect("0 c to k");
    assert!((v - 273.15).abs() < 1e-6);
    assert_eq!(u, "K");
    let (res, _) = parse_and_convert("0 celsius in kelvin").expect("parse 0 c in k");
    assert_eq!(res, "273.15 K");

    // -40 fahrenheit in celsius -> -40 °C
    let (v, u) = convert(-40.0, "fahrenheit", "celsius").expect("-40 f to c");
    assert!((v - (-40.0)).abs() < 1e-6);
    assert_eq!(u, "°C");
    let (res, _) = parse_and_convert("-40 fahrenheit in celsius").expect("parse -40 f in c");
    assert_eq!(res, "-40 °C");
}

#[test]
fn test_mismatched_dimensions_and_invalid_inputs_fail_safely() {
    let vars = Vars::new();

    // Mismatched dimension queries fail safely with None
    assert!(convert(10.0, "m", "kg").is_none());
    assert!(convert(10.0, "celsius", "litres").is_none());
    assert!(convert(100.0, "sq ft", "pounds").is_none());
    assert!(parse_and_convert("10 m in kg").is_none());
    assert!(parse_and_convert("100 sq ft in pounds").is_none());

    // Unsupported strings fail safely
    assert!(parse_and_convert("hello world in feet").is_none());
    assert!(parse_and_convert("12345 in").is_none());
    assert!(parse_and_convert("").is_none());

    // Non-conversion math sentences are not corrupted
    assert!(units::resolve_unit("foobar").is_none());
    assert!(mathwrap::eval_line("10 m in kg", &vars).is_none());
}

#[test]
fn test_intent_analysis_annotates_physical_conversions() {
    let text = "5 cuerdas in acres\n14 st in kg\n2 uk pint in ml\n32 fahrenheit in celsius\n1000 mm in m";
    let analysis = intent::analyze(text, false);
    assert_eq!(analysis.annotations.len(), 5);

    for (i, anno) in analysis.annotations.iter().enumerate() {
        assert_eq!(anno.line, i);
        assert_eq!(anno.kind, "conversion");
        assert_eq!(anno.confidence, 0.95);
        assert!(anno.chip.starts_with("Convert ("));
        assert!(!anno.value.is_empty());
        assert!(!anno.why.is_empty());
    }

    assert!(analysis.annotations[0].value.contains("4.856"));
    assert!(analysis.annotations[1].value.contains("88.9"));
    assert!(analysis.annotations[2].value.contains("1136.5"));
    assert_eq!(analysis.annotations[3].value, "0 °C");
    assert_eq!(analysis.annotations[4].value, "1 m");
}
