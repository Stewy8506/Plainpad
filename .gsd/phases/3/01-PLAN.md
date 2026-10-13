---
phase: 3
plan: 1
wave: 1
depends_on: []
files_modified:
  - crates/plainpad-core/src/mathwrap.rs
  - src-tauri/src/main.rs
  - crates/plainpad-core/tests/math_grammar_tests.rs
autonomous: true
gap_closure: false
must_haves:
  truths:
    - "Mathematical evaluation correctly resolves mixed fractions such as '1 1/2' and '3 1/4' to their decimal sums (1.5 and 3.25)"
    - "Exponents formatted as '2^8' and '2**8' evaluate consistently to 256, and scientific notation '1e6' evaluates to 1000000"
    - "Implicit multiplication parses expressions like '4(12+3)' and '(2+3)(4+5)' following standard algebraic precedence"
    - "Angle mode configuration supports 'degrees' vs 'radians' mode so sin(90) evaluates to 1 in degrees and sin(pi/2) evaluates to 1 in radians"
    - "Hyperbolic functions (sinh, cosh, tanh, asinh, acosh, atanh) and root functions (sqrt, cuberoot) evaluate accurately"
  artifacts:
    - "crates/plainpad-core/src/mathwrap.rs"
    - "src-tauri/src/main.rs"
    - "crates/plainpad-core/tests/math_grammar_tests.rs"
  key_links:
    - "mathwrap::evaluate_with_mode is called by Tauri IPC eval_math with angleMeasurementMode preference"
    - "mathwrap::eval_line applies preprocessing pipeline before fend-core evaluation"
---

# Plan 3.1: Math Grammar, Tokenizer & Expression Rewriter with Mixed Fractions, Exponents, Implicit Multiplication, and Angle Modes

<objective>
Implement a robust mathematical preprocessing and expression-rewriting pipeline in `plainpad-core::mathwrap` replicating Antinote's DDMathParser-derived grammar. Support mixed fractions (`1 1/2`, `3 1/4`), power operators (`^`, `**`), scientific notation (`1e6`), implicit multiplication (`4(12+3)`, `(2+3)(4+5)`), roots (`cuberoot`), and configurable angle measurement modes (`degrees` vs `radians`) for trigonometric and hyperbolic functions. Expose angle mode settings via Tauri IPC and verify with a dedicated integration test suite.

Purpose: Antinote scratchpad allows natural typing of mixed fractions, implicit multiplication, and scientific numbers with immediate accurate inline evaluation.
Output: Rust preprocessing routines in `mathwrap.rs`, angle mode configuration in `src-tauri/src/main.rs`, and comprehensive integration tests in `math_grammar_tests.rs`.
</objective>

<context>
Load these files for context:
- .gsd/SPEC.md (§4.1 Number Formatting & Grammar, §11 Preferences table)
- .gsd/findings.md (§9.1 Parser Architecture)
- .gsd/REQUIREMENTS.md (REQ-08, REQ-09)
- crates/plainpad-core/src/mathwrap.rs
- crates/plainpad-core/src/intent.rs
- src-tauri/src/main.rs
</context>

<tasks>

<task type="auto">
  <name>Implement mathematical preprocessor for mixed fractions, exponents, scientific notation, and implicit multiplication</name>
  <files>
    crates/plainpad-core/src/mathwrap.rs
  </files>
  <action>
    1. In `crates/plainpad-core/src/mathwrap.rs`, implement `preprocess_math_grammar(expr: &str) -> String`:
       - **Mixed Fractions**:
         - Detect mixed fractions matching pattern `\b(\d+)\s+(\d+)\/(\d+)\b` (e.g. `1 1/2`, `3 1/4`, `5 7/8`).
         - Rewrite `A B/C` into `((A) + ((B)/(C)))` so that fend evaluates them as exact fractional sums.
         - Support signed mixed fractions `-A B/C` -> `-((A) + ((B)/(C)))`.
         - Ensure simple fractions (`1/2`, `3/4`) are preserved without incorrect double nesting.
       - **Exponents & Roots**:
         - Standardize `^` and `**` operators for power operations.
         - Ensure bitwise XOR (like `0xFF ^ 0x0F` handled in `eval_bitwise`) is not corrupted when operands are hex/bitwise, while decimal `2^8` maps to exponentiation `2^8` / `2**8`.
         - Map `cuberoot(x)` to `cbrt(x)` or `(x)^(1/3)`.
       - **Scientific Notation**:
         - Support notation like `1e6`, `2.5e-3`, `3E8` without treating `e` as an identifier token or unit name.
       - **Implicit Multiplication**:
         - Rewrite patterns where a number or closing parenthesis is immediately followed by an opening parenthesis:
           - Number before parenthesis: `(\d+)\s*\(` -> `$1 * (` (e.g. `4(12+3)` -> `4 * (12+3)`).
           - Adjacent parentheses: `\)\s*\(` -> `) * (` (e.g. `(2+3)(4+5)` -> `(2+3) * (4+5)`).
           - Parenthesis before number: `\)\s*(\d+)` -> `) * $1`.
           - Number before function name: `(\d+)\s*(sin|cos|tan|asin|acos|atan|sinh|cosh|tanh|asinh|acosh|atanh|sqrt|cuberoot)\(` -> `$1 * $2(`.
    2. Hook `preprocess_math_grammar` into `evaluate_uncached` and `eval_line` pipeline after variable substitution and percent rewriting.
    
    AVOID: Naive regex replacements that alter non-math text or break multi-word labels.
    USE: Word boundary checks and stateful token scanning for mathematical rewriting.
  </action>
  <verify>
    cargo test -p plainpad-core --lib mathwrap::
  </verify>
  <done>
    `1 1/2` evaluates to 1.5, `3 1/4 + 2` evaluates to 5.25, `2^8` and `2**8` evaluate to 256, `1e6` evaluates to 1,000,000, `4(12+3)` evaluates to 60, and `cuberoot(27)` evaluates to 3.
  </done>
</task>

<task type="auto">
  <name>Implement configurable angle measurement modes (degrees vs radians) and trigonometric/hyperbolic functions</name>
  <files>
    crates/plainpad-core/src/mathwrap.rs
    src-tauri/src/main.rs
  </files>
  <action>
    1. In `crates/plainpad-core/src/mathwrap.rs`:
       - Define `#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)] pub enum AngleMode { Degrees, Radians }`.
       - Default `AngleMode::Degrees` (matching SPEC §11 table `angleMeasurementMode: "degrees"`).
       - Add `evaluate_with_mode(expr: &str, mode: AngleMode) -> Result<MathOutcome, String>`.
       - Add `eval_line_with_mode(expr: &str, vars: &Vars, mode: AngleMode) -> Option<MathOutcome>`.
       - When `mode == AngleMode::Degrees`:
         - For direct trigonometric functions `sin`, `cos`, `tan`: wrap arguments so fend calculates with degrees units (e.g. `sin(90)` -> `sin((90) deg)` or `sin((90) degrees)`).
         - For inverse trigonometric functions `asin`, `acos`, `atan`: append ` in degrees` so fend converts radian results to degrees.
         - Hyperbolic functions (`sinh`, `cosh`, `tanh`, `asinh`, `acosh`, `atanh`) remain unscaled since hyperbolic arguments are dimensionless hyperbolic angles.
       - When `mode == AngleMode::Radians`:
         - Evaluate trigonometric functions in standard radians without degrees wrapper.
    2. In `src-tauri/src/main.rs`:
       - Update `eval_math`: read `angleMeasurementMode` from `state.store.get_setting("angleMeasurementMode")` (defaulting to `"degrees"`), and call `evaluate_with_mode`.
       - Add Tauri command `set_angle_mode(state: tauri::State<'_, AppStateLock>, mode: String) -> Result<(), String>` validating `"degrees"` or `"radians"` and persisting in store `angleMeasurementMode`.
       - Add Tauri command `get_angle_mode(state: tauri::State<'_, AppStateLock>) -> Result<String, String>` returning current angle mode.
       - Register commands in `generate_handler![...]`.
    
    AVOID: Forcing radians as the only angle mode when Antinote defaults to degrees.
    USE: Explicit `AngleMode` enum and store-backed setting with graceful fallback to Degrees.
  </action>
  <verify>
    cargo test -p plainpad-core --lib mathwrap::
  </verify>
  <done>
    `sin(90)` evaluates to 1 in degrees mode, `sin(pi/2)` evaluates to 1 in radians mode, hyperbolic functions evaluate correctly, and Tauri IPC commands get/set angle mode successfully.
  </done>
</task>

<task type="auto">
  <name>Create integration test suite for math grammar, fractions, implicit multiplication, and angle modes</name>
  <files>
    crates/plainpad-core/tests/math_grammar_tests.rs
  </files>
  <action>
    1. Create `crates/plainpad-core/tests/math_grammar_tests.rs`.
    2. Write test cases covering:
       - Mixed fractions: `1 1/2`, `3 1/4`, `5 7/8`, `2 1/3 + 1 2/3 = 4`.
       - Exponents & scientific notation: `2^8`, `2**8`, `10^3`, `1e6`, `2.5e3`, `3e-2`.
       - Implicit multiplication: `4(12+3)`, `(2+3)(4+5)`, `5(2)`, `3(4 + 2(1 + 1))`.
       - Precedence: `10 + 2 * 6`, `(10 + 2) * 6`, `2^3 * 4`, `2 + 3^2`.
       - Trig in `Degrees` mode: `sin(90) == 1`, `cos(0) == 1`, `cos(90) == 0`, `tan(45) == 1`, `asin(1) == 90 deg`.
       - Trig in `Radians` mode: `sin(pi/2) == 1`, `cos(pi) == -1`, `tan(pi/4) == 1`.
       - Hyperbolic & roots: `sinh(0) == 0`, `cosh(0) == 1`, `tanh(0) == 0`, `sqrt(144) == 12`, `cuberoot(125) == 5`.
    
    AVOID: Strict floating point equality assertions (`== 1.0`); use epsilon tolerance checks (`(val - 1.0).abs() < 1e-4`) or trimmed result string comparisons.
    USE: Clean test assertions with helpful diagnostic messages on failure.
  </action>
  <verify>
    cargo test -p plainpad-core --test math_grammar_tests
  </verify>
  <done>
    All integration tests in `math_grammar_tests.rs` pass with 0 failures, proving REQ-08 and REQ-09 requirements are met.
  </done>
</task>

</tasks>

<verification>
After all tasks complete, verify:
- [ ] `1 1/2` evaluates to 1.5 and `3 1/4` evaluates to 3.25
- [ ] `2^8` and `2**8` evaluate to 256
- [ ] `4(12+3)` evaluates to 60 via implicit multiplication
- [ ] `sin(90)` in degrees mode evaluates to 1 and `sin(pi/2)` in radians mode evaluates to 1
- [ ] `sinh`, `cosh`, `tanh`, `sqrt`, and `cuberoot` evaluate accurately
- [ ] `cargo test -p plainpad-core --test math_grammar_tests` passes cleanly
</verification>

<success_criteria>
- [ ] All 3 tasks verified passing
- [ ] Must-haves confirmed
- [ ] Zero test regressions across `plainpad-core`
</success_criteria>
