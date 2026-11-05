//! Steam games the account owns but has not installed.
//!
//! The supported route is the Steam Web API with the user's OWN free key (`GetOwnedGames`), stored in the
//! Windows Credential Manager like the SteamGridDB key. Without a key the app only tries the anonymous
//! Steam Community pages, which Steam now often answers with its sign-in page. The SteamID64 comes from
//! the local `loginusers.vdf`; no Steam password is ever requested. Failures are statuses, never errors,
//! and nothing a failure says may contain the key (it travels in a query string).

use std::collections::HashSet;

use keyvalues_parser::Value;
use serde::Serialize;
use ts_rs::TS;

use crate::artwork::HttpGet;
use crate::artwork::http::HttpFailure;
use crate::artwork::secrets::{STEAM_WEB_API_KEY, SecretStore};
use crate::error::{CoreError, CoreResult};
use crate::platforms::steam::NON_GAME_APP_IDS;
use crate::redact::register_secret;

/// Most games read from one profile; a real library is a few thousand at most.
pub const MAX_OWNED_GAMES: usize = 20_000;

/// Longest title kept (a title is a short label; anything longer is malformed data).
const MAX_TITLE_CHARS: usize = 200;

/// What the last owned-library lookup found, for the UI hint under the setting.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export)]
pub enum OwnedLibraryStatus {
    /// The account's games were read.
    Listed,
    /// The profile (or its game details) is private, or lists no games.
    Private,
    /// Steam only shows the list to a signed-in client or a key holder, and no key is saved.
    NeedsKey,
    /// Steam refused the saved Web API key (HTTP 401/403).
    InvalidKey,
    /// Steam or its pages could not be reached or understood (offline, error).
    Unavailable,
    /// The setting is off: nothing was requested.
    #[default]
    Disabled,
}

/// One game from the owned list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OwnedGame {
    pub app_id: String,
    pub title: String,
    /// Playtime on record, when the account has played it.
    pub playtime_seconds: Option<i64>,
}

/// Why an XML answer produced no games.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OwnedParseError {
    /// Steam answered that the profile is private (or hides its game details).
    Private,
    /// Not the expected document.
    Unexpected,
}

/// Builds the XML profile URL for a SteamID64 (17 digits), or `None` for anything else.
pub fn profile_games_url(steam_id: &str) -> Option<String> {
    is_steam_id(steam_id).then(|| format!("https://steamcommunity.com/profiles/{steam_id}/games?tab=all&xml=1"))
}

/// The same list as a web page, which embeds it as JSON (`data-profile-gameslist`).
fn profile_games_page_url(steam_id: &str) -> Option<String> {
    is_steam_id(steam_id).then(|| format!("https://steamcommunity.com/profiles/{steam_id}/games/?tab=all"))
}

fn is_steam_id(value: &str) -> bool {
    value.len() == 17 && value.bytes().all(|byte| byte.is_ascii_digit())
}

/// What a failed request was, for logs: never its text (a URL in it could hold the key).
fn failure_kind(failure: &HttpFailure) -> &'static str {
    match failure {
        HttpFailure::Unauthorized => "unauthorized",
        HttpFailure::NotFound => "not found",
        HttpFailure::RateLimited => "rate limited",
        HttpFailure::Other(_) => "network or server error",
    }
}

/// A Steam Web API key is 32 hexadecimal characters.
pub fn is_valid_api_key(key: &str) -> bool {
    key.len() == 32 && key.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// `GetOwnedGames` URL for a valid key and SteamID64. The key is in the query string, so this URL must
/// never be logged, shown or put in an error.
fn owned_games_api_url(key: &str, steam_id: &str) -> Option<String> {
    (is_valid_api_key(key) && is_steam_id(steam_id)).then(|| {
        format!(
            "https://api.steampowered.com/IPlayerService/GetOwnedGames/v1/?key={key}&steamid={steam_id}\
             &include_appinfo=1&include_played_free_games=1&format=json"
        )
    })
}

/// Reads the owned games of a SteamID64 through `http`: the Web API with the user's key first (when one
/// is saved), then the anonymous Steam Community pages. Never returns an error: the status says why
/// there is nothing to list.
pub fn fetch_owned_games(
    http: &dyn HttpGet,
    steam_id: &str,
    api_key: Option<&str>,
) -> Result<Vec<OwnedGame>, OwnedLibraryStatus> {
    if !is_steam_id(steam_id) {
        return Err(OwnedLibraryStatus::Unavailable);
    }
    let api = api_key.map(|key| fetch_via_api(http, key, steam_id));
    match &api {
        Some(Ok(_)) => {
            if let Some(Ok(games)) = api {
                return Ok(games);
            }
        }
        // The API answered for this account: the game details are private, the anonymous pages cannot do better.
        Some(Err(OwnedLibraryStatus::Private)) => return Err(OwnedLibraryStatus::Private),
        _ => {}
    }
    match fetch_anonymous(http, steam_id) {
        Ok(games) => Ok(games),
        Err(anonymous) => Err(match (&api, anonymous) {
            (Some(Err(OwnedLibraryStatus::InvalidKey)), _) => OwnedLibraryStatus::InvalidKey,
            // A key is saved, so "needs a key" is wrong: the API simply could not be reached.
            (Some(_), OwnedLibraryStatus::NeedsKey) => OwnedLibraryStatus::Unavailable,
            (_, other) => other,
        }),
    }
}

/// The Steam Web API route. Errors are statuses only.
fn fetch_via_api(http: &dyn HttpGet, key: &str, steam_id: &str) -> Result<Vec<OwnedGame>, OwnedLibraryStatus> {
    let url = owned_games_api_url(key, steam_id).ok_or(OwnedLibraryStatus::InvalidKey)?;
    let body = http.get_text_same_host(&url).map_err(|failure| match failure {
        HttpFailure::Unauthorized => OwnedLibraryStatus::InvalidKey,
        other => {
            tracing::info!(kind = failure_kind(&other), "the Steam Web API request failed");
            OwnedLibraryStatus::Unavailable
        }
    })?;
    match parse_api_games(&body) {
        Ok(games) if games.is_empty() => Err(OwnedLibraryStatus::Private),
        Ok(games) => Ok(games),
        Err(OwnedParseError::Private) => Err(OwnedLibraryStatus::Private),
        Err(OwnedParseError::Unexpected) => Err(OwnedLibraryStatus::Unavailable),
    }
}

/// Parses a `GetOwnedGames` answer. An empty `response` object (no `games`) means the account's game
/// details are private.
pub fn parse_api_games(body: &str) -> Result<Vec<OwnedGame>, OwnedParseError> {
    let json: serde_json::Value = serde_json::from_str(body).map_err(|_| OwnedParseError::Unexpected)?;
    let response = json.get("response").and_then(serde_json::Value::as_object).ok_or(OwnedParseError::Unexpected)?;
    let Some(rows) = response.get("games").and_then(serde_json::Value::as_array) else {
        return Err(OwnedParseError::Private);
    };
    let mut games = Vec::new();
    let mut seen = HashSet::new();
    for row in rows.iter().take(MAX_OWNED_GAMES * 2) {
        let Some(app_id) = row.get("appid").and_then(serde_json::Value::as_u64).map(|id| id.to_string()) else {
            continue;
        };
        let title: String = row
            .get("name")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .trim()
            .chars()
            .take(MAX_TITLE_CHARS)
            .collect();
        if title.is_empty() || is_not_a_game(&app_id, &title) || !seen.insert(app_id.clone()) {
            continue;
        }
        let playtime_seconds = row
            .get("playtime_forever")
            .and_then(serde_json::Value::as_i64)
            .filter(|minutes| *minutes > 0 && *minutes < 53_000_000)
            .map(|minutes| minutes * 60);
        games.push(OwnedGame { app_id, title, playtime_seconds });
        if games.len() >= MAX_OWNED_GAMES {
            break;
        }
    }
    Ok(games)
}

/// The anonymous Steam Community route: the legacy XML list, then the games page's embedded JSON. A
/// sign-in page means Steam hides the list from anonymous clients (`NeedsKey`).
fn fetch_anonymous(http: &dyn HttpGet, steam_id: &str) -> Result<Vec<OwnedGame>, OwnedLibraryStatus> {
    let url = profile_games_url(steam_id).ok_or(OwnedLibraryStatus::Unavailable)?;
    let read = |url: &str| {
        http.get_text_same_host(url).map_err(|failure| {
            tracing::info!(kind = failure_kind(&failure), "the Steam profile could not be read");
            OwnedLibraryStatus::Unavailable
        })
    };
    let xml = read(&url)?;
    let outcome = match parse_owned_games(&xml) {
        Err(OwnedParseError::Unexpected) if !is_sign_in_page(&xml) => {
            // Not the XML list and not a login wall: try the web page's embedded JSON.
            let page = read(&profile_games_page_url(steam_id).ok_or(OwnedLibraryStatus::Unavailable)?)?;
            parse_embedded_games(&page)
        }
        other => other,
    };
    match outcome {
        Ok(games) if games.is_empty() => Err(OwnedLibraryStatus::Private),
        Ok(games) => Ok(games),
        Err(OwnedParseError::Private) => Err(OwnedLibraryStatus::Private),
        Err(OwnedParseError::Unexpected) => {
            Err(if is_sign_in_page(&xml) { OwnedLibraryStatus::NeedsKey } else { OwnedLibraryStatus::Unavailable })
        }
    }
}

/// Stores the user's Steam Web API key in the Credential Manager (only a valid-looking key). The value
/// is never returned, logged or echoed in an error.
pub fn save_steam_api_key(secrets: &dyn SecretStore, key: &str) -> CoreResult<()> {
    let key = key.trim();
    if !is_valid_api_key(key) {
        return Err(CoreError::Validation("a Steam Web API key is 32 characters: digits 0-9 and letters A-F".into()));
    }
    secrets.set(STEAM_WEB_API_KEY, key)?;
    register_secret(key);
    Ok(())
}

pub fn has_steam_api_key(secrets: &dyn SecretStore) -> CoreResult<bool> {
    Ok(secrets.get(STEAM_WEB_API_KEY)?.is_some())
}

pub fn clear_steam_api_key(secrets: &dyn SecretStore) -> CoreResult<()> {
    secrets.delete(STEAM_WEB_API_KEY)
}

/// The saved key, registered with the log redaction so it is masked wherever it could surface.
pub fn steam_api_key(secrets: &dyn SecretStore) -> CoreResult<Option<String>> {
    let key = secrets.get(STEAM_WEB_API_KEY)?.filter(|key| is_valid_api_key(key));
    if let Some(key) = &key {
        register_secret(key);
    }
    Ok(key)
}

/// Steam's login wall, shown to anonymous visitors of a profile whose game details are private.
fn is_sign_in_page(page: &str) -> bool {
    element_text(page, "title").is_some_and(|title| title.trim().eq_ignore_ascii_case("sign in"))
}

/// Reads the games list a Steam games page embeds as HTML-escaped JSON in `data-profile-gameslist`
/// (`rgGames`: `appid`, `name`, `hours_forever`).
pub fn parse_embedded_games(page: &str) -> Result<Vec<OwnedGame>, OwnedParseError> {
    if is_sign_in_page(page) || page.contains("profile_private_info") {
        return Err(OwnedParseError::Private);
    }
    let marker = "data-profile-gameslist=\"";
    let start = page.find(marker).ok_or(OwnedParseError::Unexpected)? + marker.len();
    let end = page[start..].find('"').ok_or(OwnedParseError::Unexpected)? + start;
    let json: serde_json::Value =
        serde_json::from_str(&decode_entities(&page[start..end])).map_err(|_| OwnedParseError::Unexpected)?;
    let rows = json.get("rgGames").and_then(serde_json::Value::as_array).ok_or(OwnedParseError::Unexpected)?;
    let mut games = Vec::new();
    let mut seen = HashSet::new();
    for row in rows.iter().take(MAX_OWNED_GAMES * 2) {
        let app_id = match row.get("appid") {
            Some(serde_json::Value::Number(number)) => number.to_string(),
            Some(serde_json::Value::String(text)) => text.trim().to_owned(),
            _ => continue,
        };
        if app_id.is_empty() || app_id.len() > 10 || !app_id.bytes().all(|byte| byte.is_ascii_digit()) {
            continue;
        }
        let title: String = row
            .get("name")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .trim()
            .chars()
            .take(MAX_TITLE_CHARS)
            .collect();
        if title.is_empty() || is_not_a_game(&app_id, &title) || !seen.insert(app_id.clone()) {
            continue;
        }
        let playtime_seconds = match row.get("hours_forever") {
            Some(serde_json::Value::String(hours)) => hours_to_seconds(hours),
            Some(serde_json::Value::Number(hours)) => {
                hours.as_f64().and_then(|value| hours_to_seconds(&value.to_string()))
            }
            _ => None,
        };
        games.push(OwnedGame { app_id, title, playtime_seconds });
        if games.len() >= MAX_OWNED_GAMES {
            break;
        }
    }
    Ok(games)
}

/// Parses the `gamesList` document defensively (no XML crate: the format is a flat list of
/// `<game>` elements). Tools, dedicated servers, soundtracks and demos are dropped.
pub fn parse_owned_games(xml: &str) -> Result<Vec<OwnedGame>, OwnedParseError> {
    if let Some(error) = element_text(xml, "error") {
        let private = error.to_ascii_lowercase().contains("private");
        return Err(if private { OwnedParseError::Private } else { OwnedParseError::Unexpected });
    }
    let list_start = xml.find("<gamesList").ok_or(OwnedParseError::Unexpected)?;
    let mut rest = &xml[list_start..];
    let mut games = Vec::new();
    let mut seen = HashSet::new();
    while let Some(start) = rest.find("<game>") {
        let after = &rest[start + "<game>".len()..];
        let Some(end) = after.find("</game>") else { break };
        let block = &after[..end];
        rest = &after[end + "</game>".len()..];

        let Some(app_id) = element_text(block, "appID").map(|id| id.trim().to_owned()) else { continue };
        if app_id.is_empty() || app_id.len() > 10 || !app_id.bytes().all(|byte| byte.is_ascii_digit()) {
            continue;
        }
        let title: String =
            element_text(block, "name").unwrap_or_default().trim().chars().take(MAX_TITLE_CHARS).collect();
        if title.is_empty() || is_not_a_game(&app_id, &title) || !seen.insert(app_id.clone()) {
            continue;
        }
        let playtime_seconds = element_text(block, "hoursOnRecord").and_then(|hours| hours_to_seconds(&hours));
        games.push(OwnedGame { app_id, title, playtime_seconds });
        if games.len() >= MAX_OWNED_GAMES {
            break;
        }
    }
    Ok(games)
}

/// `1,234.5` hours on record → seconds. Zero or unreadable means no playtime.
fn hours_to_seconds(text: &str) -> Option<i64> {
    let hours: f64 = text.trim().replace(',', "").parse().ok()?;
    let seconds = (hours * 3600.0).round();
    // Finite and within a sane range (a century of play) before the integer conversion.
    (seconds.is_finite() && seconds > 0.0 && seconds < 3.2e9).then_some(seconds as i64)
}

/// Tools, runtimes, servers, soundtracks, SDKs, playtests and demos are not games to list.
pub fn is_not_a_game(app_id: &str, title: &str) -> bool {
    if app_id.parse::<u32>().is_ok_and(|id| NON_GAME_APP_IDS.contains(&id)) {
        return true;
    }
    let lower = title.to_lowercase();
    const FRAGMENTS: [&str; 6] =
        ["dedicated server", "soundtrack", "redistributable", "playtest", "steam linux runtime", "proton "];
    if FRAGMENTS.iter().any(|fragment| lower.contains(fragment)) {
        return true;
    }
    let words: Vec<&str> = lower.split(|c: char| !c.is_alphanumeric()).filter(|word| !word.is_empty()).collect();
    words.contains(&"sdk") || words.last() == Some(&"demo")
}

/// Text of the first `<tag>…</tag>` in `block`, with CDATA unwrapped and basic entities decoded.
fn element_text(block: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let start = block.find(&open)? + open.len();
    let end = block[start..].find(&format!("</{tag}>"))? + start;
    let raw = block[start..end].trim();
    let text = raw.strip_prefix("<![CDATA[").and_then(|inner| inner.strip_suffix("]]>")).unwrap_or(raw);
    Some(decode_entities(text))
}

fn decode_entities(text: &str) -> String {
    text.replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

/// SteamID64 of the account Steam last signed in with (`MostRecent` = 1), else the first account.
pub fn most_recent_steam_id(loginusers_vdf: &str) -> Option<String> {
    let vdf = keyvalues_parser::parse(loginusers_vdf).ok()?;
    let users = vdf.value.get_obj()?;
    let mut first = None;
    for (id, values) in &users.0 {
        if !is_steam_id(id) {
            continue;
        }
        first.get_or_insert_with(|| id.to_string());
        let most_recent = values
            .first()
            .and_then(Value::get_obj)
            .and_then(|account| {
                account.0.iter().find(|(key, _)| key.eq_ignore_ascii_case("MostRecent")).and_then(|(_, v)| v.first())
            })
            .and_then(Value::get_str)
            == Some("1");
        if most_recent {
            return Some(id.to_string());
        }
    }
    first
}

/// SteamID64 of the Steam account on this PC, from `<Steam>\config\loginusers.vdf`.
pub fn locate_steam_id() -> Option<String> {
    let steam_dir = steamlocate::SteamDir::locate().ok()?;
    let text = std::fs::read_to_string(steam_dir.path().join("config").join("loginusers.vdf")).ok()?;
    most_recent_steam_id(&text)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artwork::http::HttpFailure;

    const PUBLIC: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<gamesList>
<steamID64>76561198000000000</steamID64>
<steamID><![CDATA[player]]></steamID>
<games>
<game>
<appID>814380</appID>
<name><![CDATA[Sekiro™: Shadows Die Twice]]></name>
<logo><![CDATA[https://cdn.example/logo.jpg]]></logo>
<storeLink><![CDATA[https://steamcommunity.com/app/814380]]></storeLink>
<hoursOnRecord>1,064.5</hoursOnRecord>
</game>
<game>
<appID>1245620</appID>
<name><![CDATA[ELDEN RING &amp; Co]]></name>
</game>
<game>
<appID>228980</appID>
<name><![CDATA[Steamworks Common Redistributables]]></name>
</game>
<game><appID>999</appID><name>Some Game Dedicated Server</name></game>
<game><appID>1000</appID><name>Great Game Soundtrack</name></game>
<game><appID>1001</appID><name>Source SDK Base 2013</name></game>
<game><appID>1002</appID><name>Cool Game Demo</name></game>
<game><appID>814380</appID><name>Duplicate</name></game>
<game><appID>12ab</appID><name>Bad id</name></game>
<game><appID>1003</appID><name></name></game>
<game>
<appID>1004</appID>
<name>Unfinished
</games>
</gamesList>"#;

    #[test]
    fn reads_titles_ids_and_hours() -> Result<(), OwnedParseError> {
        let games = parse_owned_games(PUBLIC)?;
        let ids: Vec<&str> = games.iter().map(|game| game.app_id.as_str()).collect();
        assert_eq!(
            ids,
            ["814380", "1245620"],
            "tools, servers, soundtracks, demos, duplicates and bad rows are dropped"
        );
        assert_eq!(games[0].title, "Sekiro™: Shadows Die Twice");
        assert_eq!(games[0].playtime_seconds, Some(3_832_200));
        assert_eq!(games[1].title, "ELDEN RING & Co");
        assert_eq!(games[1].playtime_seconds, None);
        Ok(())
    }

    #[test]
    fn a_private_profile_is_reported_as_private() {
        let private = "<gamesList><error><![CDATA[This profile is private.]]></error></gamesList>";
        assert_eq!(parse_owned_games(private), Err(OwnedParseError::Private));
        let missing = "<gamesList><error><![CDATA[The specified profile could not be found.]]></error></gamesList>";
        assert_eq!(parse_owned_games(missing), Err(OwnedParseError::Unexpected));
    }

    #[test]
    fn unexpected_documents_are_rejected_without_panicking() {
        for text in ["", "<html><body>Sign in</body></html>", "<gamesList", "\u{0}\u{1}garbage", "<game></game>"] {
            assert!(
                parse_owned_games(text).is_err() || parse_owned_games(text).is_ok_and(|g| g.is_empty()),
                "{text:?}"
            );
        }
        assert_eq!(parse_owned_games("<gamesList><games></games></gamesList>"), Ok(Vec::new()));
    }

    #[test]
    fn a_huge_list_is_capped() -> Result<(), OwnedParseError> {
        let mut xml = String::from("<gamesList><games>");
        for id in 0..(MAX_OWNED_GAMES + 500) {
            xml.push_str(&format!("<game><appID>{}</appID><name>Game {id}</name></game>", id + 10));
        }
        xml.push_str("</games></gamesList>");
        assert_eq!(parse_owned_games(&xml)?.len(), MAX_OWNED_GAMES);
        Ok(())
    }

    #[test]
    fn hours_are_parsed_with_thousand_separators() {
        assert_eq!(hours_to_seconds("1,234.5"), Some(4_444_200));
        assert_eq!(hours_to_seconds("0.0"), None);
        assert_eq!(hours_to_seconds("-3"), None);
        assert_eq!(hours_to_seconds("lots"), None);
        assert_eq!(hours_to_seconds("1e300"), None);
    }

    #[test]
    fn junk_titles_are_recognised() {
        assert!(is_not_a_game("1", "Team Fortress 2 Dedicated Server"));
        assert!(is_not_a_game("1", "Hades Soundtrack"));
        assert!(is_not_a_game("1", "Source SDK Base 2013 Singleplayer"));
        assert!(is_not_a_game("1", "Proton Experimental"));
        assert!(is_not_a_game("1", "Portal Demo"));
        assert!(is_not_a_game("250820", "SteamVR"));
        assert!(!is_not_a_game("1", "Demon's Souls"));
        assert!(!is_not_a_game("1", "Sdkfz Simulator"));
        assert!(!is_not_a_game("1", "Hades II"));
    }

    #[test]
    fn the_most_recent_account_wins() {
        let vdf = r#""users"
{
    "76561198000000001" { "AccountName" "a" "MostRecent" "0" }
    "76561198000000002" { "AccountName" "b" "MostRecent" "1" }
    "notanid" { "MostRecent" "1" }
}"#;
        assert_eq!(most_recent_steam_id(vdf).as_deref(), Some("76561198000000002"));
        let none_recent = r#""users" { "76561198000000009" { "AccountName" "c" "MostRecent" "0" } }"#;
        assert_eq!(most_recent_steam_id(none_recent).as_deref(), Some("76561198000000009"));
        assert_eq!(most_recent_steam_id("\"users\" { }"), None);
        assert_eq!(most_recent_steam_id("not vdf {{"), None);
    }

    #[test]
    fn only_a_real_steam_id_builds_a_url() {
        assert_eq!(
            profile_games_url("76561198000000000").as_deref(),
            Some("https://steamcommunity.com/profiles/76561198000000000/games?tab=all&xml=1")
        );
        for bad in ["", "123", "7656119800000000x", "../../etc", "76561198000000000/../x"] {
            assert!(profile_games_url(bad).is_none(), "{bad}");
        }
    }

    const SIGN_IN: &str = "<html><head><title>Sign In</title></head><body>log in</body></html>";

    fn games_page(rows: &str) -> String {
        let json = format!("{{&quot;strProfileURL&quot;:&quot;x&quot;,&quot;rgGames&quot;:[{rows}]}}");
        format!(
            "<html><head><title>Steam Community :: Games</title></head><body><div id=\"gameslist_config\" data-profile-gameslist=\"{json}\"></div></body></html>"
        )
    }

    #[test]
    fn the_games_page_embeds_the_list_as_json() -> Result<(), OwnedParseError> {
        let page = games_page(
            "{&quot;appid&quot;:367520,&quot;name&quot;:&quot;Hollow Knight&quot;,&quot;hours_forever&quot;:&quot;1,020.5&quot;},\
             {&quot;appid&quot;:1245620,&quot;name&quot;:&quot;ELDEN RING&quot;},\
             {&quot;appid&quot;:228980,&quot;name&quot;:&quot;Steamworks Common Redistributables&quot;},\
             {&quot;appid&quot;:367520,&quot;name&quot;:&quot;Duplicate&quot;},\
             {&quot;name&quot;:&quot;No id&quot;}",
        );
        let games = parse_embedded_games(&page)?;
        let ids: Vec<&str> = games.iter().map(|game| game.app_id.as_str()).collect();
        assert_eq!(ids, ["367520", "1245620"]);
        assert_eq!(games[0].playtime_seconds, Some(3_673_800));
        assert_eq!(parse_embedded_games(SIGN_IN), Err(OwnedParseError::Private));
        assert_eq!(parse_embedded_games("<html></html>"), Err(OwnedParseError::Unexpected));
        assert_eq!(parse_embedded_games("<div data-profile-gameslist=\"{broken\">"), Err(OwnedParseError::Unexpected));
        Ok(())
    }

    #[test]
    fn a_login_wall_without_a_key_asks_for_one() {
        let id = "76561198000000000";
        assert_eq!(fetch_owned_games(&FakeHttp(Ok(SIGN_IN.into())), id, None), Err(OwnedLibraryStatus::NeedsKey));
    }

    /// Answers each URL with its own body, to exercise the XML-then-page fallback.
    struct RoutedHttp;

    impl HttpGet for RoutedHttp {
        fn get_text(&self, url: &str, _bearer: Option<&str>) -> Result<String, HttpFailure> {
            if url.ends_with("xml=1") {
                Ok("<html><body>something else</body></html>".into())
            } else {
                Ok(games_page("{&quot;appid&quot;:367520,&quot;name&quot;:&quot;Hollow Knight&quot;}"))
            }
        }
        fn get_bytes(&self, _url: &str) -> Result<Vec<u8>, HttpFailure> {
            Err(HttpFailure::NotFound)
        }
    }

    #[test]
    fn the_web_page_is_the_fallback_when_the_xml_is_not_served() {
        let games = fetch_owned_games(&RoutedHttp, "76561198000000000", None);
        assert_eq!(games.map(|list| list.len()), Ok(1));
    }

    const KEY: &str = "0123456789ABCDEF0123456789ABCDEF";
    const ID: &str = "76561198000000000";

    const API_BODY: &str = r#"{"response":{"game_count":4,"games":[
        {"appid":367520,"name":"Hollow Knight","playtime_forever":210},
        {"appid":1245620,"name":"ELDEN RING","playtime_forever":0},
        {"appid":228980,"name":"Steamworks Common Redistributables"},
        {"appid":1,"name":"Some Server Dedicated Server"},
        {"appid":2}]}}"#;

    /// Answers the Web API and the anonymous pages independently, and remembers every URL asked.
    struct SplitHttp {
        api: Result<String, HttpFailure>,
        anonymous: Result<String, HttpFailure>,
        asked: std::sync::Mutex<Vec<String>>,
    }

    impl SplitHttp {
        fn new(api: Result<String, HttpFailure>, anonymous: Result<String, HttpFailure>) -> Self {
            Self { api, anonymous, asked: std::sync::Mutex::new(Vec::new()) }
        }
    }

    impl HttpGet for SplitHttp {
        fn get_text(&self, url: &str, _bearer: Option<&str>) -> Result<String, HttpFailure> {
            if let Ok(mut asked) = self.asked.lock() {
                asked.push(url.to_owned());
            }
            if url.starts_with("https://api.steampowered.com/") { self.api.clone() } else { self.anonymous.clone() }
        }
        fn get_bytes(&self, _url: &str) -> Result<Vec<u8>, HttpFailure> {
            Err(HttpFailure::NotFound)
        }
    }

    #[test]
    fn the_web_api_lists_owned_games_with_the_users_key() -> Result<(), OwnedParseError> {
        let games = parse_api_games(API_BODY)?;
        let ids: Vec<&str> = games.iter().map(|game| game.app_id.as_str()).collect();
        assert_eq!(ids, ["367520", "1245620"]);
        assert_eq!((games[0].playtime_seconds, games[1].playtime_seconds), (Some(12_600), None));
        assert_eq!(parse_api_games(r#"{"response":{}}"#), Err(OwnedParseError::Private));
        assert_eq!(parse_api_games("<html>Forbidden</html>"), Err(OwnedParseError::Unexpected));
        assert_eq!(parse_api_games(r#"{"other":1}"#), Err(OwnedParseError::Unexpected));

        let http = SplitHttp::new(Ok(API_BODY.into()), Ok(SIGN_IN.into()));
        let listed = fetch_owned_games(&http, ID, Some(KEY));
        assert_eq!(listed.map(|games| games.len()), Ok(2));
        let asked = http.asked.lock().map(|asked| asked.clone()).unwrap_or_default();
        assert_eq!(asked.len(), 1, "a working key never touches the anonymous pages");
        assert!(asked[0].contains(&format!("key={KEY}")) && asked[0].contains(&format!("steamid={ID}")));
        assert!(asked[0].contains("include_appinfo=1") && asked[0].contains("include_played_free_games=1"));
        Ok(())
    }

    #[test]
    fn every_key_outcome_maps_to_a_status() {
        let private = || Ok(r#"{"response":{}}"#.to_owned());
        let wall = || Ok(SIGN_IN.to_owned());
        let down = || Err(HttpFailure::Other("offline".into()));
        // Private details: authoritative, no second request.
        let http = SplitHttp::new(private(), wall());
        assert_eq!(fetch_owned_games(&http, ID, Some(KEY)), Err(OwnedLibraryStatus::Private));
        assert_eq!(http.asked.lock().map(|asked| asked.len()).unwrap_or_default(), 1);
        // A refused key.
        let refused = SplitHttp::new(Err(HttpFailure::Unauthorized), wall());
        assert_eq!(fetch_owned_games(&refused, ID, Some(KEY)), Err(OwnedLibraryStatus::InvalidKey));
        // A refused key, but the anonymous pages work: the games are listed anyway.
        let anon = SplitHttp::new(Err(HttpFailure::Unauthorized), Ok(PUBLIC.into()));
        assert_eq!(fetch_owned_games(&anon, ID, Some(KEY)).map(|games| games.len()), Ok(2));
        // The API is down and Steam shows its login wall: not "needs a key", since one is saved.
        assert_eq!(
            fetch_owned_games(&SplitHttp::new(down(), wall()), ID, Some(KEY)),
            Err(OwnedLibraryStatus::Unavailable)
        );
        // No key at all and a login wall.
        assert_eq!(fetch_owned_games(&SplitHttp::new(down(), wall()), ID, None), Err(OwnedLibraryStatus::NeedsKey));
        // API down but the anonymous page lists games.
        assert_eq!(
            fetch_owned_games(&SplitHttp::new(down(), Ok(PUBLIC.into())), ID, Some(KEY)).map(|g| g.len()),
            Ok(2)
        );
        // A malformed saved key never reaches the network.
        let never = SplitHttp::new(Ok(API_BODY.into()), wall());
        assert_eq!(fetch_owned_games(&never, ID, Some("not-a-key")), Err(OwnedLibraryStatus::InvalidKey));
        assert!(never.asked.lock().map(|asked| asked.iter().all(|url| !url.contains("not-a-key"))).unwrap_or(false));
    }

    #[test]
    fn the_key_is_validated_stored_and_never_echoed() -> CoreResult<()> {
        use crate::artwork::secrets::MemorySecretStore;
        let store = MemorySecretStore::default();
        assert!(!has_steam_api_key(&store)?);
        for bad in [
            "",
            "short",
            &KEY[..31],
            &format!("{KEY}0"),
            "ZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZZ",
            "0123456789ABCDEF 123456789ABCDEF",
        ] {
            let error = save_steam_api_key(&store, bad).err().map(|error| error.to_string()).unwrap_or_default();
            assert!(!error.is_empty(), "{bad:?} must be refused");
            assert!(bad.is_empty() || !error.contains(bad), "the error must not echo the value");
        }
        assert!(!has_steam_api_key(&store)?);
        save_steam_api_key(&store, &format!("  {KEY}\n"))?;
        assert!(has_steam_api_key(&store)?);
        assert_eq!(steam_api_key(&store)?.as_deref(), Some(KEY));
        clear_steam_api_key(&store)?;
        assert_eq!(steam_api_key(&store)?, None);
        clear_steam_api_key(&store)?;
        Ok(())
    }

    /// A tracing writer that keeps everything logged.
    #[derive(Clone, Default)]
    struct LogCapture(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

    impl std::io::Write for LogCapture {
        fn write(&mut self, buffer: &[u8]) -> std::io::Result<usize> {
            if let Ok(mut inner) = self.0.lock() {
                inner.extend_from_slice(buffer);
            }
            Ok(buffer.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    impl<'a> tracing_subscriber::fmt::MakeWriter<'a> for LogCapture {
        type Writer = LogCapture;
        fn make_writer(&'a self) -> LogCapture {
            self.clone()
        }
    }

    #[test]
    fn a_failing_request_never_puts_the_key_in_a_log_or_an_error() {
        // The failure text a transport could produce, URL and key included.
        let leaky = format!("error sending request for url (https://api.steampowered.com/x/?key={KEY}&steamid={ID})");
        let capture = LogCapture::default();
        let subscriber =
            tracing_subscriber::fmt().with_writer(capture.clone()).with_max_level(tracing::Level::TRACE).finish();
        let results = tracing::subscriber::with_default(subscriber, || {
            [
                fetch_owned_games(
                    &SplitHttp::new(Err(HttpFailure::Other(leaky.clone())), Err(HttpFailure::Other(leaky.clone()))),
                    ID,
                    Some(KEY),
                ),
                fetch_owned_games(
                    &SplitHttp::new(Err(HttpFailure::Unauthorized), Err(HttpFailure::Other(leaky.clone()))),
                    ID,
                    Some(KEY),
                ),
                fetch_owned_games(
                    &SplitHttp::new(Err(HttpFailure::RateLimited), Err(HttpFailure::NotFound)),
                    ID,
                    Some(KEY),
                ),
            ]
        });
        let logged =
            String::from_utf8_lossy(&capture.0.lock().map(|inner| inner.clone()).unwrap_or_default()).into_owned();
        assert!(!logged.contains(KEY), "log: {logged}");
        assert!(!logged.contains("api.steampowered.com"), "log: {logged}");
        assert!(!logged.is_empty(), "the failures are still logged, by kind");
        for result in results {
            assert!(!format!("{result:?}").contains(KEY));
        }
    }

    struct FakeHttp(Result<String, HttpFailure>);

    impl HttpGet for FakeHttp {
        fn get_text(&self, _url: &str, _bearer: Option<&str>) -> Result<String, HttpFailure> {
            self.0.clone()
        }
        fn get_bytes(&self, _url: &str) -> Result<Vec<u8>, HttpFailure> {
            Err(HttpFailure::NotFound)
        }
    }

    #[test]
    fn fetching_maps_every_outcome_to_a_status() {
        let id = "76561198000000000";
        assert_eq!(fetch_owned_games(&FakeHttp(Ok(PUBLIC.into())), id, None).map(|games| games.len()), Ok(2));
        let private = "<gamesList><error>This profile is private.</error></gamesList>";
        assert_eq!(fetch_owned_games(&FakeHttp(Ok(private.into())), id, None), Err(OwnedLibraryStatus::Private));
        assert_eq!(
            fetch_owned_games(&FakeHttp(Ok("<gamesList><games></games></gamesList>".into())), id, None),
            Err(OwnedLibraryStatus::Private)
        );
        assert_eq!(fetch_owned_games(&FakeHttp(Ok("<html/>".into())), id, None), Err(OwnedLibraryStatus::Unavailable));
        assert_eq!(
            fetch_owned_games(&FakeHttp(Err(HttpFailure::Other("offline".into()))), id, None),
            Err(OwnedLibraryStatus::Unavailable)
        );
        assert_eq!(fetch_owned_games(&FakeHttp(Ok(PUBLIC.into())), "bad", None), Err(OwnedLibraryStatus::Unavailable));
    }

    /// Reads the real `loginusers.vdf` and the real public profile of this PC's Steam account.
    /// Prints counts and a status only. Run with `cargo test -p l8-core live_owned -- --ignored --nocapture`.
    #[test]
    #[ignore = "network and a local Steam install"]
    fn live_owned_library_of_this_pc() {
        let Some(id) = locate_steam_id() else {
            println!("owned library: no Steam account found on this PC");
            return;
        };
        let http = crate::artwork::http::UreqHttp::new();
        match fetch_owned_games(&http, &id, None) {
            Ok(games) => {
                let played = games.iter().filter(|game| game.playtime_seconds.is_some()).count();
                println!("owned library: {} games listed, {played} with playtime", games.len());
            }
            Err(status) => println!("owned library: status {status:?}"),
        }
    }
}
