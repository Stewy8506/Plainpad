use plainpad_core::extensions::{
    fetch_ai_models_for_provider, fetch_anthropic_models, fetch_ollama_models, fetch_openai_models,
    parse_anthropic_models_response, parse_ollama_models_response, parse_openai_models_response,
    CredentialStore,
};
use rusqlite::Connection;

#[test]
fn test_parse_openai_models_response_filtering_and_sorting() {
    let raw_json = r#"{
        "object": "list",
        "data": [
            { "id": "whisper-1", "object": "model" },
            { "id": "gpt-4o", "object": "model" },
            { "id": "text-embedding-3-small", "object": "model" },
            { "id": "o1-mini", "object": "model" },
            { "id": "gpt-3.5-turbo", "object": "model" },
            { "id": "o3-mini", "object": "model" },
            { "id": "dall-e-3", "object": "model" },
            { "id": "gpt-4-turbo", "object": "model" }
        ]
    }"#;

    let models = parse_openai_models_response(raw_json).expect("should parse valid OpenAI response");

    // Non-chat models like whisper-1, text-embedding-3-small, dall-e-3 must be filtered out
    let ids: Vec<&str> = models.iter().map(|m| m.id.as_str()).collect();
    assert_eq!(
        ids,
        vec!["gpt-3.5-turbo", "gpt-4-turbo", "gpt-4o", "o1-mini", "o3-mini"]
    );

    for m in &models {
        assert_eq!(m.provider, "openai");
        assert!(m.context_length.is_some());
    }

    // Verify context lengths
    let gpt4o = models.iter().find(|m| m.id == "gpt-4o").unwrap();
    assert_eq!(gpt4o.context_length, Some(128_000));

    let gpt35 = models.iter().find(|m| m.id == "gpt-3.5-turbo").unwrap();
    assert_eq!(gpt35.context_length, Some(4_096));
}

#[test]
fn test_parse_openai_error_response() {
    let err_json = r#"{
        "error": {
            "message": "Incorrect API key provided",
            "type": "invalid_request_error",
            "param": null,
            "code": "invalid_api_key"
        }
    }"#;

    let res = parse_openai_models_response(err_json);
    assert!(res.is_err());
    let err_msg = res.unwrap_err();
    assert!(err_msg.contains("Incorrect API key provided"));
}

#[test]
fn test_parse_anthropic_models_response() {
    let raw_json = r#"{
        "data": [
            {
                "type": "model",
                "id": "claude-3-5-sonnet-20241022",
                "display_name": "Claude 3.5 Sonnet",
                "created_at": "2024-10-22T00:00:00Z"
            },
            {
                "type": "model",
                "id": "claude-3-opus-20240229",
                "display_name": "Claude 3 Opus",
                "created_at": "2024-02-29T00:00:00Z"
            },
            {
                "type": "model",
                "id": "claude-3-5-haiku-20241022",
                "display_name": "Claude 3.5 Haiku",
                "created_at": "2024-10-22T00:00:00Z"
            }
        ],
        "has_more": false
    }"#;

    let models = parse_anthropic_models_response(raw_json).expect("should parse Anthropic response");
    assert_eq!(models.len(), 3);

    // Sorted alphabetically by id
    assert_eq!(models[0].id, "claude-3-5-haiku-20241022");
    assert_eq!(models[0].name, "Claude 3.5 Haiku");
    assert_eq!(models[0].provider, "anthropic");
    assert_eq!(models[0].context_length, Some(200_000));

    assert_eq!(models[1].id, "claude-3-5-sonnet-20241022");
    assert_eq!(models[1].name, "Claude 3.5 Sonnet");

    assert_eq!(models[2].id, "claude-3-opus-20240229");
    assert_eq!(models[2].name, "Claude 3 Opus");
}

#[test]
fn test_parse_anthropic_error_response() {
    let err_json = r#"{
        "type": "error",
        "error": {
            "type": "authentication_error",
            "message": "invalid x-api-key"
        }
    }"#;

    let res = parse_anthropic_models_response(err_json);
    assert!(res.is_err());
    assert!(res.unwrap_err().contains("invalid x-api-key"));
}

#[test]
fn test_parse_ollama_models_response() {
    let raw_json = r#"{
        "models": [
            {
                "name": "llama3:latest",
                "model": "llama3:latest",
                "size": 4661224676
            },
            {
                "name": "mistral:7b",
                "model": "mistral:7b",
                "size": 4109865159
            },
            {
                "name": "deepseek-r1:8b",
                "model": "deepseek-r1:8b",
                "size": 4900000000
            }
        ]
    }"#;

    let models = parse_ollama_models_response(raw_json).expect("should parse Ollama tags");
    assert_eq!(models.len(), 3);

    // Sorted alphabetically by name: deepseek-r1:8b, llama3:latest, mistral:7b
    assert_eq!(models[0].id, "deepseek-r1:8b");
    assert_eq!(models[0].name, "deepseek-r1:8b");
    assert_eq!(models[0].provider, "ollama");
    assert_eq!(models[0].context_length, None);

    assert_eq!(models[1].id, "llama3:latest");
    assert_eq!(models[2].id, "mistral:7b");
}

#[test]
fn test_missing_credentials_and_offline_ollama() {
    // Missing OpenAI key
    let res_openai = fetch_openai_models("");
    assert!(res_openai.is_err());
    assert!(res_openai.unwrap_err().contains("missing or not configured"));

    // Missing Anthropic key
    let res_anthropic = fetch_anthropic_models("   ");
    assert!(res_anthropic.is_err());
    assert!(res_anthropic.unwrap_err().contains("missing or not configured"));

    // Dead local port for Ollama returns descriptive offline error without crashing
    let res_ollama = fetch_ollama_models(Some("http://127.0.0.1:54321"));
    assert!(res_ollama.is_err());
    let err_str = res_ollama.unwrap_err();
    assert!(err_str.contains("Ollama is not running on http://127.0.0.1:54321"));
}

#[test]
fn test_fetch_ai_models_for_provider_with_store() {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute(
        "CREATE TABLE app_state (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
        [],
    )
    .unwrap();

    // Querying without configured keys
    let err_openai = fetch_ai_models_for_provider(&conn, "openai", None).unwrap_err();
    assert!(err_openai.contains("OpenAI API key is not configured in Settings"));

    let err_anthropic = fetch_ai_models_for_provider(&conn, "anthropic", None).unwrap_err();
    assert!(err_anthropic.contains("Anthropic API key is not configured in Settings"));

    // Store dummy key and test retrieval
    CredentialStore::store_api_key(&conn, "apikey_openai", "sk-mock-key-123456").unwrap();
    let retrieved = CredentialStore::get_api_key(&conn, "apikey_openai").unwrap();
    assert_eq!(retrieved, Some("sk-mock-key-123456".to_string()));

    // Unknown provider
    let err_unknown = fetch_ai_models_for_provider(&conn, "cohere", None).unwrap_err();
    assert!(err_unknown.contains("Unknown provider 'cohere'"));
}
