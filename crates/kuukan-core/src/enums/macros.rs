//! `macro_rules!` helpers shared by the ported `App\Enums\*` classes.
//!
//! Kept private to the enum layer; the macro itself is an implementation
//! detail and must not be used from other modules.

/// Implements one enum ported from `App\Enums\*`.
///
/// Invocation:
///
/// ```ignore
/// php_enum! {
///     /// PHP `App\Enums\AnimeTypeEnum`.
///     pub enum AnimeType = "App\\Enums\\AnimeTypeEnum" {
///         Tv => "tv" => "TV",
///         // Variant => PHP index ("@method static self <index>()") => PHP label
///     }
/// }
/// ```
///
/// Add `, php_unresolvable` after the PHP class literal for classes whose
/// definition cannot be resolved by spatie/enum 3.13.0 (duplicate labels):
/// `parse` then rejects every input, exactly like PHP's `Enum::from()`.
macro_rules! php_enum {
    (@resolves) => { true };
    (@resolves php_unresolvable) => { false };

    (
        $(#[$meta:meta])*
        pub enum $name:ident = $php_class:literal $(, $unresolvable:ident)? {
            $(
                $(#[$variant_meta:meta])*
                $variant:ident => $index:literal => $label:literal
            ),+ $(,)?
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum $name {
            $(
                $(#[$variant_meta])*
                $variant,
            )+
        }

        impl $name {
            /// PHP fully qualified class name (`App\Enums\...`), as it appears
            /// in the Laravel `EnumRule` validation message.
            pub const PHP_CLASS: &'static str = $php_class;

            /// Whether PHP can resolve this enum's definition.
            ///
            /// `false` for classes that throw `DuplicateLabelsException` while
            /// building their definition; every input is rejected for those.
            pub const RESOLVES_IN_PHP: bool = php_enum!(@resolves $($unresolvable)?);

            /// All cases, in PHP docblock order.
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            /// PHP `Enum::toValues()`: the accepted query-string indexes.
            pub const INDEXES: &'static [&'static str] = &[$($index),+];

            /// PHP labels (`$enum->label`), in docblock order.
            pub const LABELS: &'static [&'static str] = &[$($label),+];

            /// The PHP enum index — the `@method static self <index>()` name.
            ///
            /// None of the Jikan enums override `values()`, so this is also
            /// the enum's PHP `value` and the only string `Enum::from()`
            /// accepts.
            pub fn index(&self) -> &'static str {
                match self {
                    $(Self::$variant => $index,)+
                }
            }

            /// The PHP `label` property, used for storage and JSON output.
            pub fn as_str(&self) -> &'static str {
                match self {
                    $(Self::$variant => $label,)+
                }
            }

            /// PHP `Enum::from($input)`, as `Option`.
            ///
            /// The index is matched case-insensitively (PHP compares
            /// `strtolower($methodName)` with `strtolower($input)`). Labels are
            /// *not* accepted unless they coincide with the index.
            pub fn parse(input: &str) -> Option<Self> {
                if !Self::RESOLVES_IN_PHP {
                    return None;
                }

                Self::ALL
                    .iter()
                    .zip(Self::INDEXES.iter())
                    .find(|(_, index)| index.eq_ignore_ascii_case(input))
                    .map(|(variant, _)| *variant)
            }

            /// Alias of [`Self::parse`], named after the PHP index.
            pub fn from_index(input: &str) -> Option<Self> {
                Self::parse(input)
            }

            /// Variant whose PHP label equals `input`.
            ///
            /// Rust-only convenience used by serde deserialization; the
            /// duplicate labels of unresolved enums resolve to the first
            /// matching case.
            pub fn from_label(input: &str) -> Option<Self> {
                Self::ALL
                    .iter()
                    .zip(Self::LABELS.iter())
                    .find(|(_, label)| **label == input)
                    .map(|(variant, _)| *variant)
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl std::str::FromStr for $name {
            type Err = $crate::enums::EnumParseError;

            fn from_str(input: &str) -> Result<Self, Self::Err> {
                Self::parse(input)
                    .ok_or_else(|| $crate::enums::EnumParseError::new($php_class, input))
            }
        }

        impl serde::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: serde::Serializer,
            {
                serializer.serialize_str(self.as_str())
            }
        }

        impl<'de> serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                struct EnumVisitor;

                impl<'de> serde::de::Visitor<'de> for EnumVisitor {
                    type Value = $name;

                    fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                        f.write_str(concat!("a ", stringify!($name), " label"))
                    }

                    fn visit_str<E>(self, value: &str) -> Result<$name, E>
                    where
                        E: serde::de::Error,
                    {
                        $name::from_label(value)
                            .or_else(|| $name::parse(value))
                            .ok_or_else(|| {
                                E::custom(format!(
                                    "invalid {} value: {value}",
                                    stringify!($name)
                                ))
                            })
                    }
                }

                deserializer.deserialize_str(EnumVisitor)
            }
        }
    };
}
