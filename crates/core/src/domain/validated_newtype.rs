macro_rules! validated_string_newtype {
    ($(#[$meta:meta])* $name:ident, $kind:literal, $max:expr, $validator:path, try_from) => {
        validated_string_newtype!(@base $(#[$meta])* $name, $kind, $max, $validator);

        impl TryFrom<String> for $name {
            type Error = $crate::domain::ValidationError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }
    };
    ($(#[$meta:meta])* $name:ident, $kind:literal, $max:expr, $validator:path) => {
        validated_string_newtype!(@base $(#[$meta])* $name, $kind, $max, $validator);
    };
    (@base $(#[$meta:meta])* $name:ident, $kind:literal, $max:expr, $validator:path) => {
        $(#[$meta])*
        #[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, $crate::domain::ValidationError> {
                let value = value.into();
                $validator(&value, $kind, $max)?;
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl ::std::fmt::Display for $name {
            fn fmt(&self, formatter: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                formatter.write_str(&self.0)
            }
        }

        impl ::serde::Serialize for $name {
            fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where
                S: ::serde::Serializer,
            {
                serializer.serialize_str(&self.0)
            }
        }

        impl<'de> ::serde::Deserialize<'de> for $name {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: ::serde::Deserializer<'de>,
            {
                let value = <String as ::serde::Deserialize>::deserialize(deserializer)?;
                Self::new(value).map_err(::serde::de::Error::custom)
            }
        }
    };
}

pub(crate) use validated_string_newtype;
