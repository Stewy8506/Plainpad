use plainpad_core::currency::{
    canonicalize_currency, evaluate_currency, format_amount, format_converted,
    get_format_config, get_manual_rates, global_engine,
    init_from_store_settings, parse_currency_phrase, remove_manual_rate,
    set_format_config, set_manual_rate, sync_rates_from_url, CurrencyEngine,
    CurrencyFormatConfig,
};
use plainpad_core::intent;
use plainpad_core::store::SqliteStore;
use serde_json::json;

#[test]
fn test_currency_seed_verification() {
    let engine = CurrencyEngine::new();
    let info = engine.info();

    // Verify 160+ fiat currencies and tokens loaded
    assert!(
        info.currency_count >= 160,
        "Expected at least 160 currencies, found {}",
        info.currency_count
    );
    assert!(info.is_seed);

    // Verify major fiat currencies
    let fiat_currencies = [
        "USD", "EUR", "GBP", "CAD", "AUD", "JPY", "CHF", "INR", "BRL", "CNY",
        "KRW", "RUB", "ZAR", "SEK", "NOK", "DKK", "PLN", "THB", "IDR", "MYR",
        "PHP", "MXN", "AED", "SAR", "ILS",
    ];
    for code in &fiat_currencies {
        assert!(
            engine.has_currency(code),
            "Expected engine to have currency {}",
            code
        );
        let rate = engine.get_rate(code);
        assert!(rate.is_some(), "Rate for {} should be Some", code);
        assert!(rate.unwrap() > 0.0, "Rate for {} should be positive", code);
    }

    // Verify top cryptocurrencies
    for crypto in &["BTC", "ETH", "SOL"] {
        assert!(
            engine.has_currency(crypto),
            "Expected engine to have crypto {}",
            crypto
        );
        let rate = engine.get_rate(crypto);
        assert!(rate.is_some(), "Rate for {} should be Some", crypto);
        assert!(rate.unwrap() > 0.0, "Rate for {} should be positive", crypto);
    }

    // Test bidirectional conversions
    let usd_to_eur = engine.convert(100.0, "USD", "EUR").unwrap();
    let eur_to_usd = engine.convert(usd_to_eur, "EUR", "USD").unwrap();
    assert!((eur_to_usd - 100.0).abs() < 1e-4);

    let usd_to_btc = engine.convert(10000.0, "USD", "BTC").unwrap();
    let btc_to_usd = engine.convert(usd_to_btc, "BTC", "USD").unwrap();
    assert!((btc_to_usd - 10000.0).abs() < 1e-4);

    let usd_to_eth = engine.convert(5000.0, "USD", "ETH").unwrap();
    let eth_to_usd = engine.convert(usd_to_eth, "ETH", "USD").unwrap();
    assert!((eth_to_usd - 5000.0).abs() < 1e-4);

    let usd_to_sol = engine.convert(1000.0, "USD", "SOL").unwrap();
    let sol_to_usd = engine.convert(usd_to_sol, "SOL", "USD").unwrap();
    assert!((sol_to_usd - 1000.0).abs() < 1e-4);
}

#[test]
fn test_natural_language_queries() {
    let queries = [
        ("$100 USD in EUR", 100.0, "USD", "EUR"),
        ("50 eur to gbp =", 50.0, "EUR", "GBP"),
        ("₹5,000 in usd", 5000.0, "INR", "USD"),
        ("0.5 btc to usd", 0.5, "BTC", "USD"),
        ("100 dollars in euros", 100.0, "USD", "EUR"),
        ("5000 yen to usd", 5000.0, "JPY", "USD"),
        ("100 usd into eur", 100.0, "USD", "EUR"),
        ("50 usd = inr", 50.0, "USD", "INR"),
    ];

    for (query, expected_amt, expected_from, expected_to) in queries {
        let parsed = parse_currency_phrase(query);
        assert!(
            parsed.is_some(),
            "Failed to parse currency phrase: '{}'",
            query
        );
        let (amt, from, to) = parsed.unwrap();
        assert_eq!(amt, expected_amt, "Amount mismatch on '{}'", query);
        assert_eq!(from, expected_from, "Source currency mismatch on '{}'", query);
        assert_eq!(to, expected_to, "Target currency mismatch on '{}'", query);

        // Verify evaluate_currency returns formatted result and explanation
        let evaluated = evaluate_currency(query);
        assert!(
            evaluated.is_some(),
            "Failed to evaluate currency query: '{}'",
            query
        );
        let (val, why) = evaluated.unwrap();
        assert!(!val.is_empty(), "Empty evaluation value for '{}'", query);
        assert!(
            why.contains(expected_from) && why.contains(expected_to),
            "Explanation should mention {} and {}: got '{}'",
            expected_from,
            expected_to,
            why
        );

        // Verify intent engine integration
        let analysis = intent::analyze(query, false);
        assert!(
            !analysis.annotations.is_empty(),
            "intent::analyze failed on '{}'",
            query
        );
        let ann = &analysis.annotations[0];
        assert_eq!(ann.kind, "conversion");
        assert!(ann.confidence >= 0.9);
        assert!(!ann.value.is_empty());
    }
}

#[test]
fn test_formatting_configurations() {
    // 1. Comma thousands + period decimal: 1,250.50
    let cfg1 = CurrencyFormatConfig {
        thousands_separator: ",".to_string(),
        decimal_separator: ".".to_string(),
        decimal_points: 2,
        use_thousands_separator: true,
    };
    assert_eq!(format_amount(1250.5, &cfg1), "1,250.50");

    // 2. Space thousands + comma decimal: 1 250,50
    let cfg2 = CurrencyFormatConfig {
        thousands_separator: " ".to_string(),
        decimal_separator: ",".to_string(),
        decimal_points: 2,
        use_thousands_separator: true,
    };
    assert_eq!(format_amount(1250.5, &cfg2), "1 250,50");

    // 3. Period thousands + comma decimal: 1.250,50
    let cfg3 = CurrencyFormatConfig {
        thousands_separator: ".".to_string(),
        decimal_separator: ",".to_string(),
        decimal_points: 2,
        use_thousands_separator: true,
    };
    assert_eq!(format_amount(1250.5, &cfg3), "1.250,50");

    // 4. Zero decimal precision (numberOfDecimalPoints = 0): 1,251
    let cfg0 = CurrencyFormatConfig {
        thousands_separator: ",".to_string(),
        decimal_separator: ".".to_string(),
        decimal_points: 0,
        use_thousands_separator: true,
    };
    assert_eq!(format_amount(1250.5, &cfg0), "1,251");

    // 5. Four decimal precision (numberOfDecimalPoints = 4): 1,250.5000
    let cfg4 = CurrencyFormatConfig {
        thousands_separator: ",".to_string(),
        decimal_separator: ".".to_string(),
        decimal_points: 4,
        use_thousands_separator: true,
    };
    assert_eq!(format_amount(1250.5, &cfg4), "1,250.5000");

    // Test format_converted with global format config
    set_format_config(cfg2);
    let formatted_custom = format_converted(1250.5, "USD");
    assert_eq!(formatted_custom, "$1 250,50");

    set_format_config(cfg0);
    let formatted_zero = format_converted(1250.5, "USD");
    assert_eq!(formatted_zero, "$1,251");

    set_format_config(cfg4);
    let formatted_four = format_converted(1250.5, "EUR");
    assert_eq!(formatted_four, "€1,250.5000");

    // Reset back to default
    set_format_config(CurrencyFormatConfig::default());
    let formatted_default = format_converted(1250.5, "USD");
    assert_eq!(formatted_default, "$1,250.50");
}

#[test]
fn test_manual_rate_overrides() {
    // 1. Add custom manual rate for currency code (e.g. TEST = 2.5 USD)
    set_manual_rate("TEST", 2.5);

    let engine = global_engine();
    let lock = engine.read().unwrap();
    let rate = lock.get_rate("TEST");
    assert_eq!(rate, Some(2.5));

    // Verify 100 USD in TEST computes 250
    let converted = lock.convert(100.0, "USD", "TEST");
    assert_eq!(converted, Some(250.0));
    drop(lock);

    // Verify evaluate_currency parses and converts with TEST
    let res = evaluate_currency("100 USD in TEST");
    assert!(res.is_some());
    let (val, why) = res.unwrap();
    assert!(val.contains("250"));
    assert!(why.contains("converted 100 USD to TEST"));

    // 2. Verify overriding existing currency (e.g. EUR = 1.0 USD)
    set_manual_rate("EUR", 1.0);
    let lock2 = engine.read().unwrap();
    let eur_rate = lock2.get_rate("EUR");
    assert_eq!(eur_rate, Some(1.0));
    let converted_eur = lock2.convert(100.0, "USD", "EUR");
    assert_eq!(converted_eur, Some(100.0));
    drop(lock2);

    let res_eur = evaluate_currency("100 USD in EUR");
    assert!(res_eur.is_some());
    let (val_eur, _) = res_eur.unwrap();
    assert!(val_eur.contains("100"));

    // 3. Remove overrides
    remove_manual_rate("TEST");
    remove_manual_rate("EUR");

    let lock3 = engine.read().unwrap();
    assert_eq!(lock3.get_rate("TEST"), None);
    assert_ne!(lock3.get_rate("EUR"), Some(1.0));
}

#[test]
fn test_sqlite_settings_hydration() {
    let store = SqliteStore::open_in_memory().unwrap();

    // Persist custom settings in app_state
    store
        .set_setting("thousandsSeparator", json!(" "))
        .unwrap();
    store
        .set_setting("numberOfDecimalPoints", json!(3))
        .unwrap();
    store
        .set_setting("_manualCurrencyRates", json!({ "XYZ": 12.5 }))
        .unwrap();

    // Hydrate
    init_from_store_settings(&store);

    let cfg = get_format_config();
    assert_eq!(cfg.thousands_separator, " ");
    assert_eq!(cfg.decimal_points, 3);

    let manual = get_manual_rates();
    assert_eq!(manual.get("XYZ"), Some(&12.5));

    let engine = global_engine();
    let lock = engine.read().unwrap();
    assert_eq!(lock.convert(100.0, "USD", "XYZ"), Some(1250.0));
    drop(lock);

    // Cleanup
    remove_manual_rate("XYZ");
    set_format_config(CurrencyFormatConfig::default());
}

#[test]
fn test_edge_cases_and_safety() {
    // Non-currency strings
    assert_eq!(parse_currency_phrase(""), None);
    assert_eq!(parse_currency_phrase("   "), None);
    assert_eq!(parse_currency_phrase("not a currency"), None);
    assert_eq!(parse_currency_phrase("goto main"), None);
    assert_eq!(parse_currency_phrase("50 to 100"), None);
    assert_eq!(parse_currency_phrase("100 in"), None);

    assert_eq!(evaluate_currency("not a currency"), None);
    assert_eq!(evaluate_currency("goto main"), None);
    assert_eq!(evaluate_currency("50 to 100"), None);

    // Incompatible or non-existent currency codes
    assert_eq!(evaluate_currency("100 USD in NONEXISTENT"), None);
    assert_eq!(evaluate_currency("100 FAKE in EUR"), None);
    assert_eq!(evaluate_currency("abc in def"), None);

    // Negative amounts should return None
    assert_eq!(parse_currency_phrase("-50 usd in eur"), None);

    // Canonicalization edge cases
    assert_eq!(canonicalize_currency(""), None);
    assert_eq!(canonicalize_currency("unknown_token"), None);
}

#[test]
fn test_sync_rates_from_url() {
    // Invalid source
    let err = sync_rates_from_url("invalid://not-found");
    assert!(err.is_err());

    // Valid raw json update
    let sample_json = r#"{
        "result": "success",
        "time_last_update_unix": 1700000000,
        "time_last_update_utc": "Thu, 15 Nov 2023 00:00:00 +0000",
        "base_code": "USD",
        "rates": {
            "USD": 1.0,
            "EUR": 0.95,
            "GBP": 0.80,
            "INR": 83.0,
            "BTC": 0.00002
        }
    }"#;
    let synced = sync_rates_from_url(sample_json);
    assert!(synced.is_ok());
    let info = synced.unwrap();
    assert_eq!(info.last_updated_unix, 1700000000);
    assert!(!info.is_seed);

    // Reset back to bundled seed
    let seed_json = include_str!("../src/seed_rates.json");
    assert!(plainpad_core::currency::update_rates(seed_json));
}
