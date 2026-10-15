use plainpad_core::extensions::{
    filter_content, mask_api_key, parse_headers, substitute_api_key, CallApiRequest, DataScope,
    EndpointSecurity, SecurityError,
};
use plainpad_core::store::SqliteStore;
use std::collections::HashMap;

#[test]
fn test_credential_store_crud_and_masking() {
    let store = SqliteStore::open_in_memory().expect("in-memory db");

    // 1. Initially empty
    let keys = store.list_api_keys().expect("list api keys");
    assert!(keys.is_empty());

    // 2. Store API keys (both with and without prefix)
    store
        .store_api_key("openai", "sk-proj-1234567890abcdef")
        .expect("store openai key");
    store
        .store_api_key("apikey_anthropic", "sk-ant-api03-abcdef1234")
        .expect("store anthropic key");

    // 3. Raw retrieval works for native bridge
    assert_eq!(
        store.get_api_key("openai").expect("get openai"),
        Some("sk-proj-1234567890abcdef".to_string())
    );
    assert_eq!(
        store.get_api_key("apikey_openai").expect("get openai with prefix"),
        Some("sk-proj-1234567890abcdef".to_string())
    );
    assert_eq!(
        store.get_api_key("anthropic").expect("get anthropic"),
        Some("sk-ant-api03-abcdef1234".to_string())
    );

    // 4. List API keys returns masked previews and never leaks raw keys
    let listed = store.list_api_keys().expect("list api keys");
    assert_eq!(listed.len(), 2);

    let anthropic_meta = listed
        .iter()
        .find(|m| m.key_name == "apikey_anthropic")
        .expect("anthropic found");
    assert!(anthropic_meta.is_configured);
    assert_eq!(anthropic_meta.masked_preview, "sk-...1234");
    assert!(!anthropic_meta.masked_preview.contains("abcdef"));

    let openai_meta = listed
        .iter()
        .find(|m| m.key_name == "apikey_openai")
        .expect("openai found");
    assert!(openai_meta.is_configured);
    assert_eq!(openai_meta.masked_preview, "sk-...cdef");
    assert!(!openai_meta.masked_preview.contains("1234567890"));

    // 5. Delete API key
    let deleted = store.delete_api_key("openai").expect("delete openai");
    assert!(deleted);
    assert_eq!(store.get_api_key("openai").expect("get deleted"), None);

    let listed_after = store.list_api_keys().expect("list after delete");
    assert_eq!(listed_after.len(), 1);
    assert_eq!(listed_after[0].key_name, "apikey_anthropic");
}

#[test]
fn test_mask_api_key_formats() {
    assert_eq!(mask_api_key(""), "");
    assert_eq!(mask_api_key("abc"), "...***");
    assert_eq!(mask_api_key("123456"), "...***");
    assert_eq!(mask_api_key("sk-1234567890abcdef"), "sk-...cdef");
    assert_eq!(mask_api_key("sk-ant-api03-9999"), "sk-...9999");
    assert_eq!(mask_api_key("ghp_1234567890"), "ghp...7890");
    assert_eq!(mask_api_key("ollama_secret"), "oll...cret");
}

#[test]
fn test_endpoint_security_rules() {
    let approved = vec![
        "https://api.openai.com".to_string(),
        "https://api.anthropic.com/v1/messages".to_string(),
        "http://localhost:11434".to_string(),
        "http://127.0.0.1:8000".to_string(),
    ];

    // Approved HTTPS endpoints and subpaths pass
    assert!(EndpointSecurity::validate_endpoint("https://api.openai.com/v1/chat/completions", &approved).is_ok());
    assert!(EndpointSecurity::validate_endpoint("https://api.openai.com", &approved).is_ok());
    assert!(EndpointSecurity::validate_endpoint("https://api.openai.com/", &approved).is_ok());
    assert!(EndpointSecurity::validate_endpoint("https://api.anthropic.com/v1/messages", &approved).is_ok());

    // Host spoofing attempts are rejected (boundary check)
    match EndpointSecurity::validate_endpoint("https://api.openai.com.attacker.com/leak", &approved) {
        Err(SecurityError::UnauthorizedEndpoint(_)) => (),
        other => panic!("Expected UnauthorizedEndpoint error, got {:?}", other),
    }

    // Unapproved external origins are rejected
    match EndpointSecurity::validate_endpoint("https://malicious-site.com/steal", &approved) {
        Err(SecurityError::UnauthorizedEndpoint(_)) => (),
        other => panic!("Expected UnauthorizedEndpoint error, got {:?}", other),
    }

    // Localhost HTTP is permitted
    assert!(EndpointSecurity::validate_endpoint("http://localhost:11434/api/tags", &approved).is_ok());
    assert!(EndpointSecurity::validate_endpoint("http://127.0.0.1:8000/v1/models", &approved).is_ok());

    // External non-HTTPS HTTP is strictly rejected
    match EndpointSecurity::validate_endpoint("http://api.openai.com/v1/chat", &approved) {
        Err(SecurityError::InsecureProtocol(_)) => (),
        other => panic!("Expected InsecureProtocol error, got {:?}", other),
    }

    // Invalid / empty URL is rejected
    match EndpointSecurity::validate_endpoint("", &approved) {
        Err(SecurityError::InvalidUrl(_)) => (),
        other => panic!("Expected InvalidUrl error, got {:?}", other),
    }

    // Empty approved endpoints rejects everything
    match EndpointSecurity::validate_endpoint("https://api.openai.com", &[]) {
        Err(SecurityError::UnauthorizedEndpoint(_)) => (),
        other => panic!("Expected UnauthorizedEndpoint error, got {:?}", other),
    }
}

#[test]
fn test_placeholder_substitution() {
    let secret = "sk-test-secret-key-9876";

    // Header replacement
    let header_template = "Bearer {{API_KEY}}";
    assert_eq!(
        substitute_api_key(header_template, secret),
        "Bearer sk-test-secret-key-9876"
    );

    // URL replacement
    let url_template = "https://api.example.com/v1?token={{API_KEY}}&format=json";
    assert_eq!(
        substitute_api_key(url_template, secret),
        "https://api.example.com/v1?token=sk-test-secret-key-9876&format=json"
    );

    // Body replacement
    let body_template = r#"{"model":"gpt-4o","key":"{{API_KEY}}"}"#;
    assert_eq!(
        substitute_api_key(body_template, secret),
        r#"{"model":"gpt-4o","key":"sk-test-secret-key-9876"}"#
    );
}

#[test]
fn test_data_scope_filtering() {
    let line = "Buy milk and eggs";
    let full = "Header line\nBuy milk and eggs\nFooter line";

    // None
    let (l_none, f_none) = filter_content(DataScope::None, line, full);
    assert_eq!(l_none, "");
    assert_eq!(f_none, "");

    // Line
    let (l_line, f_line) = filter_content(DataScope::Line, line, full);
    assert_eq!(l_line, "Buy milk and eggs");
    assert_eq!(f_line, "");

    // Full
    let (l_full, f_full) = filter_content(DataScope::Full, line, full);
    assert_eq!(l_full, "Buy milk and eggs");
    assert_eq!(f_full, "Header line\nBuy milk and eggs\nFooter line");

    // FromStr
    assert_eq!("none".parse::<DataScope>().unwrap(), DataScope::None);
    assert_eq!("line".parse::<DataScope>().unwrap(), DataScope::Line);
    assert_eq!("full".parse::<DataScope>().unwrap(), DataScope::Full);
    assert!("invalid".parse::<DataScope>().is_err());
}

#[test]
fn test_call_api_security_and_missing_credentials() {
    let store = SqliteStore::open_in_memory().expect("in-memory db");

    // 1. Unauthorized endpoint should fail immediately before network dispatch
    let unauth_req = CallApiRequest {
        api_key_name: "openai".to_string(),
        url: "https://evil.attacker.com/steal".to_string(),
        method: "POST".to_string(),
        headers: HashMap::new(),
        body: None,
        approved_endpoints: vec!["https://api.openai.com".to_string()],
    };
    let unauth_res = store.call_api(unauth_req).expect("call_api invoked");
    assert!(!unauth_res.success);
    assert!(unauth_res.error.unwrap().contains("is not authorized"));

    // 2. Unconfigured API key returns descriptive Settings error
    let missing_key_req = CallApiRequest {
        api_key_name: "apikey_openai".to_string(),
        url: "https://api.openai.com/v1/chat/completions".to_string(),
        method: "POST".to_string(),
        headers: HashMap::new(),
        body: None,
        approved_endpoints: vec!["https://api.openai.com".to_string()],
    };
    let missing_key_res = store.call_api(missing_key_req).expect("call_api invoked");
    assert!(!missing_key_res.success);
    assert_eq!(
        missing_key_res.error.unwrap(),
        "API key 'apikey_openai' not configured in Settings"
    );

    // 3. Cleartext external protocol rejected
    let insecure_req = CallApiRequest {
        api_key_name: "openai".to_string(),
        url: "http://api.openai.com/v1/chat".to_string(),
        method: "GET".to_string(),
        headers: HashMap::new(),
        body: None,
        approved_endpoints: vec!["http://api.openai.com".to_string()],
    };
    let insecure_res = store.call_api(insecure_req).expect("call_api invoked");
    assert!(!insecure_res.success);
    assert!(insecure_res.error.unwrap().contains("External non-HTTPS protocols are prohibited"));
}

#[test]
fn test_extension_preferences_persistence() {
    let store = SqliteStore::open_in_memory().expect("in-memory db");

    // Initially unset
    assert_eq!(
        store.get_extension_preference("openai", "model").unwrap(),
        None
    );

    // Set preference
    store
        .set_extension_preference("openai", "model", "gpt-4o")
        .expect("set pref");
    assert_eq!(
        store.get_extension_preference("openai", "model").unwrap(),
        Some("gpt-4o".to_string())
    );

    // Update preference
    store
        .set_extension_preference("openai", "model", "gpt-4o-mini")
        .expect("update pref");
    assert_eq!(
        store.get_extension_preference("openai", "model").unwrap(),
        Some("gpt-4o-mini".to_string())
    );

    // Isolated per extension
    assert_eq!(
        store.get_extension_preference("anthropic", "model").unwrap(),
        None
    );
}

#[test]
fn test_parse_headers() {
    let json_headers = r#"{"Content-Type": "application/json", "Authorization": "Bearer {{API_KEY}}"}"#;
    let parsed = parse_headers(json_headers);
    assert_eq!(parsed.get("Content-Type").unwrap(), "application/json");
    assert_eq!(parsed.get("Authorization").unwrap(), "Bearer {{API_KEY}}");

    let empty = parse_headers("");
    assert!(empty.is_empty());
}
