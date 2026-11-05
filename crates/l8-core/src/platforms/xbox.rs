//! Xbox app (Microsoft Store / Game Pass for PC) discovery.
//!
//! The Xbox app installs each game in `<games root>\<Title>\Content`, where `MicrosoftGame.config`
//! (and `appxmanifest.xml`) describe the package. The games root is `\XboxGames` on a drive by
//! default; the drive's `.GamingRoot` file records the folder when the player chose another one.
//! Everything is read-only; the protected `WindowsApps` folder is never touched.

use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::error::CoreResult;
use crate::platforms::{DiscoveredGame, Discovery, LibraryProvider, Platform};

const GAMING_ROOT_FILE: &str = ".GamingRoot";
const DEFAULT_ROOT_FOLDER: &str = "XboxGames";
const PACKAGE_CONFIG: &str = "MicrosoftGame.config";
const PACKAGE_MANIFEST: &str = "appxmanifest.xml";
/// Crockford-style base32 alphabet used by Windows for the publisher id of a package family name.
const PUBLISHER_ID_ALPHABET: &[u8; 32] = b"0123456789abcdefghjkmnpqrstvwxyz";

pub struct XboxProvider {
    roots: Vec<PathBuf>,
}

impl XboxProvider {
    /// Looks for Xbox games roots on every drive letter.
    pub fn from_environment() -> Self {
        let drives = ('C'..='Z').map(|letter| PathBuf::from(format!("{letter}:\\")));
        Self::with_roots(roots_on_drives(drives))
    }

    pub fn with_roots(roots: Vec<PathBuf>) -> Self {
        Self { roots }
    }
}

/// The Xbox games root of each drive that has one.
pub fn roots_on_drives(drives: impl IntoIterator<Item = PathBuf>) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    for drive in drives {
        let marker = drive.join(GAMING_ROOT_FILE);
        let folder = match std::fs::read(&marker) {
            Ok(bytes) => parse_gaming_root(&bytes).unwrap_or_else(|| DEFAULT_ROOT_FOLDER.to_owned()),
            // No marker: the default folder still counts when it exists.
            Err(_) => DEFAULT_ROOT_FOLDER.to_owned(),
        };
        let root = drive.join(folder);
        if root.is_dir() && !roots.contains(&root) {
            roots.push(root);
        }
    }
    roots
}

/// `.GamingRoot` is the bytes `RGBX`, a little-endian `u32` version, then the folder name in
/// UTF-16LE ending with a NUL.
fn parse_gaming_root(bytes: &[u8]) -> Option<String> {
    let body = bytes.strip_prefix(b"RGBX")?.get(4..)?;
    let units: Vec<u16> =
        body.as_chunks::<2>().0.iter().map(|pair| u16::from_le_bytes(*pair)).take_while(|unit| *unit != 0).collect();
    let folder = String::from_utf16(&units).ok()?;
    let folder = folder.trim().trim_matches('\\');
    (!folder.is_empty()).then(|| folder.to_owned())
}

impl LibraryProvider for XboxProvider {
    fn platform(&self) -> Platform {
        Platform::Xbox
    }

    fn discover(&self) -> CoreResult<Option<Discovery>> {
        if self.roots.is_empty() {
            tracing::info!("no Xbox games folder found; skipping Xbox discovery");
            return Ok(None);
        }
        let mut discovery = Discovery { library_paths: self.roots.clone(), ..Discovery::default() };
        for root in &self.roots {
            let entries = match std::fs::read_dir(root) {
                Ok(entries) => entries,
                Err(error) => {
                    discovery.warnings.push(format!("Xbox games folder skipped ({}): {error}", root.display()));
                    continue;
                }
            };
            for entry in entries.flatten() {
                let folder = entry.path();
                if !folder.is_dir() {
                    continue;
                }
                match read_package(&folder) {
                    Ok(Some(game)) => discovery.games.push(game),
                    Ok(None) => {}
                    Err(reason) => {
                        discovery.warnings.push(format!("Xbox game skipped ({}): {reason}", folder.display()));
                    }
                }
            }
        }
        Ok(Some(discovery))
    }
}

/// Reads one `<root>\<Title>` folder. `Ok(None)` for folders that are not game packages
/// (for example the `GameSave` folder the Xbox app keeps in the same place).
fn read_package(folder: &Path) -> Result<Option<DiscoveredGame>, String> {
    let content = folder.join("Content");
    let config = std::fs::read_to_string(content.join(PACKAGE_CONFIG)).ok();
    let manifest = std::fs::read_to_string(content.join(PACKAGE_MANIFEST)).ok();
    if config.is_none() && manifest.is_none() {
        return Ok(None);
    }
    let identity = [config.as_deref(), manifest.as_deref()]
        .into_iter()
        .flatten()
        .find_map(|xml| find_tags(xml, "Identity").into_iter().next())
        .ok_or("the package has no Identity")?;
    let name = attribute(identity, "Name").ok_or("the package identity has no name")?;
    let publisher = attribute(identity, "Publisher");

    let executable = config.as_deref().and_then(|xml| {
        find_tags(xml, "Executable")
            .into_iter()
            .filter(|tag| attribute(tag, "TargetDeviceFamily").is_none_or(|family| family.eq_ignore_ascii_case("PC")))
            .find_map(|tag| attribute(tag, "Name").map(|exe| (exe, attribute(tag, "Id"))))
    });
    let app_id = executable.as_ref().and_then(|(_, id)| id.clone()).or_else(|| {
        manifest.as_deref().and_then(|xml| find_tags(xml, "Application").into_iter().find_map(|t| attribute(t, "Id")))
    });
    let executable_path = executable.map(|(exe, _)| content.join(exe)).filter(|path| path.is_file());

    let platform_game_id = match (&publisher, &app_id) {
        (Some(publisher), Some(app_id)) => format!("{name}_{}!{app_id}", publisher_id(publisher)),
        _ => name.clone(),
    };
    let title = display_name(config.as_deref(), manifest.as_deref()).unwrap_or_else(|| folder_title(folder));
    Ok(Some(DiscoveredGame {
        platform_game_id,
        title,
        install_path: folder.to_path_buf(),
        executable_path,
        size_bytes: None,
        last_played_at: None,
        playtime_seconds: None,
    }))
}

/// A real title: `ms-resource:` values point into a resource file that is not read here.
fn display_name(config: Option<&str>, manifest: Option<&str>) -> Option<String> {
    let from_config = config.and_then(|xml| {
        find_tags(xml, "ShellVisuals").into_iter().find_map(|tag| attribute(tag, "DefaultDisplayName"))
    });
    let from_manifest = manifest.and_then(|xml| element_text(xml, "DisplayName"));
    [from_config, from_manifest].into_iter().flatten().map(|text| text.trim().to_owned()).find(|text| is_title(text))
}

fn is_title(text: &str) -> bool {
    !text.is_empty() && !text.to_ascii_lowercase().starts_with("ms-resource:")
}

fn folder_title(folder: &Path) -> String {
    folder.file_name().map_or_else(|| "Xbox game".to_owned(), |name| name.to_string_lossy().into_owned())
}

/// The package family name's publisher id: the first 8 bytes of SHA-256 over the UTF-16LE
/// publisher string, written in base32 (13 characters, the last one carrying a zero pad bit).
pub fn publisher_id(publisher: &str) -> String {
    let bytes: Vec<u8> = publisher.encode_utf16().flat_map(u16::to_le_bytes).collect();
    let digest = Sha256::digest(&bytes);
    let mut bits = 0_u128;
    for byte in &digest[..8] {
        bits = (bits << 8) | u128::from(*byte);
    }
    bits <<= 1; // 64 bits + 1 zero pad bit = 13 groups of 5 bits
    (0..13)
        .rev()
        .map(|group| {
            let index = usize::try_from((bits >> (group * 5)) & 0x1f).unwrap_or(0);
            char::from(PUBLISHER_ID_ALPHABET[index])
        })
        .collect()
}

/// Opening text of every `<tag …>` element (up to its closing `>`), in document order.
fn find_tags<'a>(xml: &'a str, tag: &str) -> Vec<&'a str> {
    let open = format!("<{tag}");
    let mut found = Vec::new();
    let mut rest = xml;
    while let Some(start) = rest.find(&open) {
        let after = &rest[start + open.len()..];
        let boundary = after.chars().next().is_some_and(|c| c.is_whitespace() || c == '/' || c == '>');
        let end = after.find('>').unwrap_or(after.len());
        if boundary {
            found.push(&after[..end]);
        }
        rest = &after[end.min(after.len())..];
    }
    found
}

/// Value of `name="…"` (or `'…'`) inside an opening tag, with the XML entities decoded.
fn attribute(tag: &str, name: &str) -> Option<String> {
    let mut rest = tag;
    while let Some(position) = rest.find(name) {
        let before_ok = rest[..position].chars().next_back().is_none_or(char::is_whitespace);
        let after = &rest[position + name.len()..];
        let after_name = after.trim_start();
        if before_ok && let Some(value) = after_name.strip_prefix('=') {
            let value = value.trim_start();
            let quote = value.chars().next().filter(|c| *c == '"' || *c == '\'')?;
            let inner = &value[1..];
            let end = inner.find(quote)?;
            return Some(decode_entities(&inner[..end]));
        }
        rest = after;
    }
    None
}

/// Text of the first `<tag>…</tag>` element.
fn element_text(xml: &str, tag: &str) -> Option<String> {
    let start = xml.find(&format!("<{tag}>"))? + tag.len() + 2;
    let end = xml[start..].find(&format!("</{tag}>"))?;
    Some(decode_entities(&xml[start..start + end]))
}

fn decode_entities(text: &str) -> String {
    text.replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&apos;", "'").replace("&amp;", "&")
}

#[cfg(test)]
mod tests {
    use super::*;

    const MICROSOFT_PUBLISHER: &str =
        "CN=Microsoft Corporation, O=Microsoft Corporation, L=Redmond, S=Washington, C=US";

    const CONFIG: &str = r#"<?xml version="1.0" encoding="utf-8"?>
<Game configVersion="1">
  <Identity Name="Microsoft.MinecraftDungeons2" Publisher="CN=Microsoft Corporation, O=Microsoft Corporation, L=Redmond, S=Washington, C=US" Version="1.1.3.0" />
  <ShellVisuals DefaultDisplayName="Minecraft &amp; Dungeons II" PublisherDisplayName="Microsoft Studios" />
  <ExecutableList>
    <Executable Name="Dungeons.exe" Id="AppMinecraftDungeonsIIShipping" TargetDeviceFamily="PC" />
  </ExecutableList>
</Game>"#;

    fn install(root: &Path, folder: &str, config: Option<&str>) -> Result<PathBuf, Box<dyn std::error::Error>> {
        let game = root.join(folder);
        let content = game.join("Content");
        std::fs::create_dir_all(&content)?;
        if let Some(config) = config {
            std::fs::write(content.join(PACKAGE_CONFIG), config)?;
            std::fs::write(content.join("Dungeons.exe"), b"MZ")?;
        }
        Ok(game)
    }

    #[test]
    fn publisher_id_matches_the_microsoft_package_family() {
        assert_eq!(publisher_id(MICROSOFT_PUBLISHER), "8wekyb3d8bbwe");
    }

    #[test]
    fn gaming_root_names_the_games_folder() {
        let mut bytes = b"RGBX\x01\x00\x00\x00".to_vec();
        bytes.extend("Workspace\\Games".encode_utf16().flat_map(u16::to_le_bytes));
        bytes.extend([0, 0]);
        assert_eq!(parse_gaming_root(&bytes).as_deref(), Some("Workspace\\Games"));
        assert_eq!(parse_gaming_root(b"junk"), None);
        assert_eq!(parse_gaming_root(b"RGBX\x01\x00\x00\x00\x00\x00"), None);
    }

    #[test]
    fn roots_follow_the_marker_then_the_default_folder() -> Result<(), Box<dyn std::error::Error>> {
        let custom = tempfile::tempdir()?;
        std::fs::create_dir_all(custom.path().join("Workspace").join("Games"))?;
        let mut marker = b"RGBX\x01\x00\x00\x00".to_vec();
        marker.extend("Workspace\\Games".encode_utf16().flat_map(u16::to_le_bytes));
        marker.extend([0, 0]);
        std::fs::write(custom.path().join(GAMING_ROOT_FILE), marker)?;
        let default = tempfile::tempdir()?;
        std::fs::create_dir_all(default.path().join(DEFAULT_ROOT_FOLDER))?;
        let empty = tempfile::tempdir()?;

        let roots = roots_on_drives([
            custom.path().to_path_buf(),
            default.path().to_path_buf(),
            empty.path().to_path_buf(),
            PathBuf::from(r"Z:\definitely\not\here"),
        ]);
        assert_eq!(
            roots,
            vec![custom.path().join("Workspace").join("Games"), default.path().join(DEFAULT_ROOT_FOLDER)]
        );
        Ok(())
    }

    #[test]
    fn discovers_packages_and_skips_other_folders() -> Result<(), Box<dyn std::error::Error>> {
        let root = tempfile::tempdir()?;
        let game = install(root.path(), "Minecraft Dungeons II", Some(CONFIG))?;
        install(root.path(), "GameSave", None)?; // no package files
        std::fs::create_dir_all(root.path().join("Loose"))?; // no Content folder
        install(root.path(), "Broken", Some("<Game></Game>"))?; // package without identity

        let discovery =
            XboxProvider::with_roots(vec![root.path().to_path_buf()]).discover()?.ok_or("a root was given")?;
        assert_eq!(discovery.games.len(), 1);
        let found = &discovery.games[0];
        assert_eq!(found.title, "Minecraft & Dungeons II");
        assert_eq!(found.platform_game_id, "Microsoft.MinecraftDungeons2_8wekyb3d8bbwe!AppMinecraftDungeonsIIShipping");
        assert_eq!(found.install_path, game);
        assert_eq!(found.executable_path, Some(game.join("Content").join("Dungeons.exe")));
        assert_eq!(discovery.warnings.len(), 1, "the package without identity is reported");
        Ok(())
    }

    #[test]
    fn resource_titles_fall_back_to_the_folder_name() -> Result<(), Box<dyn std::error::Error>> {
        let root = tempfile::tempdir()?;
        let config = CONFIG.replace("Minecraft &amp; Dungeons II", "ms-resource:AppName");
        install(root.path(), "Dungeons Two", Some(&config))?;
        let discovery =
            XboxProvider::with_roots(vec![root.path().to_path_buf()]).discover()?.ok_or("a root was given")?;
        assert_eq!(discovery.games[0].title, "Dungeons Two");
        Ok(())
    }

    #[test]
    fn no_root_means_the_app_is_absent() -> CoreResult<()> {
        assert!(XboxProvider::with_roots(Vec::new()).discover()?.is_none());
        Ok(())
    }

    /// Reads this PC's real Xbox folders: `cargo test -p l8-core xbox_live -- --ignored --nocapture`.
    #[test]
    #[ignore = "reads the real disks of this PC"]
    fn xbox_live() -> CoreResult<()> {
        let discovery = XboxProvider::from_environment().discover()?;
        println!("{discovery:#?}");
        Ok(())
    }
}
