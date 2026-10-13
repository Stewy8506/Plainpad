---
phase: 3
plan: 3
wave: 3
depends_on:
  - "3.2"
files_modified:
  - crates/plainpad-core/src/currency.rs
  - src-tauri/src/main.rs
  - crates/plainpad-core/tests/currency_tests.rs
autonomous: true
gap_closure: false
must_haves:
  truths:
    - "Currency engine supports 160+ fiat currencies and top cryptocurrencies with instant offline lookups from seed cache"
    - "Background currency rate sync refreshes live exchange rates without blocking the UI or IPC threads"
    - "Currency and number formatting respects configurable thousandsSeparator (comma, dot, space) and numberOfDecimalPoints settings"
    - "Manual currency rate overrides (_manualCurrencyRates) can be persisted in SQLite app_state and override seed rates"
    - "Tauri IPC commands expose rate sync, format preferences, and direct currency conversion"
  artifacts:
    - "crates/plainpad-core/src/currency.rs"
    - "src-tauri/src/main.rs"
    - "crates/plainpad-core/tests/currency_tests.rs"
  key_links:
    - "src-tauri/src/main.rs exposes sync_currency_rates, get_currency_rates_info, and set_currency_format_settings commands"
    - "currency::format_converted applies thousandsSeparator and numberOfDecimalPoints from SQLite app_state"
    - "intent::analyze_line evaluates currency conversion queries with formatted results"
---

# Plan 3.3: Currency Exchange Engine with 160+ Currencies, Cache Sync, Thousand Separators, and Decimal Precision

<objective>
Enhance the currency and crypto conversion engine in `crates/plainpad-core/src/currency.rs` to support complete 1:1 parity with Antinote. Guarantee coverage for 160+ fiat currencies and top cryptocurrencies (BTC, ETH, SOL), background rate refresh with persistent fallback caching, user-configurable formatting (thousands separators: comma, dot, space; configurable decimal precision via `numberOfDecimalPoints`), and manual rate overrides (`_manualCurrencyRates`). Expose non-blocking Tauri IPC endpoints in `src-tauri/src/main.rs` ensuring sub-15ms latency and zero UI freeze (NFR-01, NFR-02).

Purpose: Users can seamlessly convert between any global currencies, cryptocurrencies, or custom pegged values with formatting matching their regional preferences.
Output: Configurable currency engine in `currency.rs`, background sync and settings commands in `main.rs`, and integration test suite in `currency_tests.rs`.
</objective>

<context>
Load these files for context:
- .gsd/SPEC.md (§4.2 Comprehensive Unit Conversion Matrix - Currency, §11 Preferences table)
- .gsd/findings.md (§9.2 Complete Unit Conversion Matrix)
- .gsd/REQUIREMENTS.md (REQ-11, NFR-01, NFR-02)
- crates/plainpad-core/src/currency.rs
- crates/plainpad-core/src/seed_rates.json
- src-tauri/src/main.rs
</context>

<tasks>

<task type="auto">
  <name>Implement configurable thousands separators, decimal precision, and manual rate overrides</name>
  <files>
    crates/plainpad-core/src/currency.rs
  </files>
  <action>
    1. In `crates/plainpad-core/src/currency.rs`:
       - Define `CurrencyFormatConfig` struct:
         ```rust
         #[derive(Serialize, Deserialize, Clone, Debug)]
         pub struct CurrencyFormatConfig {
             pub thousands_separator: String,   // default: "," (or ".", " ")
             pub decimal_separator: String,     // default: "." (or ",")
             pub decimal_points: usize,         // default: 2
             pub use_thousands_separator: bool, // default: true
         }
         ```
       - Maintain a global thread-safe `RwLock<CurrencyFormatConfig>` with accessor functions:
         - `pub fn set_format_config(config: CurrencyFormatConfig)`
         - `pub fn get_format_config() -> CurrencyFormatConfig`
       - Update `format_converted(amount: f64, target_code: &str) -> String`:
         - Read active `CurrencyFormatConfig`.
         - Format integer component using `thousands_separator` if `use_thousands_separator` is true.
         - Format fractional component with `decimal_points` precision (except for tiny crypto values `< 0.0001` where dynamic satoshi precision is retained unless strictly overridden).
         - Join integer and fraction with `decimal_separator`.
       - Implement manual rate overrides in `CurrencyEngine`:
         - Add field `manual_rates: HashMap<String, f64>`.
         - Add methods `set_manual_rate(&mut self, code: &str, rate: f64)` and `remove_manual_rate(&mut self, code: &str)`.
         - Update `get_rate(&self, code: &str)` to prioritize `manual_rates` before bundled/cached rates.
         - Provide functions `get_manual_rates(&self) -> HashMap<String, f64>` and `load_manual_rates(&mut self, rates: HashMap<String, f64>)`.
    
    AVOID: Unsynchronized mutable global variables.
    USE: `RwLock` and `OnceLock` for thread-safe concurrent access across worker and IPC threads.
  </action>
  <verify>
    cargo test -p plainpad-core --lib currency::
  </verify>
  <done>
    `format_converted` formats outputs matching custom thousand separators (comma, dot, space) and decimal precision, and manual rate overrides take precedence over seed rates.
  </done>
</task>

<task type="auto">
  <name>Implement non-blocking background rates sync routine and Tauri IPC endpoints</name>
  <files>
    crates/plainpad-core/src/currency.rs
    src-tauri/src/main.rs
  </files>
  <action>
    1. In `crates/plainpad-core/src/currency.rs`:
       - Ensure `seed_rates.json` covers 160+ ISO currency codes and crypto tokens (`BTC`, `ETH`, `SOL`).
       - Add `pub fn sync_rates_from_url(url: &str) -> Result<RatesInfo, String>` to perform a fetch and update `GLOBAL_ENGINE`.
       - Add helper `pub fn init_from_store_settings(store: &plainpad_core::store::notes::SqliteStore)` to restore `thousandsSeparator`, `numberOfDecimalPoints`, and `_manualCurrencyRates` from `app_state`.
    2. In `src-tauri/src/main.rs`:
       - Implement Tauri IPC commands:
         - `get_currency_rates_info()` -> `RatesInfo`
         - `sync_currency_rates(state: tauri::State<'_, AppStateLock>) -> Result<RatesInfo, String>`:
           Fetches updated rates asynchronously in a detached task without blocking the UI (NFR-02 compliance), saves `lastCurrencyUpdateDate` in `app_state`, and returns updated `RatesInfo`.
         - `set_currency_format_settings(state: tauri::State<'_, AppStateLock>, thousands_sep: String, decimal_sep: String, decimal_points: usize)`:
           Persists `thousandsSeparator` and `numberOfDecimalPoints` to SQLite `app_state` and updates active engine format config.
         - `set_manual_currency_rate(state: tauri::State<'_, AppStateLock>, code: String, rate: f64) -> Result<(), String>`:
           Saves manual rate into `_manualCurrencyRates` JSON map in `app_state` and updates currency engine.
         - `convert_currency_ipc(amount: f64, from: String, to: String) -> Result<String, String>`:
           Evaluates currency conversion directly and returns formatted result.
       - On app startup, initialize currency settings from `SqliteStore` and spawn a background task to refresh exchange rates if network is reachable.
       - Register commands in `generate_handler![...]`.
    
    AVOID: Blocking synchronous network calls in Tauri commands or UI initialization.
    USE: Non-blocking background execution with instant fallback to bundled `seed_rates.json`.
  </action>
  <verify>
    cargo check --manifest-path src-tauri/Cargo.toml
  </verify>
  <done>
    Tauri exposes IPC commands for non-blocking currency sync, format settings, manual rates, and direct conversions, with app startup loading saved preferences.
  </done>
</task>

<task type="auto">
  <name>Create integration test suite for 160+ currencies, crypto conversions, formatting, and manual overrides</name>
  <files>
    crates/plainpad-core/tests/currency_tests.rs
  </files>
  <action>
    1. Create `crates/plainpad-core/tests/currency_tests.rs`.
    2. Add comprehensive tests:
       - Currency seed verification:
         - Engine initializes successfully with `currency_count >= 160`.
         - Supports major fiat currencies: `USD`, `EUR`, `GBP`, `CAD`, `AUD`, `JPY`, `CHF`, `INR`, `BRL`, `CNY`, `KRW`, `RUB`, `ZAR`, etc.
         - Supports cryptocurrencies: `BTC`, `ETH`, `SOL`.
       - Natural language query parsing:
         - `"$100 USD in EUR"`
         - `"50 eur to gbp ="`
         - `"₹5,000 in usd"`
         - `"0.5 btc to usd"`
         - `"100 dollars in euros"`
         - `"5000 yen to usd"`
         - `"100 usd into eur"`
         - `"50 usd = inr"`
       - Formatting configurations:
         - Comma thousands + period decimal: `1,250.50`
         - Space thousands + comma decimal: `1 250,50`
         - Period thousands + comma decimal: `1.250,50`
         - Zero decimal precision (`numberOfDecimalPoints = 0`): `1,251`
         - Four decimal precision (`numberOfDecimalPoints = 4`): `1,250.5000`
       - Manual rate overrides:
         - Add custom manual rate for currency code (e.g. `TEST = 2.5 USD`).
         - Verify `100 USD in TEST` computes using override rate.
         - Verify overriding existing currency (e.g. `EUR = 1.0 USD`) changes conversion result accordingly.
       - Edge cases & safety:
         - Non-numeric or invalid query strings return `None`.
         - Incompatible or non-existent currency codes return `None`.
    
    AVOID: External network reliance in automated tests.
    USE: Local seed rates and in-memory override verification for deterministic test execution.
  </action>
  <verify>
    cargo test -p plainpad-core --test currency_tests
  </verify>
  <done>
    All currency integration tests pass with 0 failures, proving 100% compliance with REQ-11, NFR-01, and NFR-02.
  </done>
</task>

</tasks>

<verification>
After all tasks complete, verify:
- [ ] 160+ fiat currencies and top cryptos (BTC, ETH, SOL) load and convert bidirectionally
- [ ] Natural language phrases ($100 in EUR, 50 eur to gbp, ₹5,000 in usd) parse accurately
- [ ] Custom thousands separators (comma, period, space) format properly
- [ ] Custom decimal precision (0, 2, 4) formats properly
- [ ] Manual rate overrides take precedence over seed rates
- [ ] Background sync and Tauri IPC commands operate without blocking UI
- [ ] `cargo test -p plainpad-core --test currency_tests` passes cleanly
</verification>

<success_criteria>
- [ ] All 3 tasks verified passing
- [ ] Must-haves confirmed
- [ ] Zero test regressions across `plainpad-core`
</success_criteria>
