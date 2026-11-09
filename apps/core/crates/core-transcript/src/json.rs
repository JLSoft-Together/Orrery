use serde::de::{Deserialize, Deserializer, IgnoredAny, MapAccess, SeqAccess, Visitor};
use std::fmt;
use std::marker::PhantomData;

pub trait FromObj: Default {
    fn field<'de, A: MapAccess<'de>>(&mut self, key: &str, map: &mut A) -> Result<(), A::Error>;
}

fn skip<'de, A: MapAccess<'de>>(map: &mut A) -> Result<(), A::Error> {
    map.next_value::<IgnoredAny>().map(|_| ())
}

fn drain_map<'de, A: MapAccess<'de>>(mut map: A) -> Result<(), A::Error> {
    while map.next_entry::<IgnoredAny, IgnoredAny>()?.is_some() {}
    Ok(())
}

fn drain_seq<'de, A: SeqAccess<'de>>(mut seq: A) -> Result<(), A::Error> {
    while seq.next_element::<IgnoredAny>()?.is_some() {}
    Ok(())
}

pub struct Key(pub String);

impl<'de> Deserialize<'de> for Key {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        String::deserialize(d).map(Key)
    }
}

#[derive(Default)]
pub struct JStr(pub Option<String>);

impl<'de> Deserialize<'de> for JStr {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = JStr;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("any")
            }
            fn visit_str<E>(self, s: &str) -> Result<JStr, E> {
                Ok(JStr(Some(s.to_owned())))
            }
            fn visit_string<E>(self, s: String) -> Result<JStr, E> {
                Ok(JStr(Some(s)))
            }
            fn visit_bool<E>(self, _: bool) -> Result<JStr, E> {
                Ok(JStr(None))
            }
            fn visit_i64<E>(self, _: i64) -> Result<JStr, E> {
                Ok(JStr(None))
            }
            fn visit_u64<E>(self, _: u64) -> Result<JStr, E> {
                Ok(JStr(None))
            }
            fn visit_f64<E>(self, _: f64) -> Result<JStr, E> {
                Ok(JStr(None))
            }
            fn visit_unit<E>(self) -> Result<JStr, E> {
                Ok(JStr(None))
            }
            fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<JStr, A::Error> {
                drain_map(map).map(|_| JStr(None))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<JStr, A::Error> {
                drain_seq(seq).map(|_| JStr(None))
            }
        }
        d.deserialize_any(V)
    }
}

#[derive(Default, Clone, Copy)]
pub struct JNum(pub f64);

impl<'de> Deserialize<'de> for JNum {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = JNum;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("any")
            }
            fn visit_bool<E>(self, _: bool) -> Result<JNum, E> {
                Ok(JNum(0.0))
            }
            fn visit_i64<E>(self, v: i64) -> Result<JNum, E> {
                Ok(JNum(if v >= 0 { v as f64 } else { 0.0 }))
            }
            fn visit_u64<E>(self, v: u64) -> Result<JNum, E> {
                Ok(JNum(v as f64))
            }
            fn visit_f64<E>(self, v: f64) -> Result<JNum, E> {
                Ok(JNum(if v.is_finite() && v > 0.0 { v } else { 0.0 }))
            }
            fn visit_str<E>(self, _: &str) -> Result<JNum, E> {
                Ok(JNum(0.0))
            }
            fn visit_unit<E>(self) -> Result<JNum, E> {
                Ok(JNum(0.0))
            }
            fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<JNum, A::Error> {
                drain_map(map).map(|_| JNum(0.0))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<JNum, A::Error> {
                drain_seq(seq).map(|_| JNum(0.0))
            }
        }
        d.deserialize_any(V)
    }
}

#[derive(Default)]
pub struct JTrue(pub bool);

impl<'de> Deserialize<'de> for JTrue {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V;
        impl<'de> Visitor<'de> for V {
            type Value = JTrue;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("any")
            }
            fn visit_bool<E>(self, b: bool) -> Result<JTrue, E> {
                Ok(JTrue(b))
            }
            fn visit_i64<E>(self, _: i64) -> Result<JTrue, E> {
                Ok(JTrue(false))
            }
            fn visit_u64<E>(self, _: u64) -> Result<JTrue, E> {
                Ok(JTrue(false))
            }
            fn visit_f64<E>(self, _: f64) -> Result<JTrue, E> {
                Ok(JTrue(false))
            }
            fn visit_str<E>(self, _: &str) -> Result<JTrue, E> {
                Ok(JTrue(false))
            }
            fn visit_unit<E>(self) -> Result<JTrue, E> {
                Ok(JTrue(false))
            }
            fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<JTrue, A::Error> {
                drain_map(map).map(|_| JTrue(false))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<JTrue, A::Error> {
                drain_seq(seq).map(|_| JTrue(false))
            }
        }
        d.deserialize_any(V)
    }
}

pub struct Obj<T>(pub Option<T>);

impl<T> Default for Obj<T> {
    fn default() -> Self {
        Obj(None)
    }
}

impl<'de, T: FromObj> Deserialize<'de> for Obj<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V<T>(PhantomData<T>);
        impl<'de, T: FromObj> Visitor<'de> for V<T> {
            type Value = Obj<T>;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("any")
            }
            fn visit_str<E>(self, _: &str) -> Result<Obj<T>, E> {
                Ok(Obj(None))
            }
            fn visit_bool<E>(self, _: bool) -> Result<Obj<T>, E> {
                Ok(Obj(None))
            }
            fn visit_i64<E>(self, _: i64) -> Result<Obj<T>, E> {
                Ok(Obj(None))
            }
            fn visit_u64<E>(self, _: u64) -> Result<Obj<T>, E> {
                Ok(Obj(None))
            }
            fn visit_f64<E>(self, _: f64) -> Result<Obj<T>, E> {
                Ok(Obj(None))
            }
            fn visit_unit<E>(self) -> Result<Obj<T>, E> {
                Ok(Obj(None))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, seq: A) -> Result<Obj<T>, A::Error> {
                drain_seq(seq).map(|_| Obj(None))
            }
            fn visit_map<A: MapAccess<'de>>(self, mut map: A) -> Result<Obj<T>, A::Error> {
                let mut t = T::default();
                while let Some(Key(k)) = map.next_key::<Key>()? {
                    t.field(&k, &mut map)?;
                }
                Ok(Obj(Some(t)))
            }
        }
        d.deserialize_any(V(PhantomData))
    }
}

pub struct Arr<T>(pub Option<Vec<T>>);

impl<T> Default for Arr<T> {
    fn default() -> Self {
        Arr(None)
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Arr<T> {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        struct V<T>(PhantomData<T>);
        impl<'de, T: Deserialize<'de>> Visitor<'de> for V<T> {
            type Value = Arr<T>;
            fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
                f.write_str("any")
            }
            fn visit_str<E>(self, _: &str) -> Result<Arr<T>, E> {
                Ok(Arr(None))
            }
            fn visit_bool<E>(self, _: bool) -> Result<Arr<T>, E> {
                Ok(Arr(None))
            }
            fn visit_i64<E>(self, _: i64) -> Result<Arr<T>, E> {
                Ok(Arr(None))
            }
            fn visit_u64<E>(self, _: u64) -> Result<Arr<T>, E> {
                Ok(Arr(None))
            }
            fn visit_f64<E>(self, _: f64) -> Result<Arr<T>, E> {
                Ok(Arr(None))
            }
            fn visit_unit<E>(self) -> Result<Arr<T>, E> {
                Ok(Arr(None))
            }
            fn visit_map<A: MapAccess<'de>>(self, map: A) -> Result<Arr<T>, A::Error> {
                drain_map(map).map(|_| Arr(None))
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Arr<T>, A::Error> {
                let mut out = Vec::new();
                while let Some(x) = seq.next_element::<T>()? {
                    out.push(x);
                }
                Ok(Arr(Some(out)))
            }
        }
        d.deserialize_any(V(PhantomData))
    }
}

pub fn value<'de, A: MapAccess<'de>, T: Deserialize<'de>>(map: &mut A) -> Result<T, A::Error> {
    map.next_value::<T>()
}

pub fn ignore<'de, A: MapAccess<'de>>(map: &mut A) -> Result<(), A::Error> {
    skip(map)
}
