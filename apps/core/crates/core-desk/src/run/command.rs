use super::args::{expand_args, quote_for_display, SubstituteVars};
use super::proc::subst_vars;
use super::types::Env;
use crate::preview::paths::resolve_str;
use serde_json::{json, Value};

fn s<'a>(cfg: &'a Value, key: &str) -> &'a str {
    cfg.get(key).and_then(Value::as_str).unwrap_or("")
}

pub fn cfg_env(cfg: &Value) -> Env {
    cfg.get("env").and_then(Value::as_object).map(|o| o.iter().filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_string()))).collect()).unwrap_or_default()
}

pub fn resolve_device(cfg: &Value, picked: Option<&str>) -> Option<String> {
    let d = s(cfg, "device");
    if d != "selected" {
        return Some(d.to_string());
    }
    picked.filter(|p| !p.is_empty()).map(String::from)
}

pub fn flutter_argv(cfg: &Value, device_id: Option<&str>, vars: &SubstituteVars, flutter_file: &str) -> Vec<String> {
    let mut out: Vec<String> = vec![flutter_file.into(), "run".into(), "--machine".into()];
    if let Some(d) = resolve_device(cfg, device_id) {
        out.extend(["-d".into(), d]);
    }
    out.extend(["-t".into(), s(cfg, "entrypoint").into()]);
    if !s(cfg, "flavor").is_empty() {
        out.extend(["--flavor".into(), s(cfg, "flavor").into()]);
    }
    out.push(format!("--{}", s(cfg, "mode")));
    out.extend(expand_args(s(cfg, "args"), vars));
    out
}

fn cap(v: &str) -> String {
    let mut c = v.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

pub fn android_gradle_args(cfg: &Value, vars: Option<&SubstituteVars>) -> Vec<String> {
    let own = SubstituteVars { project_dir: s(cfg, "cwd").into(), workspace_dir: s(cfg, "cwd").into(), env: cfg_env(cfg), mask_env: false };
    let extra = expand_args(s(cfg, "args"), vars.unwrap_or(&own));
    let mut out = vec![format!("{}:assemble{}", s(cfg, "module"), cap(s(cfg, "variant"))), "--console=plain".to_string()];
    out.extend(extra);
    out
}

pub fn custom_argv(cfg: &Value, vars: Option<&SubstituteVars>) -> Vec<String> {
    let own = SubstituteVars { project_dir: s(cfg, "cwd").into(), workspace_dir: s(cfg, "cwd").into(), env: cfg_env(cfg), mask_env: false };
    expand_args(s(cfg, "command"), vars.unwrap_or(&own))
}

pub fn define_files(argv: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < argv.len() {
        let a = &argv[i];
        if let Some(rest) = a.strip_prefix("--dart-define-from-file=") {
            out.push(rest.to_string());
        } else if a == "--dart-define-from-file" && i + 1 < argv.len() {
            i += 1;
            out.push(argv[i].clone());
        }
        i += 1;
    }
    out.into_iter().filter(|f| !f.is_empty()).collect()
}

pub struct PreviewCtx<'a> {
    pub executable: String,
    pub device_id: Option<String>,
    pub workspace_dir: String,
    pub env: &'a Env,
    pub project: Option<&'a Value>,
}

pub fn preview_command(cfg: &Value, ctx: &PreviewCtx) -> Value {
    let exists = |p: &str| std::path::Path::new(p).exists();
    let cwd = s(cfg, "cwd");
    let vars = subst_vars(cwd, &ctx.workspace_dir, ctx.env, &cfg_env(cfg));
    let shown = SubstituteVars { mask_env: true, ..vars.clone() };
    let mut errors: Vec<Value> = Vec::new();
    let device = ctx.device_id.as_deref();
    let argv: Vec<String>;
    match s(cfg, "type") {
        "flutter" => {
            argv = flutter_argv(cfg, device, &shown, &ctx.executable);
            if !exists(&resolve_str(&format!("{cwd}/{}", s(cfg, "entrypoint")))) {
                errors.push(json!({"code": "entrypoint-missing", "detail": s(cfg, "entrypoint")}));
            }
            for f in define_files(&expand_args(s(cfg, "args"), &vars)) {
                let full = if crate::preview::paths::is_absolute(&f) { resolve_str(&f) } else { resolve_str(&format!("{cwd}/{f}")) };
                if !exists(&full) {
                    errors.push(json!({"code": "define-file-missing", "detail": f}));
                }
            }
            if let Some(p) = ctx.project.filter(|p| p["kind"] == "flutter") {
                let flavors = p["flavors"].as_array().map(Vec::as_slice).unwrap_or(&[]);
                if !s(cfg, "flavor").is_empty() && !flavors.is_empty() && !flavors.iter().any(|f| f == s(cfg, "flavor")) {
                    errors.push(json!({"code": "flavor-unknown", "detail": s(cfg, "flavor")}));
                }
            }
            if resolve_device(cfg, device).is_none() {
                errors.push(json!({"code": "device-missing"}));
            }
        }
        "android" => {
            let mut a = vec![ctx.executable.clone()];
            a.extend(android_gradle_args(cfg, Some(&shown)));
            argv = a;
            if let Some(p) = ctx.project.filter(|p| p["kind"] == "android") {
                let has = p["modules"].as_array().is_some_and(|m| m.iter().any(|x| x["path"] == cfg["module"]));
                if !has {
                    errors.push(json!({"code": "module-unknown", "detail": s(cfg, "module")}));
                }
            }
            if resolve_device(cfg, device).is_none() {
                errors.push(json!({"code": "device-missing"}));
            }
        }
        _ => {
            argv = custom_argv(cfg, Some(&shown));
            if argv.is_empty() {
                errors.push(json!({"code": "command-empty"}));
            }
        }
    }
    json!({"argv": argv, "command": quote_for_display(&argv), "errors": errors})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn previews_flutter_and_custom() {
        let mut env = Env::new();
        env.insert("SECRET".into(), "s3".into());
        let cfg = json!({"type": "flutter", "cwd": "/nonexistent/p", "entrypoint": "lib/main.dart", "device": "selected", "mode": "profile", "flavor": "dev", "args": "--dart-define=K=${env:SECRET} --dart-define-from-file=e.json", "env": {}});
        let ctx = PreviewCtx { executable: "/sdk/bin/flutter".into(), device_id: None, workspace_dir: "/nonexistent".into(), env: &env, project: Some(&json!({"kind": "flutter", "flavors": ["prod"]})) };
        let v = preview_command(&cfg, &ctx);
        assert_eq!(v["argv"][0], "/sdk/bin/flutter");
        assert!(v["command"].as_str().unwrap().contains("--flavor dev --profile --dart-define=K=***"));
        let codes: Vec<&str> = v["errors"].as_array().unwrap().iter().map(|e| e["code"].as_str().unwrap()).collect();
        assert_eq!(codes, ["entrypoint-missing", "define-file-missing", "flavor-unknown", "device-missing"]);
        let custom = preview_command(&json!({"type": "custom", "cwd": "/p", "command": "", "env": {}}), &PreviewCtx { executable: String::new(), device_id: None, workspace_dir: "/p".into(), env: &env, project: None });
        assert_eq!(custom["errors"], json!([{"code": "command-empty"}]));
    }

    #[test]
    fn android_args() {
        let cfg = json!({"cwd": "/p", "module": ":app", "variant": "freeDebug", "args": "-Pa=1", "env": {}});
        assert_eq!(android_gradle_args(&cfg, None), [":app:assembleFreeDebug", "--console=plain", "-Pa=1"]);
        assert_eq!(define_files(&["--dart-define-from-file=a".into(), "--dart-define-from-file".into(), "b".into()]), ["a", "b"]);
    }
}
