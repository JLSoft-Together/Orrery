use std::sync::Arc;

use crate::num::JsNum;

pub trait JsEq {
    fn js_eq(&self, other: &Self) -> bool;
}

macro_rules! by_value {
    ($($t:ty),*) => {
        $(
            impl JsEq for $t {
                fn js_eq(&self, other: &Self) -> bool {
                    self == other
                }
            }
        )*
    };
}

by_value!(String, u64, i64, bool);

impl JsEq for JsNum {
    fn js_eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<T> JsEq for Arc<T> {
    fn js_eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(self, other)
    }
}

#[cfg(test)]
mod tests {
    use super::JsEq;
    use crate::num::JsNum;
    use std::sync::Arc;

    #[test]
    fn numbers_compare_like_triple_equals() {
        assert!(JsNum(0.0).js_eq(&JsNum(-0.0)));
        assert!(!JsNum(f64::NAN).js_eq(&JsNum(f64::NAN)));
    }

    #[test]
    fn shared_values_compare_by_identity() {
        let a = Arc::new(vec!["Bash".to_string()]);
        let b = Arc::new(vec!["Bash".to_string()]);
        assert!(a.js_eq(&a.clone()));
        assert!(!a.js_eq(&b));
    }
}
