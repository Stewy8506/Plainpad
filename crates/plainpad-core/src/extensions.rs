use crate::store::StoreError;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

/// Metadata for an API key stored in the secure credential store.
/// Raw secrets are never included.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ApiKeyMeta {
    pub key_name: String,
    pub is_configured: bool,
    pub masked_preview: String,
}

/// Normalizes key names to always carry the `apikey_` prefix.
pub fn normalize_key_name(key_name: &str) -> String {
    let trimmed = key_name.trim();
    if trimmed.starts_with("apikey_") {
        trimmed.to_string()
    } else {
        format!("apikey_{trimmed}")
    }
}

/// Masks an API key for safe display in settings/preferences.
/// e.g. "sk-1234567890abcdef" -> "sk-...cdef", "ghp_123456789" -> "ghp...6789"
pub fn mask_api_key(raw: &str) -> String {
    let s = raw.trim();
    if s.is_empty() {
        return String::new();
    }
    if s.len() <= 6 {
        return "...***".to_string();
    }
    let last_4 = &s[s.len().saturating_sub(4)..];
    if s.starts_with("sk-") {
        format!("sk-...{last_4}")
    } else if s.len() >= 8 {
        let prefix_len = s.chars().take(3).count();
        let prefix: String = s.chars().take(prefix_len).collect();
        format!("{prefix}...{last_4}")
    } else {
        format!("...{last_4}")
    }
}

/// Credential storage using the SQLite `app_state` table.
pub struct CredentialStore;

impl CredentialStore {
    /// Stores or updates an API key in `app_state`.
    pub fn store_api_key(
        conn: &Connection,
        key_name: &str,
        key_value: &str,
    ) -> Result<(), StoreError> {
        let normalized = normalize_key_name(key_name);
        conn.execute(
            "INSERT INTO app_state (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = ?2",
            rusqlite::params![normalized, key_value],
        )?;
        Ok(())
    }

    /// Retrieves the raw API key from `app_state` for native dispatch only.
    pub fn get_api_key(
        conn: &Connection,
        key_name: &str,
    ) -> Result<Option<String>, StoreError> {
        let normalized = normalize_key_name(key_name);
        let mut stmt = conn.prepare("SELECT value FROM app_state WHERE key = ?1")?;
        let mut rows = stmt.query([normalized])?;
        if let Some(row) = rows.next()? {
            let val: String = row.get(0)?;
            Ok(Some(val))
        } else {
            Ok(None)
        }
    }

    /// Deletes an API key from `app_state`.
    pub fn delete_api_key(conn: &Connection, key_name: &str) -> Result<bool, StoreError> {
        let normalized = normalize_key_name(key_name);
        let rows = conn.execute(
            "DELETE FROM app_state WHERE key = ?1",
            rusqlite::params![normalized],
        )?;
        Ok(rows > 0)
    }

    /// Lists all configured API keys without revealing raw secrets.
    pub fn list_api_keys(conn: &Connection) -> Result<Vec<ApiKeyMeta>, StoreError> {
        let mut stmt = conn.prepare(
            "SELECT key, value FROM app_state WHERE key LIKE 'apikey_%' ORDER BY key ASC",
        )?;
        let rows = stmt.query_map([], |row| {
            let key_name: String = row.get(0)?;
            let raw_val: String = row.get(1)?;
            let is_configured = !raw_val.trim().is_empty();
            let masked_preview = mask_api_key(&raw_val);
            Ok(ApiKeyMeta {
                key_name,
                is_configured,
                masked_preview,
            })
        })?;

        let mut list = Vec::new();
        for item in rows {
            list.push(item?);
        }
        Ok(list)
    }
}

/// Security errors when evaluating network endpoints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SecurityError {
    InvalidUrl(String),
    InsecureProtocol(String),
    UnauthorizedEndpoint(String),
}

impl fmt::Display for SecurityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidUrl(msg) => write!(f, "Invalid URL: {msg}"),
            Self::InsecureProtocol(msg) => write!(f, "Insecure protocol: {msg}"),
            Self::UnauthorizedEndpoint(msg) => write!(f, "Unauthorized endpoint: {msg}"),
        }
    }
}

impl std::error::Error for SecurityError {}

/// Endpoint security verifier preventing extensions from accessing unapproved domains
/// or making insecure cleartext external requests.
pub struct EndpointSecurity;

impl EndpointSecurity {
    /// Validates an endpoint URL against the extension's approved endpoints list.
    /// Requirements:
    /// 1. Protocol must be HTTPS, with an exception for local HTTP (localhost / 127.0.0.1).
    /// 2. Must match at least one approved origin/prefix with proper path boundaries.
    pub fn validate_endpoint(
        url: &str,
        approved_endpoints: &[String],
    ) -> Result<(), SecurityError> {
        let trimmed = url.trim();
        if trimmed.is_empty() {
            return Err(SecurityError::InvalidUrl("Endpoint URL is empty".to_string()));
        }

        // Protocol check
        if trimmed.starts_with("http://") {
            let without_proto = &trimmed["http://".len()..];
            let is_localhost = without_proto.starts_with("localhost")
                || without_proto.starts_with("127.0.0.1");
            if is_localhost {
                let rest = if without_proto.starts_with("localhost") {
                    &without_proto["localhost".len()..]
                } else {
                    &without_proto["127.0.0.1".len()..]
                };
                if rest.is_empty() || rest.starts_with(':') || rest.starts_with('/') {
                    // Valid localhost HTTP
                } else {
                    return Err(SecurityError::InsecureProtocol(format!(
                        "Cleartext HTTP is only permitted for localhost/127.0.0.1, got: {trimmed}"
                    )));
                }
            } else {
                return Err(SecurityError::InsecureProtocol(format!(
                    "External non-HTTPS protocols are prohibited: {trimmed}"
                )));
            }
        } else if !trimmed.starts_with("https://") {
            return Err(SecurityError::InsecureProtocol(format!(
                "Unsupported protocol (only https:// or local http:// allowed): {trimmed}"
            )));
        }

        if approved_endpoints.is_empty() {
            return Err(SecurityError::UnauthorizedEndpoint(format!(
                "Endpoint not approved: {trimmed} (no approved endpoints declared)"
            )));
        }

        // Match against approved endpoints list
        let matched = approved_endpoints.iter().any(|approved| {
            let app_trimmed = approved.trim();
            if app_trimmed.is_empty() {
                return false;
            }

            if trimmed == app_trimmed {
                return true;
            }

            // Subpath matching: url starts with approved prefix and has a boundary
            if trimmed.starts_with(app_trimmed) {
                if app_trimmed.ends_with('/') {
                    return true;
                }
                let remainder = &trimmed[app_trimmed.len()..];
                if remainder.starts_with('/')
                    || remainder.starts_with('?')
                    || remainder.starts_with('#')
                {
                    return true;
                }
            }

            // Also check if approved endpoint has trailing slash and url does not
            if app_trimmed.ends_with('/') && trimmed == app_trimmed.trim_end_matches('/') {
                return true;
            }

            false
        });

        if matched {
            Ok(())
        } else {
            Err(SecurityError::UnauthorizedEndpoint(format!(
                "Endpoint '{trimmed}' is not authorized. Approved endpoints: {approved_endpoints:?}"
            )))
        }
    }
}

/// Data access scope controlling note contents passed to extensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DataScope {
    None,
    Line,
    Full,
}

impl std::str::FromStr for DataScope {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "none" => Ok(DataScope::None),
            "line" => Ok(DataScope::Line),
            "full" => Ok(DataScope::Full),
            other => Err(format!("Unknown data scope: {other}")),
        }
    }
}

/// Filters note content according to declared `DataScope`.
/// Returns `(line_text, full_note)`.
pub fn filter_content(scope: DataScope, line_text: &str, full_note: &str) -> (String, String) {
    match scope {
        DataScope::None => (String::new(), String::new()),
        DataScope::Line => (line_text.to_string(), String::new()),
        DataScope::Full => (line_text.to_string(), full_note.to_string()),
    }
}

/// Request payload for native API calls from extensions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallApiRequest {
    pub api_key_name: String,
    pub url: String,
    pub method: String,
    #[serde(default)]
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    #[serde(default)]
    pub approved_endpoints: Vec<String>,
}

/// Response payload from native API calls.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CallApiResponse {
    pub success: bool,
    #[serde(rename = "statusCode", default)]
    pub status_code: u16,
    pub data: Option<String>,
    pub error: Option<String>,
}

/// Parses JSON stringified headers or maps into `HashMap<String, String>`.
pub fn parse_headers(raw: &str) -> HashMap<String, String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return HashMap::new();
    }
    if let Ok(map) = serde_json::from_str::<HashMap<String, String>>(trimmed) {
        return map;
    }
    if let Ok(map) = serde_json::from_str::<HashMap<String, serde_json::Value>>(trimmed) {
        let mut result = HashMap::new();
        for (k, v) in map {
            match v {
                serde_json::Value::String(s) => {
                    result.insert(k, s);
                }
                other => {
                    result.insert(k, other.to_string());
                }
            }
        }
        return result;
    }
    HashMap::new()
}

/// Substitutes `{{API_KEY}}` placeholders with the provided secret key.
pub fn substitute_api_key(template: &str, key_value: &str) -> String {
    template.replace("{{API_KEY}}", key_value)
}

/// Executes a native API call on behalf of an extension after validating endpoints,
/// retrieving credentials from SQLite, and substituting `{{API_KEY}}` placeholders.
pub fn call_api(
    conn: &Connection,
    request: CallApiRequest,
) -> CallApiResponse {
    // 1. Validate endpoint against approved list before doing anything
    if let Err(sec_err) = EndpointSecurity::validate_endpoint(&request.url, &request.approved_endpoints) {
        return CallApiResponse {
            success: false,
            status_code: 0,
            data: None,
            error: Some(sec_err.to_string()),
        };
    }

    // 2. Fetch secret key if required
    let mut secret_key = String::new();
    let key_name = request.api_key_name.trim();
    if !key_name.is_empty() && key_name != "none" {
        match CredentialStore::get_api_key(conn, key_name) {
            Ok(Some(k)) if !k.trim().is_empty() => {
                secret_key = k;
            }
            Ok(_) => {
                return CallApiResponse {
                    success: false,
                    status_code: 0,
                    data: None,
                    error: Some(format!("API key '{key_name}' not configured in Settings")),
                };
            }
            Err(e) => {
                return CallApiResponse {
                    success: false,
                    status_code: 0,
                    data: None,
                    error: Some(format!("Failed to retrieve API key '{key_name}': {e}")),
                };
            }
        }
    }

    // 3. Substitute {{API_KEY}} placeholders across URL, headers, and body
    let final_url = substitute_api_key(&request.url, &secret_key);
    let mut final_headers = HashMap::new();
    for (k, v) in request.headers {
        final_headers.insert(k, substitute_api_key(&v, &secret_key));
    }
    let final_body = request.body.map(|b| substitute_api_key(&b, &secret_key));

    // 4. Construct HTTP client and request
    let client = match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return CallApiResponse {
                success: false,
                status_code: 0,
                data: None,
                error: Some(format!("Failed to build HTTP client: {e}")),
            };
        }
    };

    let method_upper = request.method.trim().to_uppercase();
    let req_builder = match method_upper.as_str() {
        "GET" => client.get(&final_url),
        "POST" => client.post(&final_url),
        "PUT" => client.put(&final_url),
        "DELETE" => client.delete(&final_url),
        "PATCH" => client.patch(&final_url),
        "HEAD" => client.head(&final_url),
        other => {
            return CallApiResponse {
                success: false,
                status_code: 0,
                data: None,
                error: Some(format!("Unsupported HTTP method: {other}")),
            };
        }
    };

    let mut req_builder = req_builder;
    for (header_key, header_val) in final_headers {
        req_builder = req_builder.header(header_key, header_val);
    }

    if let Some(body_content) = final_body {
        req_builder = req_builder.body(body_content);
    }

    // 5. Send request
    match req_builder.send() {
        Ok(resp) => {
            let status = resp.status().as_u16();
            let is_success = resp.status().is_success();
            let body_text = resp.text().unwrap_or_default();
            CallApiResponse {
                success: is_success,
                status_code: status,
                data: Some(body_text),
                error: if is_success {
                    None
                } else {
                    Some(format!("HTTP status {status}"))
                },
            }
        }
        Err(err) => CallApiResponse {
            success: false,
            status_code: 0,
            data: None,
            error: Some(err.to_string()),
        },
    }
}

/// Retrieves an extension preference from `app_state`.
/// Key format: `ext_pref_{extension_name}_{key}`
pub fn get_extension_preference(
    conn: &Connection,
    extension_name: &str,
    key: &str,
) -> Result<Option<String>, StoreError> {
    let state_key = format!("ext_pref_{}_{}", extension_name.trim(), key.trim());
    let mut stmt = conn.prepare("SELECT value FROM app_state WHERE key = ?1")?;
    let mut rows = stmt.query([state_key])?;
    if let Some(row) = rows.next()? {
        let val: String = row.get(0)?;
        Ok(Some(val))
    } else {
        Ok(None)
    }
}

/// Sets an extension preference in `app_state`.
/// Key format: `ext_pref_{extension_name}_{key}`
pub fn set_extension_preference(
    conn: &Connection,
    extension_name: &str,
    key: &str,
    value: &str,
) -> Result<(), StoreError> {
    let state_key = format!("ext_pref_{}_{}", extension_name.trim(), key.trim());
    conn.execute(
        "INSERT INTO app_state (key, value) VALUES (?1, ?2) ON CONFLICT(key) DO UPDATE SET value = ?2",
        rusqlite::params![state_key, value],
    )?;
    Ok(())
}

/// Standardized AI model descriptor across cloud and local providers.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AiModel {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub context_length: Option<u32>,
}

/// Parses raw JSON response from OpenAI `/v1/models`.
pub fn parse_openai_models_response(json_str: &str) -> Result<Vec<AiModel>, String> {
    let v: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| format!("Failed to parse OpenAI models response JSON: {e}"))?;

    if let Some(err_obj) = v.get("error") {
        let msg = err_obj
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("Unknown OpenAI error");
        return Err(format!("OpenAI error: {msg}"));
    }

    let data_array = v
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| "Invalid OpenAI response format: missing 'data' array".to_string())?;

    let mut models = Vec::new();
    for item in data_array {
        if let Some(id_str) = item.get("id").and_then(|i| i.as_str()) {
            let trimmed_id = id_str.trim();
            // Filter relevant chat/reasoning models
            let is_relevant = trimmed_id.starts_with("gpt-4")
                || trimmed_id.starts_with("gpt-3.5")
                || trimmed_id.starts_with("o1")
                || trimmed_id.starts_with("o3")
                || trimmed_id.starts_with("chatgpt");

            if is_relevant {
                let context_len = if trimmed_id.starts_with("gpt-4o")
                    || trimmed_id.starts_with("gpt-4-turbo")
                    || trimmed_id.starts_with("o1")
                    || trimmed_id.starts_with("o3")
                {
                    Some(128_000)
                } else if trimmed_id.starts_with("gpt-3.5-turbo-16k") {
                    Some(16_384)
                } else if trimmed_id.starts_with("gpt-3.5") {
                    Some(4_096)
                } else {
                    Some(128_000)
                };

                models.push(AiModel {
                    id: trimmed_id.to_string(),
                    name: trimmed_id.to_string(),
                    provider: "openai".to_string(),
                    context_length: context_len,
                });
            }
        }
    }

    models.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(models)
}

/// Fetches OpenAI models from https://api.openai.com/v1/models.
pub fn fetch_openai_models(api_key: &str) -> Result<Vec<AiModel>, String> {
    let key = api_key.trim();
    if key.is_empty() {
        return Err("OpenAI API key is missing or not configured".to_string());
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {e}"))?;

    let resp = client
        .get("https://api.openai.com/v1/models")
        .header("Authorization", format!("Bearer {key}"))
        .send()
        .map_err(|e| format!("Network error connecting to OpenAI: {e}"))?;

    let body = resp
        .text()
        .map_err(|e| format!("Failed to read response body: {e}"))?;

    parse_openai_models_response(&body)
}

/// Parses raw JSON response from Anthropic `/v1/models`.
pub fn parse_anthropic_models_response(json_str: &str) -> Result<Vec<AiModel>, String> {
    let v: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| format!("Failed to parse Anthropic models response JSON: {e}"))?;

    if let Some(err_obj) = v.get("error") {
        let msg = err_obj
            .get("message")
            .and_then(|m| m.as_str())
            .unwrap_or("Unknown Anthropic error");
        return Err(format!("Anthropic error: {msg}"));
    }

    let data_array = v
        .get("data")
        .and_then(|d| d.as_array())
        .ok_or_else(|| "Invalid Anthropic response format: missing 'data' array".to_string())?;

    let mut models = Vec::new();
    for item in data_array {
        if let Some(id_str) = item.get("id").and_then(|i| i.as_str()) {
            let trimmed_id = id_str.trim();
            let display_name = item
                .get("display_name")
                .and_then(|d| d.as_str())
                .unwrap_or(trimmed_id)
                .trim();

            models.push(AiModel {
                id: trimmed_id.to_string(),
                name: if display_name.is_empty() {
                    trimmed_id.to_string()
                } else {
                    display_name.to_string()
                },
                provider: "anthropic".to_string(),
                context_length: Some(200_000),
            });
        }
    }

    models.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(models)
}

/// Fetches Anthropic models from https://api.anthropic.com/v1/models?limit=1000.
pub fn fetch_anthropic_models(api_key: &str) -> Result<Vec<AiModel>, String> {
    let key = api_key.trim();
    if key.is_empty() {
        return Err("Anthropic API key is missing or not configured".to_string());
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {e}"))?;

    let resp = client
        .get("https://api.anthropic.com/v1/models?limit=1000")
        .header("x-api-key", key)
        .header("anthropic-version", "2023-06-01")
        .send()
        .map_err(|e| format!("Network error connecting to Anthropic: {e}"))?;

    let body = resp
        .text()
        .map_err(|e| format!("Failed to read response body: {e}"))?;

    parse_anthropic_models_response(&body)
}

/// Parses raw JSON response from Ollama `/api/tags`.
pub fn parse_ollama_models_response(json_str: &str) -> Result<Vec<AiModel>, String> {
    let v: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| format!("Failed to parse Ollama tags response JSON: {e}"))?;

    if let Some(err_obj) = v.get("error") {
        let msg = err_obj.as_str().unwrap_or("Unknown Ollama error");
        return Err(format!("Ollama error: {msg}"));
    }

    let models_array = v
        .get("models")
        .and_then(|m| m.as_array())
        .ok_or_else(|| "Invalid Ollama response format: missing 'models' array".to_string())?;

    let mut models = Vec::new();
    for item in models_array {
        let name_str = item
            .get("name")
            .or_else(|| item.get("model"))
            .and_then(|n| n.as_str());

        if let Some(name) = name_str {
            let trimmed = name.trim();
            if !trimmed.is_empty() {
                models.push(AiModel {
                    id: trimmed.to_string(),
                    name: trimmed.to_string(),
                    provider: "ollama".to_string(),
                    context_length: None,
                });
            }
        }
    }

    models.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(models)
}

/// Fetches local Ollama models from `{base_url}/api/tags` (default http://localhost:11434).
pub fn fetch_ollama_models(base_url: Option<&str>) -> Result<Vec<AiModel>, String> {
    let default_host = "http://localhost:11434";
    let base = match base_url {
        Some(u) if !u.trim().is_empty() => u.trim(),
        _ => default_host,
    };
    let clean_base = base.trim_end_matches('/');
    let tags_url = format!("{clean_base}/api/tags");

    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| format!("Failed to build HTTP client: {e}"))?;

    let resp = client
        .get(&tags_url)
        .send()
        .map_err(|e| format!("Ollama is not running on {clean_base}: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!(
            "Ollama is not running on {clean_base} (HTTP {}): {}",
            resp.status(),
            resp.text().unwrap_or_default()
        ));
    }

    let body = resp
        .text()
        .map_err(|e| format!("Failed to read Ollama response: {e}"))?;

    parse_ollama_models_response(&body)
}

/// Fetches AI models for a specified provider using configured credentials from SQLite.
pub fn fetch_ai_models_for_provider(
    conn: &Connection,
    provider: &str,
    custom_url: Option<&str>,
) -> Result<Vec<AiModel>, String> {
    let p = provider.trim().to_lowercase();
    match p.as_str() {
        "openai" => {
            let key = CredentialStore::get_api_key(conn, "apikey_openai")
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "OpenAI API key is not configured in Settings".to_string())?;
            if key.trim().is_empty() {
                return Err("OpenAI API key is not configured in Settings".to_string());
            }
            fetch_openai_models(&key)
        }
        "anthropic" => {
            let key = CredentialStore::get_api_key(conn, "apikey_anthropic")
                .map_err(|e| e.to_string())?
                .ok_or_else(|| "Anthropic API key is not configured in Settings".to_string())?;
            if key.trim().is_empty() {
                return Err("Anthropic API key is not configured in Settings".to_string());
            }
            fetch_anthropic_models(&key)
        }
        "ollama" => {
            let host_from_db = conn
                .query_row(
                    "SELECT value FROM app_state WHERE key = 'ollama_host'",
                    [],
                    |row| row.get::<_, String>(0),
                )
                .ok();
            let effective_url = custom_url
                .or(host_from_db.as_deref())
                .filter(|s| !s.trim().is_empty());
            fetch_ollama_models(effective_url)
        }
        "all" => {
            let mut all_models = Vec::new();
            if let Ok(m) = fetch_ai_models_for_provider(conn, "openai", None) {
                all_models.extend(m);
            }
            if let Ok(m) = fetch_ai_models_for_provider(conn, "anthropic", None) {
                all_models.extend(m);
            }
            if let Ok(m) = fetch_ai_models_for_provider(conn, "ollama", custom_url) {
                all_models.extend(m);
            }
            Ok(all_models)
        }
        other => Err(format!(
            "Unknown provider '{other}'. Supported providers: openai, anthropic, ollama, all"
        )),
    }
}
