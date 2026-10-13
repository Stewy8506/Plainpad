---
phase: 3
plan: 2
wave: 2
depends_on:
  - "3.1"
files_modified:
  - crates/plainpad-core/src/units.rs
  - crates/plainpad-core/src/lib.rs
  - crates/plainpad-core/src/mathwrap.rs
  - crates/plainpad-core/src/intent.rs
  - crates/plainpad-core/tests/unit_conversion_tests.rs
autonomous: true
gap_closure: false
must_haves:
  truths:
    - "Bidirectional unit conversion operates across all 5 physical dimensions: Distance, Area, Mass/Weight, Volume, and Temperature"
    - "Antinote regional area units cuerda, dunam, and bigha convert accurately to and from metric and imperial area units"
    - "Regional mass units stones (14 lb) and long tons (2240 lb) convert accurately to and from kg, lb, and tonnes"
    - "UK and US volume variations (ukTeaspoon vs tsp, ukTablespoon vs tbsp, uk pint vs pint, imperial gallon vs us gallon, ukFluidOunce vs fl oz) convert accurately"
    - "Affine temperature conversions between Celsius, Fahrenheit, and Kelvin calculate exact values"
  artifacts:
    - "crates/plainpad-core/src/units.rs"
    - "crates/plainpad-core/src/lib.rs"
    - "crates/plainpad-core/src/mathwrap.rs"
    - "crates/plainpad-core/src/intent.rs"
    - "crates/plainpad-core/tests/unit_conversion_tests.rs"
  key_links:
    - "units module is exported in plainpad-core/src/lib.rs"
    - "mathwrap::eval_line and evaluate call units::parse_and_convert"
    - "intent::analyze_line annotates physical conversions with kind 'conversion'"
---

# Plan 3.2: 5 Physical Unit Conversion Domains (Distance, Area, Mass, Volume, Temperature) with Regional Aliases

<objective>
Implement a high-performance physical unit conversion engine in `crates/plainpad-core/src/units.rs` covering all 5 physical dimensions specified in Antinote: Distance, Area (including regional units `cuerda`, `dunam`, `bigha`), Mass/Weight (including `stones`, `long ton`), Volume (including UK and US regional variants: `ukTeaspoon`, `ukTablespoon`, `uk pint`, `imperial gallon`, `ukFluidOunce`), and Temperature (`celsius`, `fahrenheit`, `kelvin`). Wire the engine into `mathwrap.rs` and `intent.rs` for instant inline note evaluation and annotations.

Purpose: Plainpad must match Antinote's exact unit conversion matrix including non-standard and regional land, weight, and volume measures.
Output: Module `units.rs`, updated `mathwrap.rs` and `intent.rs`, and comprehensive integration tests in `unit_conversion_tests.rs`.
</objective>

<context>
Load these files for context:
- .gsd/SPEC.md (§4.2 Comprehensive Unit Conversion Matrix)
- .gsd/findings.md (§9.2 Complete Unit Conversion Matrix)
- .gsd/REQUIREMENTS.md (REQ-10)
- crates/plainpad-core/src/mathwrap.rs
- crates/plainpad-core/src/intent.rs
- crates/plainpad-core/src/lib.rs
</context>

<tasks>

<task type="auto">
  <name>Create the 5-domain physical conversion engine with Antinote regional unit aliases</name>
  <files>
    crates/plainpad-core/src/units.rs
    crates/plainpad-core/src/lib.rs
  </files>
  <action>
    1. Create `crates/plainpad-core/src/units.rs`:
       - Define dimensions: `Distance`, `Area`, `Mass`, `Volume`, `Temperature`.
       - Implement canonical SI pivot factors for linear dimensions:
         - **Distance** (base: meter `m`):
           `millimeter`/`mm` (0.001), `centimetre`/`cm` (0.01), `meter`/`m` (1.0), `kilometer`/`km` (1000.0), `inch`/`in`/`"` (0.0254), `foot`/`feet`/`ft`/`'` (0.3048), `yard`/`yards`/`yd` (0.9144), `mile`/`miles`/`mi` (1609.344).
         - **Area** (base: square meter `m²`):
           `sq mm`/`square millimeters` (1e-6), `sq cm`/`square centimeters` (1e-4), `sq m`/`square meters` (1.0), `sq ft`/`square feet` (0.09290304), `sq yd`/`square yards` (0.83612736), `sq km`/`square kilometers` (1e6), `acre`/`acres` (4046.8564224), `hectare`/`hectares`/`ha` (10000.0).
           - Regional units from Antinote:
             - `cuerda`/`cuerdas`: `3930.395625 m²` (Puerto Rican cuerda = 0.9712 acre).
             - `dunam`/`dunams`: `1000.0 m²` (metric dunam).
             - `bigha`/`bighas`: `2500.0 m²` (standard bigha).
         - **Mass / Weight** (base: kilogram `kg`):
           `milligrams`/`mg` (1e-6), `gram`/`grams`/`g` (0.001), `kilograms`/`kilos`/`kg` (1.0), `tonne`/`metric ton` (1000.0), `pound`/`pounds`/`lb`/`lbs` (0.45359237), `ounce`/`ounces`/`oz` (0.028349523125).
           - Regional & Imperial units from Antinote:
             - `stones`/`stone`/`st`: `6.35029318 kg` (14 lb).
             - `imperial ton`/`uk ton`/`long ton`/`long tons`: `1016.0469088 kg` (2240 lb).
         - **Volume** (base: liter `l`):
           `milliliters`/`ml` (0.001), `litre`/`litres`/`l` (1.0), `cubic millimeter`/`cu mm` (1e-6), `cubic meter`/`m3` (1000.0), `cubic foot`/`cu ft` (28.316846592).
           - US vs UK regional variants from Antinote:
             - `teaspoon`/`teaspoons`/`tsp`/`tsps`: US 0.00492892159375 l.
             - `ukTeaspoon`/`uk teaspoon`/`uk teaspoons`: UK 0.00591939 l.
             - `tablespoon`/`tablespoons`/`tbsp`/`tbsps`: US 0.01478676478125 l.
             - `ukTablespoon`/`uk tablespoon`/`uk tablespoons`: UK 0.0177582 l.
             - `pint`/`pints`/`pt`: US liquid 0.473176473 l.
             - `uk pint`/`ukPint`/`imperial pint`: UK 0.56826125 l.
             - `us gallon`/`gallon`/`gal`: US liquid 3.785411784 l.
             - `imperial gallon`/`uk gallon`: UK 4.54609 l.
             - `fluid ounce`/`fl oz`: US 0.0295735295625 l.
             - `imperial fluid ounce`/`ukFluidOunce`/`uk fl oz`: UK 0.0284130625 l.
         - **Temperature** (affine conversions):
           `celsius`/`degrees C`/`°C`, `fahrenheit`/`degrees F`/`°F`, `kelvin`/`K`.
           Exact conversion routines:
           - C to F: `C * 9.0/5.0 + 32.0`, F to C: `(F - 32.0) * 5.0/9.0`
           - C to K: `C + 273.15`, K to C: `K - 273.15`
           - F to K: `(F - 32.0) * 5.0/9.0 + 273.15`, K to F: `(K - 273.15) * 9.0/5.0 + 32.0`
       - Implement `convert(val: f64, from_unit: &str, to_unit: &str) -> Option<(f64, String)>`.
       - Check dimensional consistency: conversion between differing dimensions returns `None`.
    2. Register `pub mod units;` in `crates/plainpad-core/src/lib.rs`.
    
    AVOID: Permitting conversions between incompatible dimensions (e.g. converting mass to length).
    USE: Dimension-tagging per unit entry and strict dimension validation.
  </action>
  <verify>
    cargo test -p plainpad-core --lib units::
  </verify>
  <done>
    Unit engine converts accurately between all listed units in Distance, Area (including cuerda, dunam, bigha), Mass (including stones, long tons), Volume (including UK and US variants), and Temperature.
  </done>
</task>

<task type="auto">
  <name>Integrate physical unit conversion into mathwrap and note intent detection</name>
  <files>
    crates/plainpad-core/src/mathwrap.rs
    crates/plainpad-core/src/intent.rs
  </files>
  <action>
    1. In `crates/plainpad-core/src/units.rs`:
       - Implement natural-language parser `parse_and_convert(line: &str) -> Option<(String, String)>`:
         - Extracts pattern: `VALUE [FROM_UNIT] (in|to|into|as|=) [TO_UNIT]`.
         - Normalizes punctuation (`?`, `=`, trailing spaces).
         - Returns `Some((formatted_result, explanation_string))`.
    2. In `crates/plainpad-core/src/mathwrap.rs`:
       - In `eval_line`: check `crate::units::parse_and_convert(raw)` before falling back to fend.
       - If matched, return `Some(MathOutcome { result: formatted_result, unit_aware: true })`.
    3. In `crates/plainpad-core/src/intent.rs`:
       - In `analyze_line`: add a dedicated physical conversion intent branch checking `crate::units::parse_and_convert(t)`.
       - Emit `Annotation` with `kind: "conversion"`, `confidence: 0.95`, `value: formatted_result`, `chip: format!("Convert ({})", formatted_result)`, and human-readable `why: explanation`.
    
    AVOID: Confusing currency queries with physical unit queries (e.g. `$50 in EUR` vs `50 km in miles`).
    USE: Check currency first or disambiguate by checking recognized unit dictionaries.
  </action>
  <verify>
    cargo test -p plainpad-core --lib intent::
  </verify>
  <done>
    Typing lines like `5 cuerdas in acres`, `14 st in kg`, or `2 uk pint in ml` returns computed conversion results in `eval_line` and generates conversion annotations in `intent::analyze`.
  </done>
</task>

<task type="auto">
  <name>Create comprehensive unit conversion test suite covering all 5 dimensions and regional variants</name>
  <files>
    crates/plainpad-core/tests/unit_conversion_tests.rs
  </files>
  <action>
    1. Create `crates/plainpad-core/tests/unit_conversion_tests.rs`.
    2. Add comprehensive tests:
       - **Distance**:
         - `1000 mm in m` -> `1 m`
         - `5 km in miles` -> ~3.1068 mi
         - `12 in in cm` -> 30.48 cm
         - `6 ft in m` -> ~1.8288 m
         - `3 yd in feet` -> 9 ft
       - **Area**:
         - `100 sq ft in sq m` -> ~9.29 sq m
         - `5 cuerdas in acres` -> ~4.856 acres
         - `1 dunam in sq m` -> 1000 sq m
         - `1 bigha in sq m` -> 2500 sq m
         - `1 hectare in acres` -> ~2.471 acres
       - **Mass / Weight**:
         - `1000 g in kg` -> 1 kg
         - `14 stones in lbs` -> 196 lbs
         - `1 stone in kg` -> ~6.3503 kg
         - `1 long ton in lbs` -> 2240 lbs
         - `1 imperial ton in kg` -> ~1016.05 kg
         - `16 oz in lb` -> 1 lb
       - **Volume**:
         - `1000 ml in l` -> 1 l
         - `1 us gallon in litres` -> ~3.7854 l
         - `1 imperial gallon in litres` -> ~4.5461 l
         - `1 us pint in ml` -> ~473.18 ml
         - `1 uk pint in ml` -> ~568.26 ml
         - `1 tbsp in tsp` -> 3 tsp
         - `1 ukTeaspoon in ml` -> ~5.92 ml
         - `1 ukTablespoon in ml` -> ~17.76 ml
         - `1 ukFluidOunce in ml` -> ~28.41 ml
         - `1 fl oz in ml` -> ~29.57 ml
       - **Temperature**:
         - `32 fahrenheit in celsius` -> 0 °C
         - `212 fahrenheit in celsius` -> 100 °C
         - `0 celsius in fahrenheit` -> 32 °F
         - `100 celsius in fahrenheit` -> 212 °F
         - `0 celsius in kelvin` -> 273.15 K
         - `-40 fahrenheit in celsius` -> -40 °C
       - **Validation**:
         - Mismatched dimension queries (e.g. `10 m in kg`) fail safely with `None`.
         - Unsupported strings fail safely.
    
    AVOID: Floating point precision drift issues.
    USE: Approximate float comparison (`(actual - expected).abs() < 1e-3`) or normalized string format checks.
  </action>
  <verify>
    cargo test -p plainpad-core --test unit_conversion_tests
  </verify>
  <done>
    All unit conversion integration tests pass with 0 failures, proving 100% compliance with REQ-10.
  </done>
</task>

</tasks>

<verification>
After all tasks complete, verify:
- [ ] Distance conversions (mm, cm, m, km, in, ft, yd, mi) function bidirectionally
- [ ] Regional area conversions (cuerda, dunam, bigha) calculate exact values
- [ ] Regional mass conversions (stones, long tons) calculate exact values
- [ ] UK vs US volume variants (ukTeaspoon, ukTablespoon, uk pint, imperial gallon, ukFluidOunce) calculate accurately
- [ ] Temperature affine conversions between Celsius, Fahrenheit, and Kelvin calculate exact values
- [ ] `cargo test -p plainpad-core --test unit_conversion_tests` passes cleanly
</verification>

<success_criteria>
- [ ] All 3 tasks verified passing
- [ ] Must-haves confirmed
- [ ] Zero test regressions across `plainpad-core`
</success_criteria>
