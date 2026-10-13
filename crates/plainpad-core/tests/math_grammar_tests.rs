use plainpad_core::mathwrap::{
    eval_line, eval_line_with_mode, evaluate, evaluate_with_mode, AngleMode, Vars,
};

/// Helper to parse floating point results that may include fend's "approx. " prefix or commas
fn parse_number(res: &str) -> f64 {
    let cleaned = res
        .replace("approx.", "")
        .replace(',', "")
        .trim()
        .to_string();
    cleaned.parse::<f64>().unwrap_or_else(|e| {
        panic!("Failed to parse number from result '{}': {}", res, e);
    })
}

#[test]
fn test_mixed_fractions() {
    let out = evaluate("1 1/2").expect("1 1/2 must evaluate");
    assert_eq!(out.result, "1.5");

    let out = evaluate("3 1/4").expect("3 1/4 must evaluate");
    assert_eq!(out.result, "3.25");

    let out = evaluate("5 7/8").expect("5 7/8 must evaluate");
    assert_eq!(out.result, "5.875");

    let out = evaluate("2 1/3 + 1 2/3").expect("2 1/3 + 1 2/3 must evaluate");
    assert_eq!(out.result, "4");

    let out = evaluate("-1 1/2").expect("-1 1/2 must evaluate");
    assert_eq!(out.result, "-1.5");

    let out = evaluate("5 - 1 1/2").expect("5 - 1 1/2 must evaluate");
    assert_eq!(out.result, "3.5");

    let out = evaluate("10 * 3 1/4").expect("10 * 3 1/4 must evaluate");
    assert_eq!(out.result, "32.5");
}

#[test]
fn test_exponents_and_scientific_notation() {
    let out = evaluate("2^8").expect("2^8 must evaluate");
    assert_eq!(out.result, "256");

    let out = evaluate("2**8").expect("2**8 must evaluate");
    assert_eq!(out.result, "256");

    let out = evaluate("10^3").expect("10^3 must evaluate");
    assert_eq!(out.result.replace(',', ""), "1000");

    let out = evaluate("1e6").expect("1e6 must evaluate");
    assert_eq!(out.result.replace(',', ""), "1000000");

    let out = evaluate("2.5e3").expect("2.5e3 must evaluate");
    assert_eq!(out.result.replace(',', ""), "2500");

    let out = evaluate("3e-2").expect("3e-2 must evaluate");
    assert_eq!(out.result, "0.03");
}

#[test]
fn test_implicit_multiplication() {
    let out = evaluate("4(12+3)").expect("4(12+3) must evaluate");
    assert_eq!(out.result, "60");

    let out = evaluate("(2+3)(4+5)").expect("(2+3)(4+5) must evaluate");
    assert_eq!(out.result, "45");

    let out = evaluate("5(2)").expect("5(2) must evaluate");
    assert_eq!(out.result, "10");

    let out = evaluate("3(4 + 2(1 + 1))").expect("3(4 + 2(1 + 1)) must evaluate");
    assert_eq!(out.result, "24");

    let out = evaluate("(2+3) 4").expect("(2+3) 4 must evaluate");
    assert_eq!(out.result, "20");

    let out = evaluate("4(2.5)").expect("4(2.5) must evaluate");
    assert_eq!(out.result, "10");
}

#[test]
fn test_algebraic_precedence() {
    let out = evaluate("10 + 2 * 6").expect("10 + 2 * 6 must evaluate");
    assert_eq!(out.result, "22");

    let out = evaluate("(10 + 2) * 6").expect("(10 + 2) * 6 must evaluate");
    assert_eq!(out.result, "72");

    let out = evaluate("2^3 * 4").expect("2^3 * 4 must evaluate");
    assert_eq!(out.result, "32");

    let out = evaluate("2 + 3^2").expect("2 + 3^2 must evaluate");
    assert_eq!(out.result, "11");
}

#[test]
fn test_trigonometry_degrees_mode() {
    let out = evaluate_with_mode("sin(90)", AngleMode::Degrees).expect("sin(90) deg must evaluate");
    assert_eq!(out.result, "1");

    let out = evaluate_with_mode("cos(0)", AngleMode::Degrees).expect("cos(0) deg must evaluate");
    assert_eq!(out.result, "1");

    let out = evaluate_with_mode("cos(90)", AngleMode::Degrees).expect("cos(90) deg must evaluate");
    assert_eq!(out.result, "0");

    let out = evaluate_with_mode("tan(45)", AngleMode::Degrees).expect("tan(45) deg must evaluate");
    let val = parse_number(&out.result);
    assert!((val - 1.0).abs() < 1e-4, "tan(45) in degrees should be ~1, got {}", out.result);

    let out = evaluate_with_mode("asin(1)", AngleMode::Degrees).expect("asin(1) deg must evaluate");
    assert!(
        out.result.contains("90") && out.result.contains("deg"),
        "asin(1) in degrees mode must yield 90 deg, got: {}",
        out.result
    );

    let out = evaluate_with_mode("acos(1)", AngleMode::Degrees).expect("acos(1) deg must evaluate");
    assert!(
        out.result.contains('0') && out.result.contains("deg"),
        "acos(1) in degrees mode must yield 0 deg, got: {}",
        out.result
    );

    let out = evaluate_with_mode("atan(1)", AngleMode::Degrees).expect("atan(1) deg must evaluate");
    assert!(
        out.result.contains("45") && out.result.contains("deg"),
        "atan(1) in degrees mode must yield 45 deg, got: {}",
        out.result
    );

    // Default evaluate() must use Degrees mode
    let default_out = evaluate("sin(90)").expect("Default evaluate must succeed");
    assert_eq!(default_out.result, "1");
}

#[test]
fn test_trigonometry_radians_mode() {
    let out = evaluate_with_mode("sin(pi/2)", AngleMode::Radians).expect("sin(pi/2) rad must evaluate");
    assert_eq!(out.result, "1");

    let out = evaluate_with_mode("cos(pi)", AngleMode::Radians).expect("cos(pi) rad must evaluate");
    assert_eq!(out.result, "-1");

    let out = evaluate_with_mode("tan(pi/4)", AngleMode::Radians).expect("tan(pi/4) rad must evaluate");
    let val = parse_number(&out.result);
    assert!((val - 1.0).abs() < 1e-4, "tan(pi/4) in radians should be ~1, got {}", out.result);

    let out = evaluate_with_mode("asin(1)", AngleMode::Radians).expect("asin(1) rad must evaluate");
    let val = parse_number(&out.result);
    assert!(
        (val - std::f64::consts::FRAC_PI_2).abs() < 1e-4,
        "asin(1) in radians should be ~pi/2, got {}",
        out.result
    );
}

#[test]
fn test_hyperbolic_and_roots() {
    let out = evaluate("sinh(0)").expect("sinh(0) must evaluate");
    let val = parse_number(&out.result);
    assert!(val.abs() < 1e-4, "sinh(0) should be 0, got {}", out.result);

    let out = evaluate("cosh(0)").expect("cosh(0) must evaluate");
    let val = parse_number(&out.result);
    assert!((val - 1.0).abs() < 1e-4, "cosh(0) should be 1, got {}", out.result);

    let out = evaluate("tanh(0)").expect("tanh(0) must evaluate");
    let val = parse_number(&out.result);
    assert!(val.abs() < 1e-4, "tanh(0) should be 0, got {}", out.result);

    let out = evaluate("asinh(0)").expect("asinh(0) must evaluate");
    let val = parse_number(&out.result);
    assert!(val.abs() < 1e-4, "asinh(0) should be 0, got {}", out.result);

    let out = evaluate("acosh(1)").expect("acosh(1) must evaluate");
    let val = parse_number(&out.result);
    assert!(val.abs() < 1e-4, "acosh(1) should be 0, got {}", out.result);

    let out = evaluate("atanh(0)").expect("atanh(0) must evaluate");
    let val = parse_number(&out.result);
    assert!(val.abs() < 1e-4, "atanh(0) should be 0, got {}", out.result);

    let out = evaluate("sqrt(144)").expect("sqrt(144) must evaluate");
    assert_eq!(out.result, "12");

    let out = evaluate("cuberoot(125)").expect("cuberoot(125) must evaluate");
    assert_eq!(out.result, "5");

    let out = evaluate("cuberoot(27)").expect("cuberoot(27) must evaluate");
    assert_eq!(out.result, "3");
}

#[test]
fn test_eval_line_integration() {
    let mut vars = Vars::new();
    vars.insert("x".into(), "1 1/2".into());
    vars.insert("y".into(), "2".into());

    let out = eval_line("x + y", &vars).expect("x + y must evaluate");
    assert_eq!(out.result, "3.5");

    let out = eval_line_with_mode("sin(90)", &vars, AngleMode::Degrees)
        .expect("sin(90) with degrees mode must evaluate");
    assert_eq!(out.result, "1");

    let out = eval_line_with_mode("sin(pi/2)", &vars, AngleMode::Radians)
        .expect("sin(pi/2) with radians mode must evaluate");
    assert_eq!(out.result, "1");

    // Programmer math fallback for XOR vs exponentiation
    let out = eval_line("0xFF ^ 0x0F", &vars).expect("0xFF ^ 0x0F must evaluate to bitwise XOR");
    assert_eq!(out.result, "240");

    let out = eval_line("2^8", &vars).expect("2^8 must evaluate to exponentiation");
    assert_eq!(out.result, "256");
}
