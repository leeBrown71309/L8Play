//! Makes the project's Google OAuth client available at compile time without ever writing it in
//! the source: `L8_GOOGLE_CLIENT_ID` and `L8_GOOGLE_CLIENT_SECRET` come from the environment (CI
//! secrets) or, for local builds, from an untracked `.env` file at the repository root. When they
//! are absent the build still succeeds and the app falls back to a user-provided client.

use std::path::Path;

const KEYS: [&str; 2] = ["L8_GOOGLE_CLIENT_ID", "L8_GOOGLE_CLIENT_SECRET"];

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    for key in KEYS {
        println!("cargo:rerun-if-env-changed={key}");
    }
    let env_file = Path::new(env!("CARGO_MANIFEST_DIR")).join("..").join("..").join(".env");
    println!("cargo:rerun-if-changed={}", env_file.display());
    let from_file = std::fs::read_to_string(&env_file).unwrap_or_default();
    for key in KEYS {
        // A real environment variable wins over the file.
        if std::env::var(key).is_ok_and(|value| !value.trim().is_empty()) {
            continue;
        }
        if let Some(value) = value_from_env_file(&from_file, key) {
            println!("cargo:rustc-env={key}={value}");
        }
    }
}

/// `KEY=value` (optionally quoted) from simple dotenv text; comments and blank lines are ignored.
fn value_from_env_file(text: &str, key: &str) -> Option<String> {
    text.lines().filter_map(|line| line.trim().strip_prefix(key)?.trim_start().strip_prefix('=')).find_map(|value| {
        let value = value.trim().trim_matches(['"', '\'']).trim();
        (!value.is_empty()).then(|| value.to_owned())
    })
}
