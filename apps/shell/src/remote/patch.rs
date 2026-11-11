use serde_json::{json, Value};

fn escape(key: &str) -> String {
    key.replace('~', "~0").replace('/', "~1")
}

fn walk(a: &Value, b: &Value, path: &str, out: &mut Vec<Value>) {
    if a == b {
        return;
    }
    match (a, b) {
        (Value::Object(x), Value::Object(y)) => {
            for k in x.keys().filter(|k| !y.contains_key(*k)) {
                out.push(json!({ "op": "remove", "path": format!("{path}/{}", escape(k)) }));
            }
            for (k, v) in y {
                let here = format!("{path}/{}", escape(k));
                match x.get(k) {
                    Some(old) => walk(old, v, &here, out),
                    None => out.push(json!({ "op": "add", "path": here, "value": v })),
                }
            }
        }
        (Value::Array(x), Value::Array(y)) => {
            let common = x.len().min(y.len());
            for i in 0..common {
                walk(&x[i], &y[i], &format!("{path}/{i}"), out);
            }
            for i in (common..x.len()).rev() {
                out.push(json!({ "op": "remove", "path": format!("{path}/{i}") }));
            }
            for (i, v) in y.iter().enumerate().skip(common) {
                out.push(json!({ "op": "add", "path": format!("{path}/{i}"), "value": v }));
            }
        }
        _ => out.push(json!({ "op": "replace", "path": path, "value": b })),
    }
}

pub fn diff(a: &Value, b: &Value) -> Vec<Value> {
    let mut out = Vec::new();
    walk(a, b, "", &mut out);
    out
}
