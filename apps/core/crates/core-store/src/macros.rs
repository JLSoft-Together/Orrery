macro_rules! record {
    (
        $(#[$meta:meta])*
        pub struct $name:ident {
            req { $($rf:ident : $rt:ty),* $(,)? }
            opt { $($of:ident : $ot:ty),* $(,)? }
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        pub struct $name {
            $(pub $rf: $rt,)*
            $(
                #[serde(default, skip_serializing_if = "Option::is_none")]
                pub $of: Option<$ot>,
            )*
        }

        impl $name {
            pub const FIELDS: &'static [&'static str] = &[$(stringify!($rf),)* $(stringify!($of),)*];
        }
    };
}

macro_rules! entity {
    (
        $(#[$meta:meta])*
        pub struct $name:ident / $patch:ident {
            req { $($rf:ident : $rt:ty),* $(,)? }
            opt { $($of:ident : $ot:ty),* $(,)? }
        }
    ) => {
        record! {
            $(#[$meta])*
            pub struct $name {
                req { id: String, $($rf: $rt),* }
                opt { $($of: $ot),* }
            }
        }

        #[derive(Debug, Clone, Default, PartialEq, serde::Serialize, serde::Deserialize)]
        #[serde(rename_all = "camelCase", deny_unknown_fields)]
        pub struct $patch {
            pub id: String,
            $(
                #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "crate::num::nullable")]
                pub $rf: Option<Option<$rt>>,
            )*
            $(
                #[serde(default, skip_serializing_if = "Option::is_none", deserialize_with = "crate::num::nullable")]
                pub $of: Option<Option<$ot>>,
            )*
        }

        impl $patch {
            pub const FIELDS: &'static [&'static str] = &["id", $(stringify!($rf),)* $(stringify!($of),)*];
        }
    };
}

macro_rules! string_enum {
    (
        $(#[$meta:meta])*
        pub enum $name:ident { $($variant:ident = $text:literal),* $(,)? }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
        pub enum $name {
            $(#[serde(rename = $text)] $variant,)*
        }

        impl $name {
            pub const ALL: &'static [&'static str] = &[$($text,)*];

            pub fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $text,)*
                }
            }
        }
    };
}
