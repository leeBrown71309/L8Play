//! URL- and folder-safe identifiers derived from game titles.

/// `"Sekiro: Shadows Die Twice"` → `"sekiro-shadows-die-twice"`. Never empty.
pub fn slugify(title: &str) -> String {
    let mut slug = String::with_capacity(title.len());
    let mut last_dash = true;
    for c in title.chars().flat_map(char::to_lowercase) {
        if c.is_ascii_alphanumeric() {
            slug.push(c);
            last_dash = false;
        } else if !last_dash {
            slug.push('-');
            last_dash = true;
        }
    }
    let trimmed = slug.trim_end_matches('-');
    if trimmed.is_empty() {
        "game".to_owned()
    } else {
        trimmed.chars().take(60).collect::<String>().trim_end_matches('-').to_owned()
    }
}

/// Sort key: case-insensitive, ignoring a leading article.
pub fn sort_title(title: &str) -> String {
    let lower = title.trim().to_lowercase();
    ["the ", "a ", "an "]
        .iter()
        .find_map(|article| lower.strip_prefix(article))
        .map_or_else(|| lower.clone(), str::to_owned)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slugs_are_folder_safe() {
        assert_eq!(slugify("Sekiro: Shadows Die Twice"), "sekiro-shadows-die-twice");
        assert_eq!(slugify("Baldur's Gate 3"), "baldur-s-gate-3");
        assert_eq!(slugify("  ***  "), "game");
        assert_eq!(slugify("Élden Ring"), "lden-ring");
    }

    #[test]
    fn sort_ignores_articles() {
        assert_eq!(sort_title("The Witcher 3"), "witcher 3");
        assert_eq!(sort_title("Hades II"), "hades ii");
    }
}
