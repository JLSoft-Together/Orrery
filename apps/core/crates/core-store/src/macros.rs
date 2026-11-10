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

            pub fn merge(&mut self, other: &Self) {
                $(
                    if other.$rf.is_some() {
                        self.$rf = other.$rf.clone();
                    }
                )*
                $(
                    if other.$of.is_some() {
                        self.$of = other.$of.clone();
                    }
                )*
            }
        }

        impl $name {
            pub fn diff(&self, next: &Self) -> Option<$patch> {
                let mut patch = $patch { id: next.id.clone(), ..Default::default() };
                let mut changed = false;
                $(
                    if !crate::jseq::JsEq::js_eq(&self.$rf, &next.$rf) {
                        patch.$rf = Some(Some(next.$rf.clone()));
                        changed = true;
                    }
                )*
                $(
                    let same = match (&self.$of, &next.$of) {
                        (None, None) => true,
                        (Some(a), Some(b)) => crate::jseq::JsEq::js_eq(a, b),
                        _ => false,
                    };
                    if !same {
                        patch.$of = Some(next.$of.clone());
                        changed = true;
                    }
                )*
                changed.then_some(patch)
            }

            pub fn apply(&mut self, patch: &$patch) {
                $(
                    if let Some(Some(v)) = &patch.$rf {
                        self.$rf = v.clone();
                    }
                )*
                $(
                    if let Some(v) = &patch.$of {
                        self.$of = v.clone();
                    }
                )*
            }
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

        impl crate::jseq::JsEq for $name {
            fn js_eq(&self, other: &Self) -> bool {
                self == other
            }
        }
    };
}
