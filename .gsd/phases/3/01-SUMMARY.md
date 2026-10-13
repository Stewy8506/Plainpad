# Phase 3 Plan 1 Summary: Math Grammar, Tokenizer & Expression Rewriter with Mixed Fractions, Exponents, Implicit Multiplication, and Angle Modes

**Plan:** `.gsd/phases/3/01-PLAN.md`  
**Phase:** 3  
**Status:** Complete  
**Commits:** `15880b6`, `e887430`, `be878d6`  

---

## 1. Objectives & Overview

Replicated Antinote's DDMathParser-derived grammar and arithmetic evaluation layer in `plainpad-core::mathwrap` and exposed angle measurement preferences through Tauri IPC:
- Mixed fractions (`1 1/2`, `3 1/4`, `5 7/8`, signed `-1 1/2`) rewritten to fractional expressions (`((A) + ((B)/(C)))`) for exact evaluation.
- Power operators (`^`, `**`) and scientific notation (`1e6`, `2.5e3`, `3e-2`).
- Programmer math bitwise operations disambiguated: hex/binary `0xFF ^ 0x0F` evaluates to bitwise XOR (`240`), whereas decimal `2^8` evaluates to exponentiation (`256`).
- Implicit multiplication support: `4(12+3)` -> `4 * (12+3)`, `(2+3)(4+5)` -> `(2+3) * (4+5)`, `(2+3) 4` -> `(2+3) * 4`, and `4 sin(90)` -> `4 * sin(90)`.
- Configurable angle measurement modes (`degrees` vs `radians`):
  - In `degrees` mode (default), direct circular trigonometric functions (`sin`, `cos`, `tan`) have arguments automatically wrapped with `((arg) deg)` when lacking explicit units, and inverse functions (`asin`, `acos`, `atan`) convert results to degrees.
  - In `radians` mode, expressions evaluate in native radians.
  - Hyperbolic functions (`sinh`, `cosh`, `tanh`, `asinh`, `acosh`, `atanh`) and root functions (`sqrt`, `cbrt` / `cuberoot`) evaluate accurately across all modes.
- Tauri IPC commands `eval_math`, `set_angle_mode`, and `get_angle_mode` backed by SQLite configuration store.

---

## 2. Tasks Completed

| Task | Name | Commit | Key Changes |
|---|---|---|---|
| 1 | Mathematical preprocessor for mixed fractions, exponents, scientific notation, and implicit multiplication | `15880b6` | Added `preprocess_math_grammar`, `rewrite_cuberoot`, `rewrite_mixed_fractions`, `rewrite_implicit_multiplication`, bitwise disambiguation in `mathwrap.rs`. |
| 2 | Configurable angle measurement modes (degrees vs radians) and trigonometric/hyperbolic functions | `e887430` | Added `AngleMode` enum, `evaluate_with_mode`, `eval_line_with_mode`, `apply_degrees_mode`, and Tauri IPC commands `set_angle_mode`/`get_angle_mode` in `src-tauri/src/main.rs`. |
| 3 | Integration test suite for math grammar, fractions, implicit multiplication, and angle modes | `be878d6` | Added `crates/plainpad-core/tests/math_grammar_tests.rs` with 8 comprehensive integration test cases covering REQ-08 and REQ-09. |

---

## 3. Verification & Empirical Proof

### Test Runner Results
- Unit tests (`cargo test -p plainpad-core --lib mathwrap::`): **11 passed, 0 failed**.
- Integration tests (`cargo test -p plainpad-core --test math_grammar_tests`): **8 passed, 0 failed**.
- Entire workspace tests (`cargo test -p plainpad-core`): **69 passed, 0 failed**.
- Shell build check (`cargo check -p plainpad`): **Passed, 0 errors, 0 warnings**.

### Key Truths Verified
- `1 1/2` -> `1.5`, `3 1/4` -> `3.25`, `2 1/3 + 1 2/3` -> `4`, `-1 1/2` -> `-1.5`, `5 - 1 1/2` -> `3.5`.
- `2^8` and `2**8` -> `256`, `1e6` -> `1000000`, `2.5e3` -> `2500`, `3e-2` -> `0.03`.
- `4(12+3)` -> `60`, `(2+3)(4+5)` -> `45`, `(2+3) 4` -> `20`, `3(4 + 2(1 + 1))` -> `24`.
- `sin(90)` in degrees mode -> `1`, `cos(0)` -> `1`, `cos(90)` -> `0`, `tan(45)` -> `1`, `asin(1)` in degrees mode -> contains `90` and `deg`.
- `sin(pi/2)` in radians mode -> `1`, `cos(pi)` -> `-1`, `tan(pi/4)` -> `1`, `asin(1)` in radians mode -> `approx. 1.5707963268`.
- `sinh(0)` -> `approx. 0`, `cosh(0)` -> `approx. 1`, `tanh(0)` -> `approx. 0`, `sqrt(144)` -> `12`, `cuberoot(125)` -> `5`, `cuberoot(27)` -> `3`.
- `0xFF ^ 0x0F` -> `240` (bitwise XOR), while `2^8` -> `256` (exponentiation).

---

## 4. Deviations & Remediation

- **Hyperbolic name overlap**: During integration test development, `rewrite_inverse_trig_degrees` matched substrings `"asin"`, `"acos"`, `"atan"`, which unintentionally intercepted hyperbolic inverse functions `asinh(0)`, `acosh(1)`, `atanh(0)`. Updated the matching logic with strict alphabetic boundary checks to distinguish circular inverse functions from hyperbolic inverse functions.

---

## 5. Artifacts Created & Modified

- `crates/plainpad-core/src/mathwrap.rs`: Preprocessing grammar routines, `AngleMode` enum, `evaluate_with_mode`, `eval_line_with_mode`, `apply_degrees_mode`.
- `src-tauri/src/main.rs`: `eval_math` updated to respect store-backed `angleMeasurementMode`, added `set_angle_mode` and `get_angle_mode` commands.
- `crates/plainpad-core/tests/math_grammar_tests.rs`: Integration tests for fractions, powers, scientific notation, implicit multiplication, and angle modes.
