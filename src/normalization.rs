use serde::{Deserialize, Deserializer};

/// Normalizes a name by stripping any whitespace and decomposing it
/// into Unicode Normalization Form D.
///
/// ```
/// use backend::normalization::normalize_name;
/// assert_eq!(normalize_name(" hï "), "hï");
/// ```
pub fn normalize_name(name: impl AsRef<str>) -> String {
    use unicode_normalization::UnicodeNormalization;

    name.as_ref().trim().nfd().to_string()
}

/// Deserializes a `String` after running it through `normalize_name`.
pub fn deserialize<'de, D>(deserializer: D) -> Result<String, D::Error>
where D: Deserializer<'de> {
    let s: &str = Deserialize::deserialize(deserializer)?;
    Ok(normalize_name(s))
}

/// Deserializes an optional `String` after running it through `normalize_name`.
pub fn deserialize_option<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where D: Deserializer<'de> {
    let o: Option<&str> = Deserialize::deserialize(deserializer)?;
    Ok(o.map(normalize_name))
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;
    use unicode_normalization::is_nfd;

    use super::normalize_name;

    fn count_whitespace(s: impl AsRef<str>) -> usize {
        s.as_ref().chars().filter(|c| c.is_whitespace()).count()
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 10000, ..ProptestConfig::default()
        })]

        #[test]
        fn normalization_works(string in "(\\S.*\\S|\\S+)", space_before in "\\s*", space_after in "\\s*") {
            let normalized = normalize_name(format!("{}{}{}", space_before, string, space_after));

            prop_assert!(is_nfd(&normalized), "{:?} (normalized form of {:?}) is in NFD", normalized, string);

            prop_assert!(!normalized.starts_with(char::is_whitespace) && !normalized.ends_with(char::is_whitespace), "{:?} (normalized form of {:?}) has no leading or trailing whitespace", normalized, string);

            let trimmed = normalized.trim();

            prop_assert_eq!(count_whitespace(&normalized), count_whitespace(&trimmed), "{:?} (normalized form of {:?}) preserves inner whitespace", normalized, string);
        }
    }
}
