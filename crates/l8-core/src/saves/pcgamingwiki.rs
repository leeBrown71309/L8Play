//! Layer 2 of save detection: PCGamingWiki (MediaWiki + Cargo API, CC BY-NC-SA 3.0).
//!
//! 1. `cargoquery` finds the wiki page of a game by Steam app id or exact title.
//! 2. `parse` returns the page wikitext, whose `{{Game data/saves|Windows|path…}}` templates list
//!    where the game keeps its saves, using `{{p|appdata}}`-style placeholders.
//!
//! Paths are expanded against this PC and only existing folders are offered; nothing is used until
//! the user confirms it.

use std::path::{Path, PathBuf};

use serde::Deserialize;

#[cfg(test)]
use crate::artwork::http::HttpFailure;
use crate::artwork::http::{HttpGet, percent_decode, percent_encode};
use crate::artwork::steamgriddb::normalize_title;
use crate::error::{CoreError, CoreResult};

const API: &str = "https://www.pcgamingwiki.com/w/api.php";
const PAGE_URL: &str = "https://www.pcgamingwiki.com/wiki/";
const APP_ID_SERVICE: &str = "https://www.pcgamingwiki.com/api/appid.php?appid=";
const MAX_TEMPLATES: usize = 40;

/// Folders a `{{p|…}}` placeholder expands to on this PC.
#[derive(Debug, Clone)]
pub struct PathEnvironment {
    pub appdata: PathBuf,
    pub localappdata: PathBuf,
    pub userprofile: PathBuf,
    pub programdata: PathBuf,
    pub username: String,
    pub install_dir: Option<PathBuf>,
}

impl PathEnvironment {
    pub fn from_process(install_dir: Option<PathBuf>) -> Self {
        let var = |name: &str| std::env::var_os(name).map(PathBuf::from).unwrap_or_default();
        Self {
            appdata: var("APPDATA"),
            localappdata: var("LOCALAPPDATA"),
            userprofile: var("USERPROFILE"),
            programdata: var("PROGRAMDATA"),
            username: std::env::var("USERNAME").unwrap_or_default(),
            install_dir,
        }
    }
}

/// A path written on the wiki, before expansion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiPath {
    pub platform: String,
    pub raw: String,
}

/// A folder the wiki says holds saves, resolved on this PC.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WikiCandidate {
    pub path: PathBuf,
    pub template: String,
}

/// "https://www.pcgamingwiki.com/wiki/Sekiro:_Shadows_Die_Twice" → "Sekiro: Shadows Die Twice".
fn page_from_url(url: &str) -> Option<String> {
    let name = url.strip_prefix(PAGE_URL)?.split(['#', '?']).next()?;
    let page = percent_decode(name).replace('_', " ");
    (!page.trim().is_empty()).then_some(page)
}

pub fn page_url(page: &str) -> String {
    format!("{PAGE_URL}{}", percent_encode(&page.replace(' ', "_")).replace("%2F", "/"))
}

#[derive(Deserialize)]
struct QueryResponse {
    query: Option<QueryBody>,
}

#[derive(Deserialize)]
struct QueryBody {
    #[serde(default)]
    pages: Vec<QueryPage>,
}

#[derive(Deserialize)]
struct QueryPage {
    title: String,
    #[serde(default)]
    missing: bool,
}

#[derive(Deserialize)]
struct ParseResponse {
    parse: Option<ParseBody>,
}

#[derive(Deserialize)]
struct ParseBody {
    wikitext: String,
}

pub struct PcgwClient<'a> {
    http: &'a dyn HttpGet,
}

impl<'a> PcgwClient<'a> {
    pub fn new(http: &'a dyn HttpGet) -> Self {
        Self { http }
    }

    fn get(&self, query: &str) -> CoreResult<String> {
        self.http.get_text(&format!("{API}?{query}&format=json&maxlag=5"), None).map_err(CoreError::from)
    }

    /// The wiki page of a game: by Steam app id (PCGamingWiki's own redirect service), then by
    /// exact title, then by a search whose result must match the title exactly (never a guess).
    /// Anonymous clients cannot run Cargo queries, so those are not used.
    pub fn find_page(&self, steam_app_id: Option<&str>, title: &str) -> CoreResult<Option<String>> {
        if let Some(id) = steam_app_id.filter(|id| !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()))
            && let Some(target) =
                self.http.redirect_target(&format!("{APP_ID_SERVICE}{id}")).map_err(CoreError::from)?
            && let Some(page) = page_from_url(&target)
        {
            return Ok(Some(page));
        }
        let title = title.trim();
        if title.is_empty() {
            return Ok(None);
        }
        if let Some(page) = self.existing_page(title)? {
            return Ok(Some(page));
        }
        self.exact_search_hit(title)
    }

    /// The canonical page name when a page with that title exists.
    fn existing_page(&self, title: &str) -> CoreResult<Option<String>> {
        let body = match self.get(&format!("action=query&titles={}&redirects=1&formatversion=2", percent_encode(title)))
        {
            Ok(body) => body,
            Err(CoreError::NotFound(_)) => return Ok(None),
            Err(other) => return Err(other),
        };
        let parsed: QueryResponse = serde_json::from_str(&body)
            .map_err(|error| CoreError::External(format!("unexpected answer from PCGamingWiki: {error}")))?;
        Ok(parsed.query.and_then(|query| query.pages.into_iter().find(|page| !page.missing)).map(|page| page.title))
    }

    /// A search result whose normalized name equals the title's.
    fn exact_search_hit(&self, title: &str) -> CoreResult<Option<String>> {
        let body = match self.get(&format!("action=opensearch&search={}&limit=5", percent_encode(title))) {
            Ok(body) => body,
            Err(CoreError::NotFound(_)) => return Ok(None),
            Err(other) => return Err(other),
        };
        let parsed: serde_json::Value = serde_json::from_str(&body)
            .map_err(|error| CoreError::External(format!("unexpected answer from PCGamingWiki: {error}")))?;
        let wanted = normalize_title(title);
        Ok(parsed
            .get(1)
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_str)
            .find(|candidate| normalize_title(candidate) == wanted)
            .map(str::to_owned))
    }

    pub fn wikitext(&self, page: &str) -> CoreResult<Option<String>> {
        let query = format!(
            "action=parse&page={}&prop=wikitext&redirects=1&formatversion=2",
            percent_encode(&page.replace(' ', "_"))
        );
        let body = match self.get(&query) {
            Ok(body) => body,
            Err(CoreError::NotFound(_)) => return Ok(None),
            Err(other) => return Err(other),
        };
        let parsed: ParseResponse = serde_json::from_str(&body)
            .map_err(|error| CoreError::External(format!("unexpected answer from PCGamingWiki: {error}")))?;
        Ok(parsed.parse.map(|body| body.wikitext))
    }
}

/// Windows paths from every `{{Game data/saves|Windows|…}}` template of the page, in order.
pub fn extract_windows_paths(wikitext: &str) -> Vec<WikiPath> {
    let mut found = Vec::new();
    let lower = wikitext.to_ascii_lowercase();
    let mut cursor = 0;
    while let Some(offset) = lower[cursor..].find("{{game data/saves") {
        let start = cursor + offset;
        let Some(end) = matching_close(wikitext, start) else { break };
        let inner = &wikitext[start + 2..end];
        cursor = end + 2;
        let parts = split_top_level(inner);
        let (Some(platform), rest) = (parts.get(1), parts.iter().skip(2)) else { continue };
        if !platform.trim().eq_ignore_ascii_case("windows") {
            continue;
        }
        for raw in rest {
            let cleaned = strip_markup(raw);
            if !cleaned.is_empty() {
                found.push(WikiPath { platform: "Windows".into(), raw: cleaned });
            }
        }
        if found.len() >= MAX_TEMPLATES {
            break;
        }
    }
    found
}

/// Index of the `}}` closing the template that opens at `start`.
fn matching_close(text: &str, start: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut depth = 0usize;
    let mut index = start;
    while index + 1 < bytes.len() {
        match (bytes[index], bytes[index + 1]) {
            (b'{', b'{') => {
                depth += 1;
                index += 2;
            }
            (b'}', b'}') => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(index);
                }
                index += 2;
            }
            _ => index += 1,
        }
    }
    None
}

/// Splits on `|` that are not inside nested `{{…}}` or `[[…]]`.
fn split_top_level(inner: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut depth = 0i32;
    let chars: Vec<char> = inner.chars().collect();
    let mut index = 0;
    while index < chars.len() {
        let pair = (chars[index], chars.get(index + 1).copied());
        match pair {
            ('{', Some('{')) | ('[', Some('[')) => {
                depth += 1;
                current.push(chars[index]);
                current.push(chars[index + 1]);
                index += 2;
            }
            ('}', Some('}')) | (']', Some(']')) => {
                depth -= 1;
                current.push(chars[index]);
                current.push(chars[index + 1]);
                index += 2;
            }
            ('|', _) if depth == 0 => {
                parts.push(std::mem::take(&mut current));
                index += 1;
            }
            _ => {
                current.push(chars[index]);
                index += 1;
            }
        }
    }
    parts.push(current);
    parts
}

/// Drops `<ref>…</ref>`, `<!-- -->`, other tags and surrounding whitespace.
fn strip_markup(raw: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for ch in remove_between(raw, "<ref", "</ref>").chars() {
        match ch {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out.trim().to_owned()
}

fn remove_between(text: &str, open: &str, close: &str) -> String {
    let mut result = String::new();
    let mut rest = text;
    while let Some(start) = rest.find(open) {
        result.push_str(&rest[..start]);
        match rest[start..].find(close) {
            Some(end) => rest = &rest[start + end + close.len()..],
            None => return result,
        }
    }
    result.push_str(rest);
    result
}

/// Replaces `{{p|…}}` placeholders. `None` when one cannot be resolved on this PC (registry
/// keys, other launchers' folders…), so such paths are skipped rather than guessed.
pub fn expand_placeholders(raw: &str, env: &PathEnvironment) -> Option<String> {
    let mut out = String::new();
    let mut rest = raw;
    while let Some(start) = rest.find("{{") {
        out.push_str(&rest[..start]);
        let end = rest[start..].find("}}")? + start;
        let token = &rest[start + 2..end];
        let mut parts = token.splitn(2, '|');
        let template = parts.next()?.trim().to_ascii_lowercase();
        let argument = parts.next()?.trim();
        if template != "p" {
            return None;
        }
        out.push_str(&expand_one(argument, env)?);
        rest = &rest[end + 2..];
    }
    out.push_str(rest);
    if out.contains("{{") || out.contains("}}") || out.starts_with("HKEY") {
        return None;
    }
    Some(out.replace('/', "\\"))
}

fn expand_one(argument: &str, env: &PathEnvironment) -> Option<String> {
    let normalized = argument.replace('/', "\\");
    let (name, tail) = normalized.split_once('\\').map_or((normalized.as_str(), ""), |(n, t)| (n, t));
    let base: String = match name.to_ascii_lowercase().as_str() {
        "appdata" => env.appdata.to_string_lossy().into_owned(),
        "localappdata" => env.localappdata.to_string_lossy().into_owned(),
        "locallow" | "localappdatalow" => {
            env.userprofile.join("AppData").join("LocalLow").to_string_lossy().into_owned()
        }
        "userprofile" | "profile" => env.userprofile.to_string_lossy().into_owned(),
        "programdata" => env.programdata.to_string_lossy().into_owned(),
        "public" => r"C:\Users\Public".to_owned(),
        "game" => env.install_dir.as_ref()?.to_string_lossy().into_owned(),
        "username" => env.username.clone(),
        "uid" | "steamid3" | "steamid64" => "*".to_owned(),
        _ => return None,
    };
    if base.is_empty() {
        return None;
    }
    Some(if tail.is_empty() { base } else { format!("{base}\\{tail}") })
}

/// Turns an expanded path into an existing folder: a path with wildcards resolves to the folder
/// that holds the wildcard component; a file resolves to its parent folder.
pub fn resolve_existing_folder(expanded: &str) -> Option<PathBuf> {
    let components: Vec<&str> = expanded.split('\\').filter(|part| !part.is_empty()).collect();
    let wildcard_at = components.iter().position(|part| part.contains('*') || part.contains('?'));
    let fixed = &components[..wildcard_at.unwrap_or(components.len())];
    if fixed.is_empty() {
        return None;
    }
    // Only absolute drive paths: a bare drive letter or a relative path is never a save folder.
    if fixed.len() < 2 || !fixed[0].ends_with(':') {
        return None;
    }
    let mut path = PathBuf::from(fixed.join("\\"));
    if path.is_file() {
        path.pop();
    }
    path.is_dir().then_some(path)
}

/// Existing folders named by the page, deduplicated, with the wiki template that named them.
pub fn candidates_from_wikitext(wikitext: &str, env: &PathEnvironment) -> Vec<WikiCandidate> {
    let mut found: Vec<WikiCandidate> = Vec::new();
    for wiki_path in extract_windows_paths(wikitext) {
        let Some(expanded) = expand_placeholders(&wiki_path.raw, env) else { continue };
        let Some(path) = resolve_existing_folder(&expanded) else { continue };
        if !found.iter().any(|candidate| same_folder(&candidate.path, &path)) {
            found.push(WikiCandidate { path, template: wiki_path.raw });
        }
    }
    found
}

fn same_folder(a: &Path, b: &Path) -> bool {
    a.to_string_lossy().eq_ignore_ascii_case(&b.to_string_lossy())
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use super::*;

    fn env(root: &Path) -> PathEnvironment {
        PathEnvironment {
            appdata: root.join("AppData").join("Roaming"),
            localappdata: root.join("AppData").join("Local"),
            userprofile: root.to_path_buf(),
            programdata: root.join("ProgramData"),
            username: "ryu".into(),
            install_dir: Some(root.join("Games").join("Foo")),
        }
    }

    const WIKITEXT: &str = r"
==Save game data location==
{{Game data|
{{Game data/saves|Windows|{{p|appdata}}\Foo\saves|{{p|userprofile}}\Documents\My Games\Foo\{{p|uid}}\}}
{{Game data/saves|Steam|{{p|game}}\steam}}
{{Game data/saves|Windows|{{p|hkcu}}\Software\Foo<ref>registry</ref>}}
{{Game data/saves|Linux|~/.config/foo}}
{{Game data/saves|Windows|{{p|game}}\save<!-- comment -->}}
}}";

    #[test]
    fn extracts_only_windows_templates_and_handles_nesting() {
        let paths = extract_windows_paths(WIKITEXT);
        let raws: Vec<&str> = paths.iter().map(|p| p.raw.as_str()).collect();
        assert_eq!(
            raws,
            vec![
                r"{{p|appdata}}\Foo\saves",
                r"{{p|userprofile}}\Documents\My Games\Foo\{{p|uid}}\",
                r"{{p|hkcu}}\Software\Foo",
                r"{{p|game}}\save",
            ]
        );
    }

    #[test]
    fn expands_known_placeholders_and_refuses_unknown_ones() {
        let root = Path::new(r"C:\Users\ryu");
        let env = env(root);
        assert_eq!(
            expand_placeholders(r"{{p|appdata}}\Foo", &env).as_deref(),
            Some(r"C:\Users\ryu\AppData\Roaming\Foo")
        );
        assert_eq!(
            expand_placeholders(r"{{p|userprofile\Documents}}\Foo\{{p|uid}}", &env).as_deref(),
            Some(r"C:\Users\ryu\Documents\Foo\*")
        );
        assert_eq!(expand_placeholders(r"{{p|hkcu}}\Software", &env), None);
        assert_eq!(expand_placeholders(r"{{p|steam}}\userdata", &env), None);
        let no_install = PathEnvironment { install_dir: None, ..env };
        assert_eq!(expand_placeholders(r"{{p|game}}\save", &no_install), None);
    }

    #[test]
    fn resolves_only_folders_that_exist_and_collapses_wildcards() -> CoreResult<()> {
        let dir = tempfile::tempdir()?;
        let env = env(dir.path());
        std::fs::create_dir_all(env.appdata.join("Foo").join("saves"))?;
        std::fs::create_dir_all(dir.path().join("Documents").join("My Games").join("Foo").join("12345"))?;
        std::fs::create_dir_all(dir.path().join("Games").join("Foo"))?;
        std::fs::write(dir.path().join("Games").join("Foo").join("save"), b"x")?;

        let found = candidates_from_wikitext(WIKITEXT, &env);
        let paths: Vec<PathBuf> = found.into_iter().map(|c| c.path).collect();
        assert_eq!(
            paths,
            vec![
                env.appdata.join("Foo").join("saves"),
                dir.path().join("Documents").join("My Games").join("Foo"),
                dir.path().join("Games").join("Foo"),
            ],
            "a file path resolves to its folder; the registry path and missing folders are skipped"
        );
        Ok(())
    }

    struct FakeHttp {
        answers: RefCell<Vec<Result<String, HttpFailure>>>,
        urls: RefCell<Vec<String>>,
        redirect: Option<String>,
    }

    impl HttpGet for FakeHttp {
        fn get_text(&self, url: &str, _bearer: Option<&str>) -> Result<String, HttpFailure> {
            self.urls.borrow_mut().push(url.to_owned());
            self.answers.borrow_mut().remove(0)
        }

        fn get_bytes(&self, _url: &str) -> Result<Vec<u8>, HttpFailure> {
            Err(HttpFailure::NotFound)
        }

        fn redirect_target(&self, _url: &str) -> Result<Option<String>, HttpFailure> {
            Ok(self.redirect.clone())
        }
    }

    #[test]
    fn finds_a_page_by_steam_id_through_the_redirect_service() -> CoreResult<()> {
        let http = FakeHttp {
            answers: RefCell::new(Vec::new()),
            urls: RefCell::new(Vec::new()),
            redirect: Some("https://www.pcgamingwiki.com/wiki/Sekiro:_Shadows_Die_Twice".into()),
        };
        let page = PcgwClient::new(&http).find_page(Some("814380"), "Whatever")?;
        assert_eq!(page.as_deref(), Some("Sekiro: Shadows Die Twice"));
        assert!(http.urls.borrow().is_empty(), "no other request was needed");
        Ok(())
    }

    #[test]
    fn falls_back_to_the_exact_title_then_to_an_exact_search_hit() -> CoreResult<()> {
        let http = FakeHttp {
            answers: RefCell::new(vec![
                Ok(r#"{"query":{"pages":[{"title":"Tlou I","missing":true}]}}"#.into()),
                Ok(r#"["Tlou I",["The Last of Us Part I","Tlou I Remastered"],[],[]]"#.into()),
            ]),
            urls: RefCell::new(Vec::new()),
            redirect: None,
        };
        // No exact page and no exact search hit: the client refuses to guess.
        assert_eq!(PcgwClient::new(&http).find_page(Some("1888930"), "Tlou I")?, None);
        let urls = http.urls.borrow();
        assert!(urls[0].contains("action=query&titles=Tlou%20I&redirects=1"));
        assert!(urls[1].contains("action=opensearch&search=Tlou%20I"));
        Ok(())
    }

    #[test]
    fn an_existing_title_resolves_to_its_canonical_page_and_search_needs_an_exact_match() -> CoreResult<()> {
        let existing = FakeHttp {
            answers: RefCell::new(vec![Ok(r#"{"query":{"pages":[{"title":"Hollow Knight"}]}}"#.into())]),
            urls: RefCell::new(Vec::new()),
            redirect: None,
        };
        assert_eq!(PcgwClient::new(&existing).find_page(None, "hollow knight")?.as_deref(), Some("Hollow Knight"));

        let searched = FakeHttp {
            answers: RefCell::new(vec![
                Ok(r#"{"query":{"pages":[{"title":"Hades 2","missing":true}]}}"#.into()),
                Ok(r#"["Hades 2",["Hades II","Hades"],[],[]]"#.into()),
            ]),
            urls: RefCell::new(Vec::new()),
            redirect: None,
        };
        assert_eq!(PcgwClient::new(&searched).find_page(None, "Hades 2")?, None, "Hades II is not Hades 2");
        Ok(())
    }

    #[test]
    fn page_names_are_read_from_wiki_urls_and_encoded_back() {
        assert_eq!(
            page_from_url("https://www.pcgamingwiki.com/wiki/Fate/Grand_Order#Save").as_deref(),
            Some("Fate/Grand Order")
        );
        assert_eq!(page_from_url("https://example.com/wiki/Foo"), None);
        assert_eq!(page_url("Hollow Knight"), "https://www.pcgamingwiki.com/wiki/Hollow_Knight");
        assert_eq!(page_url("Fate/Grand"), "https://www.pcgamingwiki.com/wiki/Fate/Grand");
    }

    /// Talks to the real PCGamingWiki (about five requests, paced at one per second).
    /// Run by hand: `cargo test -p l8-core live_pcgamingwiki -- --ignored`.
    #[test]
    #[ignore = "needs internet access"]
    fn live_pcgamingwiki_finds_and_parses_a_real_page() -> CoreResult<()> {
        let http = crate::artwork::http::UreqHttp::new();
        let client = PcgwClient::new(&http);
        let by_steam_id = client.find_page(Some("814380"), "ignored title")?;
        assert_eq!(by_steam_id.as_deref(), Some("Sekiro: Shadows Die Twice"));
        let by_title = client.find_page(None, "Hollow Knight")?;
        assert_eq!(by_title.as_deref(), Some("Hollow Knight"));
        assert_eq!(client.find_page(None, "Surely Not A Real Game Title 12345")?, None);

        let wikitext = client.wikitext("Sekiro: Shadows Die Twice")?.ok_or(CoreError::NotFound("wikitext".into()))?;
        let paths = extract_windows_paths(&wikitext);
        assert!(!paths.is_empty(), "the page lists at least one Windows save path");
        let env = PathEnvironment::from_process(None);
        let expanded = paths.iter().filter_map(|path| expand_placeholders(&path.raw, &env)).collect::<Vec<_>>();
        assert!(expanded.iter().any(|path| path.contains("Sekiro")), "{expanded:?}");
        Ok(())
    }

    #[test]
    fn parses_the_wikitext_answer() -> CoreResult<()> {
        let http = FakeHttp {
            answers: RefCell::new(vec![Ok(r#"{"parse":{"title":"X","wikitext":"hello"}}"#.into())]),
            urls: RefCell::new(Vec::new()),
            redirect: None,
        };
        assert_eq!(PcgwClient::new(&http).wikitext("X")?.as_deref(), Some("hello"));
        Ok(())
    }
}
