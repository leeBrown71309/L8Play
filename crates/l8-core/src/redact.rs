//! Removes secrets from text before it reaches a log file.
//!
//! API keys and OAuth tokens must never be logged. Everything written by the log layer passes
//! through [`redact_secrets`], which masks the common shapes they take in URLs, headers and JSON.

use std::sync::{Mutex, OnceLock};

const MASK: &str = "[REDACTED]";

/// Secret values learned at run time (the Google client secret, tokens): masked wherever they appear.
fn registered() -> &'static Mutex<Vec<String>> {
    static VALUES: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
    VALUES.get_or_init(|| Mutex::new(Vec::new()))
}

/// Makes `redact_secrets` mask this exact value from now on. Short values are ignored: masking
/// them would damage ordinary text.
pub fn register_secret(value: &str) {
    let value = value.trim();
    if value.len() < 8 {
        return;
    }
    if let Ok(mut values) = registered().lock()
        && !values.iter().any(|known| known == value)
    {
        values.push(value.to_owned());
    }
}

/// Prefixes of secrets whose shape is recognisable without knowing their value.
const TOKEN_PREFIXES: &[&str] = &["GOCSPX-", "ya29.", "1//"];

/// Parameter and field names whose values are always secret (case-insensitive).
const SECRET_NAMES: &[&str] = &[
    "access_token",
    "refresh_token",
    "id_token",
    "client_secret",
    "api_key",
    "apikey",
    "key",
    "token",
    "password",
    "code_verifier",
    "code",
];

/// Masks bearer tokens and the values of known secret parameters (`name=value`, `"name":"value"`).
pub fn redact_secrets(input: &str) -> String {
    let mut output = mask_bearer(input);
    for prefix in TOKEN_PREFIXES {
        output = mask_prefixed_token(&output, prefix);
    }
    if let Ok(values) = registered().lock() {
        for value in values.iter() {
            output = output.replace(value.as_str(), MASK);
        }
    }
    for name in SECRET_NAMES {
        output = mask_query_value(&output, name);
        output = mask_json_value(&output, name);
    }
    output
}

/// Masks `<prefix><token characters>` where the prefix starts a token (not the middle of a word).
fn mask_prefixed_token(input: &str, prefix: &str) -> String {
    let mut output = String::with_capacity(input.len());
    let mut cursor = 0;
    while let Some(found) = input[cursor..].find(prefix) {
        let start = cursor + found;
        let boundary = start == 0 || !is_name_char(input.as_bytes()[start - 1]);
        let value_start = start + prefix.len();
        let length = input[value_start..]
            .find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '~')))
            .unwrap_or(input.len() - value_start);
        if boundary && length >= 8 {
            output.push_str(&input[cursor..start]);
            output.push_str(MASK);
            cursor = value_start + length;
        } else {
            output.push_str(&input[cursor..value_start]);
            cursor = value_start;
        }
    }
    output.push_str(&input[cursor..]);
    output
}

fn mask_bearer(input: &str) -> String {
    let lower = input.to_ascii_lowercase();
    let mut output = String::with_capacity(input.len());
    let mut cursor = 0;
    while let Some(found) = lower[cursor..].find("bearer ") {
        let value_start = cursor + found + "bearer ".len();
        output.push_str(&input[cursor..value_start]);
        output.push_str(MASK);
        cursor = value_start + token_length(&input[value_start..]);
    }
    output.push_str(&input[cursor..]);
    output
}

fn mask_query_value(input: &str, name: &str) -> String {
    let lower = input.to_ascii_lowercase();
    let pattern = format!("{name}=");
    let mut output = String::with_capacity(input.len());
    let mut cursor = 0;
    while let Some(found) = lower[cursor..].find(&pattern) {
        let start = cursor + found;
        let is_whole_name = start == 0 || !is_name_char(input.as_bytes()[start - 1]);
        let value_start = start + pattern.len();
        output.push_str(&input[cursor..value_start]);
        if is_whole_name {
            output.push_str(MASK);
            cursor = value_start + token_length(&input[value_start..]);
        } else {
            cursor = value_start;
        }
    }
    output.push_str(&input[cursor..]);
    output
}

fn mask_json_value(input: &str, name: &str) -> String {
    let lower = input.to_ascii_lowercase();
    let pattern = format!("\"{name}\"");
    let mut output = String::with_capacity(input.len());
    let mut cursor = 0;
    while let Some(found) = lower[cursor..].find(&pattern) {
        let after_name = cursor + found + pattern.len();
        let rest = &input[after_name..];
        let trimmed = rest.trim_start_matches([' ', ':']);
        let skipped = rest.len() - trimmed.len();
        output.push_str(&input[cursor..after_name + skipped]);
        if rest[..skipped].contains(':') && trimmed.starts_with('"') {
            let closing = trimmed[1..].find('"').map_or(trimmed.len(), |end| end + 2);
            output.push('"');
            output.push_str(MASK);
            output.push('"');
            cursor = after_name + skipped + closing;
        } else {
            cursor = after_name + skipped;
        }
    }
    output.push_str(&input[cursor..]);
    output
}

fn token_length(value: &str) -> usize {
    value.find(|c: char| c.is_whitespace() || matches!(c, '&' | '"' | '\'' | ',' | ';' | ')')).unwrap_or(value.len())
}

fn is_name_char(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn masks_bearer_header() {
        assert_eq!(redact_secrets("Authorization: Bearer abc.def-123 sent"), "Authorization: Bearer [REDACTED] sent");
    }

    #[test]
    fn masks_query_parameters() {
        let url = "https://oauth2.example/token?code=4/0Ab&client_secret=s3cr3t&state=xyz";
        assert_eq!(
            redact_secrets(url),
            "https://oauth2.example/token?code=[REDACTED]&client_secret=[REDACTED]&state=xyz"
        );
    }

    #[test]
    fn masks_the_steam_web_api_key_in_urls_and_text() {
        let key = "0123456789ABCDEF0123456789ABCDEF";
        let url = format!("https://api.steampowered.com/IPlayerService/GetOwnedGames/v1/?key={key}&steamid=7656");
        assert_eq!(
            redact_secrets(&url),
            "https://api.steampowered.com/IPlayerService/GetOwnedGames/v1/?key=[REDACTED]&steamid=7656"
        );
        register_secret(key);
        let loose = format!("failed for {key} (request ?KEY={key})");
        assert!(!redact_secrets(&loose).contains(key));
    }

    #[test]
    fn masks_json_fields() {
        let body = r#"{"access_token": "ya29.a0", "expires_in": 3599, "refresh_token":"1//0g"}"#;
        assert_eq!(
            redact_secrets(body),
            r#"{"access_token": "[REDACTED]", "expires_in": 3599, "refresh_token":"[REDACTED]"}"#
        );
    }

    #[test]
    fn leaves_similar_names_alone() {
        assert_eq!(redact_secrets("monkey=1 hotkey=2 barcode=3"), "monkey=1 hotkey=2 barcode=3");
    }

    #[test]
    fn masks_google_token_shapes_without_a_parameter_name() {
        let text = "client GOCSPX-abcDEF123_xyz-456 token ya29.a0AfH6SMB-xyz refresh 1//0gAbCdEfGhIjK done";
        let masked = redact_secrets(text);
        assert_eq!(masked, "client [REDACTED] token [REDACTED] refresh [REDACTED] done");
        assert_eq!(redact_secrets("path C:/a1//b and ya29. alone"), "path C:/a1//b and ya29. alone");
    }

    #[test]
    fn masks_registered_values_everywhere() {
        register_secret("my-embedded-secret-value");
        register_secret("tiny");
        assert_eq!(
            redact_secrets("sent my-embedded-secret-value in a message about a tiny thing"),
            "sent [REDACTED] in a message about a tiny thing"
        );
    }

    #[test]
    fn leaves_plain_text_alone() {
        let text = "Snapshot created for Sekiro (9.8 MB)";
        assert_eq!(redact_secrets(text), text);
    }
}
