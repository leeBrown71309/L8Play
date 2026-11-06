//! Turns a backup folder name into a plausible game title.
//!
//! Folder names are written for humans in a hurry (`appdata_local_sifu`, `DarkSoulsIII`,
//! `Absolum_SaveGame_Steam`). The result is only a suggestion: the review step lets the user correct it.

/// Words that describe the backup, not the game, when they end a folder name.
const TRAILING_NOISE: &[&str] = &["steam", "gog", "epic", "savegame", "savegames", "saves", "save", "backup", "backups"];

/// Folder names that group games instead of naming one (`save`, `My Games`…).
const CONTAINER_NAMES: &[&str] = &[
    "save", "saves", "savegame", "savegames", "save games", "saved games", "backup", "backups", "my games", "games",
    "documents", "appdata", "local", "roaming", "steam",
];

/// Suggests a title for a folder name.
pub fn clean_title(folder_name: &str) -> String {
    let mut name = strip_prefix(folder_name.trim());
    let had_separator = name.chars().any(|c| c == '_' || c.is_whitespace());
    name = fix_possessives(&name).replace('_', " ");
    if !had_separator {
        name = split_camel_case(&name);
    }
    let mut words: Vec<&str> = name.split_whitespace().collect();
    while words.len() > 1 && words.last().is_some_and(|word| TRAILING_NOISE.contains(&word.to_lowercase().as_str())) {
        words.pop();
    }
    capitalize_if_lowercase(&words.join(" "))
}

/// `true` for folder names that only hold other games (`save`, `My Games`…).
pub fn is_container_name(folder_name: &str) -> bool {
    CONTAINER_NAMES.contains(&folder_name.trim().to_lowercase().as_str())
}

/// A comparison key that ignores case, accents-free punctuation and spacing.
pub fn normalize_title(title: &str) -> String {
    title.chars().filter(|c| c.is_alphanumeric()).flat_map(char::to_lowercase).collect()
}

fn strip_prefix(name: &str) -> String {
    let lower = name.to_lowercase();
    for prefix in ["appdata_locallow_", "appdata_local_", "appdata_roaming_", "appdata_"] {
        if lower.starts_with(prefix) {
            return name.chars().skip(prefix.chars().count()).collect();
        }
    }
    name.to_owned()
}

/// `Marvel_s Spider-Man` → `Marvel's Spider-Man` (the underscore replaced an apostrophe).
fn fix_possessives(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    let mut out = String::with_capacity(name.len());
    for (index, &current) in chars.iter().enumerate() {
        let after_letter = index.checked_sub(1).and_then(|i| chars.get(i)).is_some_and(|c| c.is_alphabetic());
        let is_s = chars.get(index + 1).is_some_and(|c| c.eq_ignore_ascii_case(&'s'));
        let ends_word = chars.get(index + 2).is_none_or(|c| c.is_whitespace());
        out.push(if current == '_' && after_letter && is_s && ends_word { '\'' } else { current });
    }
    out
}

/// `DarkSoulsIII` → `Dark Souls III`, `Hellblade2` → `Hellblade 2`. Only used for names without spaces.
fn split_camel_case(name: &str) -> String {
    let chars: Vec<char> = name.chars().collect();
    let mut out = String::with_capacity(name.len() + 4);
    for (index, &current) in chars.iter().enumerate() {
        if let Some(previous) = index.checked_sub(1).and_then(|i| chars.get(i)) {
            let next = chars.get(index + 1);
            let lower_to_upper = previous.is_lowercase() && current.is_uppercase();
            let letter_to_digit = previous.is_alphabetic() && current.is_ascii_digit();
            let acronym_end = previous.is_uppercase()
                && current.is_uppercase()
                && next.is_some_and(|c| c.is_lowercase())
                && index >= 2;
            if lower_to_upper || letter_to_digit || acronym_end {
                out.push(' ');
            }
        }
        out.push(current);
    }
    out
}

/// `sifu` → `Sifu`; names that already use capitals are left alone.
fn capitalize_if_lowercase(name: &str) -> String {
    if name.chars().any(char::is_uppercase) {
        return name.to_owned();
    }
    name.split(' ')
        .enumerate()
        .map(|(index, word)| {
            let small = matches!(word, "of" | "the" | "and" | "in" | "a");
            if index > 0 && small {
                return word.to_owned();
            }
            let mut chars = word.chars();
            chars.next().map_or_else(String::new, |first| first.to_uppercase().chain(chars).collect())
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn folder_names_become_titles() {
        let cases = [
            ("appdata_local_sifu", "Sifu"),
            ("appdata_local_palworld", "Palworld"),
            ("Absolum_SaveGame_Steam", "Absolum"),
            ("DarkSoulsIII", "Dark Souls III"),
            ("DarkSoulsII", "Dark Souls II"),
            ("EldenRing", "Elden Ring"),
            ("Hellblade2", "Hellblade 2"),
            ("Remnant2", "Remnant 2"),
            ("NieR_Automata", "NieR Automata"),
            ("Marvel_s Spider-Man Miles Morales", "Marvel's Spider-Man Miles Morales"),
            ("Ghost of Tsushima DIRECTOR_S CUT", "Ghost of Tsushima DIRECTOR'S CUT"),
            ("outward", "Outward"),
            ("DSR", "DSR"),
            ("Wo long", "Wo long"),
            ("Skyrim Special Edition GOG", "Skyrim Special Edition"),
            ("Hades 2", "Hades 2"),
        ];
        for (folder, expected) in cases {
            assert_eq!(clean_title(folder), expected, "{folder}");
        }
    }

    #[test]
    fn containers_are_recognised() {
        assert!(is_container_name("save"));
        assert!(is_container_name("My Games"));
        assert!(!is_container_name("Stellar Blade"));
    }

    #[test]
    fn normalized_titles_compare_loosely() {
        assert_eq!(normalize_title("Dark Souls III"), normalize_title("DarkSoulsIII"));
        assert_ne!(normalize_title("Sifu"), normalize_title("Sekiro"));
    }
}
