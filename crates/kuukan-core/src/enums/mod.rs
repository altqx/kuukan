//! Enum and constant layer, ported 1:1 from `app/Enums/**` of jikan-rest
//! v4.2.2 (`App\Enums\*`), its `App\Providers\JikanEnumServiceProvider` and
//! `Jikan\Helper\Constants`.
//!
//! Jikan types query parameters as `Spatie\Enum\Laravel\Enum` subclasses and
//! resolves raw request strings with `Enum::from()`; validation goes through
//! `Spatie\Enum\Laravel\Rules\EnumRule`, casting through
//! `App\Casts\EnumCast`. `JikanEnumServiceProvider` itself only disables the
//! package's route-model-binding macro — it adds no aliases, casing rules or
//! defaults — so the mapping implemented here is exactly spatie/enum 3.13.0's
//! `from()`:
//!
//! * the PHP enum *index* (the `@method static self <index>()` docblock name)
//!   is matched case-insensitively (`strtolower($methodName) ===
//!   strtolower($input)`);
//! * the *value* is matched strictly — but none of the Jikan enums override
//!   `values()`, so value == index and this adds nothing;
//! * labels (`TV`, `Finished Airing`, `aired.from`, ...) are **not** accepted
//!   unless they happen to equal the index case-insensitively.
//!
//! Every Rust enum exposes:
//!
//! * [`as_str`](anime::AnimeType::as_str) — the PHP `label` property (storage
//!   / JSON form);
//! * [`index`](anime::AnimeType::index) — the PHP index / enum value;
//! * `parse` / [`FromStr`](std::str::FromStr) — PHP `Enum::from()` semantics;
//! * `Serialize` / `Deserialize` — serde uses the label string.
//!
//! Three PHP classes are broken in the reference implementation: their
//! `labels()` maps contain duplicates, so spatie/enum throws
//! `DuplicateLabelsException` from `resolveDefinition()` and `Enum::from()`
//! fails for *every* input (`EnumRule` catches the throwable and reports a
//! validation error, `EnumCast` throws `CannotCastEnum`). Those types have
//! `RESOLVES_IN_PHP == false` and their `parse` always returns `None`:
//! [`manga::UserMangaListOrderBy`], [`manga::UserMangaListStatusFilter`] and
//! [`anime::AnimeListAiringStatusFilter`]. This is intentional; do not
//! "repair" it without checking the PHP behavior first ("if the PHP does
//! something odd, port the oddity").
//!
//! Constants from `Jikan\Helper\Constants` live in [`constants`] and are
//! re-exported here. (`Jikan\Helper\Media` in this jikan-php revision contains
//! only YouTube URL helpers, no constants.)

#[macro_use]
mod macros;

pub mod anime;
pub mod club;
pub mod common;
pub mod constants;
pub mod manga;
pub mod search;
pub mod user;

#[cfg(test)]
mod testutil;

pub use anime::*;
pub use club::*;
pub use common::*;
pub use constants::*;
pub use manga::*;
pub use search::*;
pub use user::*;

/// Error returned when a raw request string cannot be resolved to an enum.
///
/// Mirrors the failure of `Spatie\Enum\Enum::from()` (`BadMethodCallException`)
/// and of `DuplicateLabelsException`/`DuplicateValuesException` for enums that
/// cannot be resolved at all. The HTTP layer turns this into the Laravel
/// validation error produced by `EnumRule`.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("`{value}` is not a valid {php_class}")]
pub struct EnumParseError {
    /// PHP fully qualified class name, e.g. `App\Enums\AnimeTypeEnum`.
    pub php_class: &'static str,
    /// The rejected raw input.
    pub value: String,
}

impl EnumParseError {
    /// Build an error for `value` against the given PHP enum class.
    pub fn new(php_class: &'static str, value: &str) -> Self {
        EnumParseError {
            php_class,
            value: value.to_string(),
        }
    }

    /// The exact message rendered by `Spatie\Enum\Laravel\Rules\EnumRule`
    /// (vendor lang line `enum::validation.enum`):
    /// `The {field} field is not a valid {php_class}.`
    pub fn validation_message(&self, field: &str) -> String {
        format!("The {field} field is not a valid {}.", self.php_class)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enum_error_uses_php_validation_message() {
        let err = "TV Special".parse::<AnimeType>().unwrap_err();
        assert_eq!(err.php_class, "App\\Enums\\AnimeTypeEnum");
        assert_eq!(
            err.validation_message("type"),
            "The type field is not a valid App\\Enums\\AnimeTypeEnum."
        );
    }
}
