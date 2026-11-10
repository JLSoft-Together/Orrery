use serde::{Deserialize, Deserializer, Serialize, Serializer};

const MAX_SAFE: f64 = 9_007_199_254_740_991.0;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default)]
pub struct JsNum(pub f64);

impl From<f64> for JsNum {
    fn from(v: f64) -> Self {
        Self(v)
    }
}

impl From<u64> for JsNum {
    fn from(v: u64) -> Self {
        Self(v as f64)
    }
}

impl Serialize for JsNum {
    fn serialize<S: Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        let v = self.0;
        if !v.is_finite() {
            return s.serialize_unit();
        }
        if v.fract() == 0.0 && v.abs() <= MAX_SAFE {
            return if v >= 0.0 { s.serialize_u64(v as u64) } else { s.serialize_i64(v as i64) };
        }
        s.serialize_f64(v)
    }
}

impl<'de> Deserialize<'de> for JsNum {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        f64::deserialize(d).map(Self)
    }
}

pub fn nullable<'de, D, T>(d: D) -> Result<Option<Option<T>>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(d).map(Some)
}

#[cfg(test)]
mod tests {
    use super::JsNum;

    #[test]
    fn integral_values_print_like_javascript() {
        let out = serde_json::to_string(&[JsNum(42.0), JsNum(-0.0), JsNum(3.4), JsNum(-7.0), JsNum(1e300), JsNum(f64::NAN)]).unwrap();
        assert_eq!(out, "[42,0,3.4,-7,1e+300,null]");
    }

    #[test]
    fn reads_integers_and_floats() {
        let v: Vec<JsNum> = serde_json::from_str("[1, 2.5, -3]").unwrap();
        assert_eq!(v, vec![JsNum(1.0), JsNum(2.5), JsNum(-3.0)]);
    }
}
