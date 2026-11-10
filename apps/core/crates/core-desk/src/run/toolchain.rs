use crate::preview::spawn::{run, End, Opts, Spec};
use regex::Regex;
use serde_json::{json, Value};
use std::sync::OnceLock;

pub struct ExecOut {
    pub stdout: String,
    pub stderr: String,
    pub timed_out: bool,
}

pub type ToolchainExec<'a> = &'a dyn Fn(&str, &[String], bool, u64) -> Option<ExecOut>;

pub fn default_exec(file: &str, args: &[String], verbatim: bool, timeout_ms: u64) -> Option<ExecOut> {
    let spec = Spec { file: file.into(), args: args.to_vec(), cwd: std::env::temp_dir().to_string_lossy().to_string(), verbatim };
    let opts = Opts { input: None, timeout_ms, max_stdout: 2 * 1024 * 1024, merged_cap: 64 * 1024, cancel: None };
    let out = run(&spec, &opts).ok()?;
    let timed_out = out.end == End::Timeout;
    let (stdout, stderr) = (String::from_utf8_lossy(&out.stdout).to_string(), String::from_utf8_lossy(&out.stderr).to_string());
    Some(ExecOut { stdout, stderr, timed_out })
}

const DOCTOR_TIMEOUT: u64 = 180_000;
const TOOL_TIMEOUT: u64 = 20_000;

fn re(cell: &'static OnceLock<Regex>, src: &str) -> &'static Regex {
    cell.get_or_init(|| Regex::new(src).expect("regex"))
}

fn doctor_status(t: &str) -> &'static str {
    match t {
        "installed" => "ok",
        "partial" => "warning",
        "notAvailable" | "missing" => "error",
        _ => "warning",
    }
}

fn message_status(t: &str) -> &'static str {
    match t {
        "hint" => "warning",
        "error" => "error",
        _ => "ok",
    }
}

fn str_of(v: Option<&Value>) -> String {
    v.and_then(Value::as_str).unwrap_or("").to_string()
}

fn json_array(stdout: &str) -> Option<Vec<Value>> {
    let start = stdout.find('[')?;
    let end = stdout.rfind(']')?;
    if end <= start {
        return None;
    }
    match serde_json::from_str::<Value>(&stdout[start..=end]).ok()? {
        Value::Array(a) => Some(a),
        _ => None,
    }
}

pub fn parse_doctor_machine(stdout: &str) -> Option<Vec<Value>> {
    let list = json_array(stdout)?;
    let mut checks = Vec::new();
    for raw in list {
        let Some(v) = raw.as_object() else { continue };
        let name = [v.get("name"), v.get("title"), v.get("validatorName")].into_iter().map(str_of).find(|s| !s.is_empty()).unwrap_or_default();
        if name.is_empty() {
            continue;
        }
        let messages: Vec<Value> = v
            .get("messages")
            .and_then(Value::as_array)
            .map(|a| {
                a.iter()
                    .filter_map(|m| {
                        let text = str_of(m.get("message"));
                        (!text.is_empty()).then(|| json!({"status": message_status(&str_of(m.get("type"))), "text": text}))
                    })
                    .collect()
            })
            .unwrap_or_default();
        let kind = Some(str_of(v.get("type"))).filter(|s| !s.is_empty()).unwrap_or_else(|| str_of(v.get("status")));
        checks.push(json!({"name": name, "status": doctor_status(&kind), "summary": str_of(v.get("statusInfo")), "messages": messages}));
    }
    (!checks.is_empty()).then_some(checks)
}

fn mark_status(m: &str) -> &'static str {
    match m {
        "✓" | "√" => "ok",
        "✗" | "X" | "x" | "☠" => "error",
        _ => "warning",
    }
}

fn bullet_status(m: &str) -> &'static str {
    match m {
        "!" => "warning",
        "✗" | "X" | "☠" => "error",
        _ => "ok",
    }
}

pub fn parse_doctor_text(stdout: &str) -> Vec<Value> {
    static HEAD: OnceLock<Regex> = OnceLock::new();
    static PAREN: OnceLock<Regex> = OnceLock::new();
    static DASH: OnceLock<Regex> = OnceLock::new();
    static ITEM: OnceLock<Regex> = OnceLock::new();
    let mut checks: Vec<Value> = Vec::new();
    for raw in stdout.split('\n') {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        if let Some(h) = re(&HEAD, r"^\[(.)\]\s+(.+)$").captures(line) {
            let title = h[2].trim();
            let (name, summary) = if let Some(p) = re(&PAREN, r"^(.*?)\s+\((.*)\)\s*$").captures(title) {
                (p[1].to_string(), p[2].to_string())
            } else if let Some(d) = re(&DASH, r"^(.*?)\s+-\s+(.*)$").captures(title) {
                (d[1].to_string(), d[2].to_string())
            } else {
                (title.to_string(), String::new())
            };
            checks.push(json!({"name": name, "status": mark_status(&h[1]), "summary": summary, "messages": []}));
            continue;
        }
        if let Some(i) = re(&ITEM, r"^\s+([•✓√!✗X☠])\s+(.+)$").captures(line) {
            if let Some(last) = checks.last_mut() {
                last["messages"].as_array_mut().unwrap().push(json!({"status": bullet_status(&i[1]), "text": i[2].trim()}));
            }
        }
    }
    checks
}

fn report(kind: &str, checks: Vec<Value>, error: Option<Value>) -> Value {
    json!({"kind": kind, "checks": checks, "error": error, "at": crate::util::now_ms()})
}

fn masked(checks: Vec<Value>, mask: &dyn Fn(&str) -> String) -> Vec<Value> {
    checks
        .into_iter()
        .map(|mut c| {
            for k in ["name", "summary"] {
                c[k] = json!(mask(c[k].as_str().unwrap_or("")));
            }
            for m in c["messages"].as_array_mut().into_iter().flatten() {
                m["text"] = json!(mask(m["text"].as_str().unwrap_or("")));
                if let Some(d) = m.get("issue").and_then(|i| i.get("detail")).and_then(Value::as_str).map(String::from) {
                    m["issue"]["detail"] = json!(mask(&d));
                }
            }
            c
        })
        .collect()
}

pub fn flutter_toolchain(sdk: Option<&str>, win: bool, env: &super::types::Env, exec: ToolchainExec, mask: &dyn Fn(&str) -> String) -> Value {
    let Some(sdk) = sdk else { return report("flutter", vec![], Some(json!({"code": "flutter-missing"}))) };
    for args in [["doctor", "--machine"], ["doctor", "-v"]] {
        let a: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        let Some(spec) = super::flutter::env::flutter_spawn(sdk, &a, win, env) else {
            return report("flutter", vec![], Some(json!({"code": "bad-arg"})));
        };
        let Some(res) = exec(&spec.file, &spec.args, spec.verbatim, DOCTOR_TIMEOUT) else {
            return report("flutter", vec![], Some(json!({"code": "doctor-failed"})));
        };
        if res.timed_out {
            return report("flutter", vec![], Some(json!({"code": "doctor-failed", "detail": "timeout"})));
        }
        let checks = if args[1] == "--machine" { parse_doctor_machine(&res.stdout) } else { Some(parse_doctor_text(&res.stdout)) };
        if let Some(c) = checks.filter(|c| !c.is_empty()) {
            return report("flutter", masked(c, mask), None);
        }
    }
    report("flutter", vec![], Some(json!({"code": "doctor-failed"})))
}

fn first_line(text: &str) -> String {
    text.split('\n').map(|l| l.trim()).find(|l| !l.is_empty()).unwrap_or("").to_string()
}

fn issues_of(env: &Value, prefix: &str) -> Vec<Value> {
    env["issues"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|i| i["code"].as_str().is_some_and(|c| c.starts_with(prefix)))
        .map(|i| {
            let code = i["code"].as_str().unwrap_or("");
            json!({"status": if code.ends_with("-unverified") { "warning" } else { "error" }, "text": code, "issue": i})
        })
        .collect()
}

pub fn android_toolchain(env: &Value, win: bool, exec: ToolchainExec, mask: &dyn Fn(&str) -> String) -> Value {
    let exe = |n: &str| if win { format!("{n}.exe") } else { n.to_string() };
    let sep = if win { "\\" } else { "/" };
    let mut checks: Vec<Value> = Vec::new();
    let jdk = env["jdk"].as_str();
    let java_out = jdk.and_then(|j| exec(&format!("{j}{sep}bin{sep}{}", exe("java")), &["-version".to_string()], false, TOOL_TIMEOUT));
    let jdk_issues = issues_of(env, "jdk-");
    let status = if jdk.is_none() || jdk_issues.iter().any(|i| i["status"] == "error") {
        "error"
    } else if !jdk_issues.is_empty() {
        "warning"
    } else {
        "ok"
    };
    let summary = match (&java_out, env["jdkVersion"].as_f64()) {
        (Some(o), _) => first_line(&format!("{}\n{}", o.stderr, o.stdout)),
        (None, Some(v)) => format!("Java {}", crate::js::number(v)),
        _ => String::new(),
    };
    let mut messages: Vec<Value> = Vec::new();
    if let Some(j) = jdk {
        messages.push(json!({"status": "ok", "text": j}));
    }
    if let Some(n) = env["jdkNote"].as_str() {
        messages.push(json!({"status": "ok", "text": n}));
    }
    messages.extend(jdk_issues);
    checks.push(json!({"name": "jdk", "status": status, "summary": summary, "messages": messages}));
    checks.push(json!({"name": "sdk", "status": if env["sdk"].is_string() { "ok" } else { "error" }, "summary": env["sdk"].as_str().unwrap_or(""), "messages": issues_of(env, "sdk-")}));
    let adb = env["adb"].as_str();
    let adb_out = adb.and_then(|a| exec(a, &["version".to_string()], false, TOOL_TIMEOUT));
    checks.push(json!({
        "name": "adb", "status": if adb.is_some() && adb_out.is_some() { "ok" } else { "error" },
        "summary": adb_out.as_ref().map(|o| first_line(&o.stdout)).unwrap_or_default(),
        "messages": adb.map(|a| vec![json!({"status": "ok", "text": a})]).unwrap_or_default(),
    }));
    checks.push(json!({"name": "emulator", "status": if env["emulator"].is_string() { "ok" } else { "warning" }, "summary": env["emulator"].as_str().unwrap_or(""), "messages": []}));
    let gw = issues_of(env, "gradlew-");
    checks.push(json!({"name": "gradlew", "status": if env["gradlew"] == true && gw.is_empty() { "ok" } else { "error" }, "summary": "", "messages": gw}));
    report("android", masked(checks, mask), None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_doctor_outputs() {
        let m = parse_doctor_machine("noise [{\"name\":\"Flutter\",\"type\":\"installed\",\"statusInfo\":\"3.x\",\"messages\":[{\"message\":\"hi\",\"type\":\"hint\"}]}] tail").unwrap();
        assert_eq!(m[0]["status"], "ok");
        assert_eq!(m[0]["messages"][0]["status"], "warning");
        assert!(parse_doctor_machine("nothing").is_none());
        let t = parse_doctor_text("[✓] Flutter (Channel stable)\n    • Framework x\n[!] Android toolchain - bad\n    ✗ missing\n");
        assert_eq!(t.len(), 2);
        assert_eq!((t[0]["name"].as_str(), t[0]["summary"].as_str()), (Some("Flutter"), Some("Channel stable")));
        assert_eq!(t[1]["status"], "warning");
        assert_eq!(t[1]["messages"][0]["status"], "error");
    }

    #[test]
    fn android_report_uses_exec() {
        let env = json!({"kind": "android", "sdk": "/sdk", "adb": "/sdk/adb", "emulator": null, "jdk": "/jdk", "jdkVersion": 17, "gradlew": true, "issues": [{"code": "jdk-old", "detail": "x"}]});
        let exec = |file: &str, _: &[String], _: bool, _: u64| Some(ExecOut { stdout: format!("out {file}\n"), stderr: "ver 17\n".into(), timed_out: false });
        let r = android_toolchain(&env, false, &exec, &|t| t.to_string());
        assert_eq!(r["checks"][0]["status"], "error");
        assert_eq!(r["checks"][0]["summary"], "ver 17");
        assert_eq!(r["checks"][2]["summary"], "out /sdk/adb");
        assert_eq!(r["checks"][3]["status"], "warning");
        assert_eq!(r["checks"][4]["status"], "ok");
    }
}
