//! Shared assertions for the ported enums (`#[cfg(test)]` only).

use std::fmt::Debug;
use std::str::FromStr;

/// Asserts the invariants every resolvable ported enum must satisfy against
/// its PHP-parsed tables:
///
/// * `ALL`/`INDEXES`/`LABELS` line up one-to-one;
/// * `Display` renders the PHP label;
/// * `FromStr` accepts the exact index and any case variant of it;
/// * serde round-trips through the label string.
pub(crate) fn assert_enum_roundtrip<T>(all: &[T], indexes: &[&str], labels: &[&str])
where
    T: Copy
        + Debug
        + std::fmt::Display
        + PartialEq
        + FromStr
        + serde::Serialize
        + serde::de::DeserializeOwned,
    T::Err: Debug,
{
    assert_eq!(all.len(), indexes.len(), "variant/index count mismatch");
    assert_eq!(all.len(), labels.len(), "variant/label count mismatch");

    for (i, variant) in all.iter().enumerate() {
        assert_eq!(
            variant.to_string(),
            labels[i],
            "Display for index `{}`",
            indexes[i]
        );
        assert_eq!(
            T::from_str(indexes[i]).unwrap(),
            *variant,
            "exact index `{}`",
            indexes[i]
        );
        assert_eq!(
            T::from_str(&indexes[i].to_ascii_uppercase()).unwrap(),
            *variant,
            "uppercase index `{}`",
            indexes[i]
        );

        let json = serde_json::to_string(variant).unwrap();
        assert_eq!(json, format!("\"{}\"", labels[i]), "serde label");
        assert_eq!(
            serde_json::from_str::<T>(&json).unwrap(),
            *variant,
            "serde round trip for `{}`",
            indexes[i]
        );
    }
}
