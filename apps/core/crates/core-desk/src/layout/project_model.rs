use crate::run::android::jsre::Js;
use crate::run::android::pathx;
use serde_json::{json, Map, Value};
use std::collections::HashSet;

pub static PACKAGE_RE: Js = Js::new(r"^[A-Za-z_][\w]*(\.[A-Za-z_][\w]*)*$", "");
pub static THEME_RE: Js = Js::new(r"^@?(android:)?(style\/)?[A-Za-z_][\w.]*$", "");
static MODULE_RE: Js = Js::new(r"^(:[A-Za-z0-9_.-]+)+$", "");
static VARIANT_RE: Js = Js::new(r"^[A-Za-z][A-Za-z0-9]*$", "");

pub const CAP_RES_DIRS: usize = 64;
pub const CAP_AAR_RES_DIRS: usize = 1000;
pub const CAP_CLASSPATH: usize = 4000;
pub const CAP_ACTIVITY_THEMES: usize = 500;
pub const CAP_PROJECT_DEPS: usize = 500;
const KIT_THEME: &str = "@style/Theme.Material3.DayNight.NoActionBar";

pub type Exists<'a> = &'a dyn Fn(&str) -> bool;

#[derive(Debug, Clone, Default, PartialEq)]
pub struct Payload {
    pub res_dirs: Vec<String>,
    pub aar_res_dirs: Vec<(String, String)>,
    pub classpath: Vec<String>,
    pub r_jar: Option<String>,
    pub package_name: String,
    pub namespace: String,
    pub app_theme: Option<String>,
    pub activity_themes: Vec<(String, String)>,
    pub min_sdk: Option<u32>,
    pub target_sdk: Option<u32>,
    pub sample_data_dirs: Vec<String>,
}

impl Payload {
    pub fn to_value(&self) -> Value {
        let mut m = Map::new();
        m.insert("resDirs".into(), json!(self.res_dirs));
        m.insert("aarResDirs".into(), Value::Array(self.aar_res_dirs.iter().map(|(d, n)| json!({"dir": d, "name": n})).collect()));
        m.insert("classpath".into(), json!(self.classpath));
        m.insert("packageName".into(), json!(self.package_name));
        m.insert("namespace".into(), json!(self.namespace));
        let themes: Map<String, Value> = self.activity_themes.iter().map(|(a, t)| (a.clone(), json!(t))).collect();
        m.insert("activityThemes".into(), Value::Object(themes));
        if let Some(r) = &self.r_jar {
            m.insert("rJar".into(), json!(r));
        }
        if let Some(t) = &self.app_theme {
            m.insert("appTheme".into(), json!(t));
        }
        if let Some(v) = self.min_sdk {
            m.insert("minSdk".into(), json!(v));
        }
        if let Some(v) = self.target_sdk {
            m.insert("targetSdk".into(), json!(v));
        }
        if !self.sample_data_dirs.is_empty() {
            m.insert("sampleDataDirs".into(), json!(self.sample_data_dirs));
        }
        Value::Object(m)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parsed {
    pub module: String,
    pub variant: String,
    pub payload: Payload,
    pub project_deps: Vec<String>,
    pub warnings: Vec<String>,
}

fn str_of(v: &Value) -> String {
    v.as_str().unwrap_or("").to_string()
}

fn strings(v: &Value) -> Vec<String> {
    v.as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(String::from)).collect()).unwrap_or_default()
}

fn sdk(v: &Value) -> Option<u32> {
    let f = v.as_f64()?;
    (f.fract() == 0.0 && f > 0.0 && f < 1000.0).then_some(f as u32)
}

pub fn project_model_args(module: &str, out_file: &str, init_script: &str, variant: &str, compile: bool, win: bool) -> Result<Vec<String>, String> {
    let module = if module.starts_with(':') { module.to_string() } else { format!(":{module}") };
    if !MODULE_RE.test(&module) {
        return Err(format!("invalid Gradle module path: {module}"));
    }
    if !VARIANT_RE.test(variant) {
        return Err(format!("invalid variant: {variant}"));
    }
    if !pathx::is_absolute(out_file, win) || !pathx::is_absolute(init_script, win) {
        return Err("outFile and initScript must be absolute".into());
    }
    let mut args: Vec<String> = vec!["-q".into(), "--init-script".into(), init_script.into(), format!("{module}:ccmProjectModel"), format!("-Pccm.out={out_file}"), format!("-Pccm.variant={variant}")];
    if compile {
        args.push("-Pccm.compile=true".into());
    }
    Ok(args)
}

fn keep(label: &str, list: &[String], cap: usize, exists: Exists, win: bool, warnings: &mut Vec<String>) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();
    let mut missing = 0;
    for p in list {
        if !pathx::is_absolute(p, win) || p.contains('\0') {
            warnings.push(format!("{label}: ignored non-absolute path"));
            continue;
        }
        let key = pathx::normalize(p, win);
        if !seen.insert(key.clone()) {
            continue;
        }
        if !exists(&key) {
            missing += 1;
            continue;
        }
        out.push(key);
    }
    if missing > 0 {
        warnings.push(format!("{label}: {missing} missing on disk (build the project)"));
    }
    if out.len() > cap {
        warnings.push(format!("{label}: capped at {cap} of {}", out.len()));
    }
    out.truncate(cap);
    out
}

pub fn parse_project_model_text(text: &str, exists: Exists, win: bool) -> Result<Parsed, String> {
    let raw: Value = serde_json::from_str(text).map_err(|e| e.to_string())?;
    parse_project_model(&raw, exists, win)
}

pub fn parse_project_model(raw: &Value, exists: Exists, win: bool) -> Result<Parsed, String> {
    let empty = Value::Object(Map::new());
    let m = match raw {
        Value::Object(_) => raw,
        Value::Array(_) => &empty,
        _ => return Err("project model: not an object".into()),
    };
    let mut warnings: Vec<String> = strings(&m["warnings"]).into_iter().take(50).collect();
    let namespace = str_of(&m["namespace"]);
    if namespace.is_empty() || !PACKAGE_RE.test(&namespace) {
        return Err("project model: namespace missing".into());
    }
    let application_id = str_of(&m["applicationId"]);
    let package_name = if !application_id.is_empty() && PACKAGE_RE.test(&application_id) { application_id } else { namespace.clone() };
    let mut res_raw = strings(&m["resDirs"]);
    res_raw.extend(strings(&m["generatedResDirs"]));
    let res_dirs = keep("resDirs", &res_raw, CAP_RES_DIRS, exists, win, &mut warnings);
    let mut aar_by_dir: Vec<(String, String)> = Vec::new();
    for a in m["aarRes"].as_array().map(Vec::as_slice).unwrap_or(&[]) {
        let dir = str_of(&a["dir"]);
        if dir.is_empty() {
            continue;
        }
        let key = pathx::normalize(&dir, win);
        let name = str_of(&a["name"]);
        let name = if name.is_empty() { pathx::basename(&pathx::dirname(&dir, win), win) } else { name };
        match aar_by_dir.iter_mut().find(|(k, _)| *k == key) {
            Some(e) => e.1 = name,
            None => aar_by_dir.push((key, name)),
        }
    }
    let aar_keys: Vec<String> = aar_by_dir.iter().map(|(k, _)| k.clone()).collect();
    let aar_res_dirs: Vec<(String, String)> = keep("aarResDirs", &aar_keys, CAP_AAR_RES_DIRS, exists, win, &mut warnings)
        .into_iter()
        .map(|dir| {
            let name = aar_by_dir.iter().find(|(k, _)| *k == dir).map(|(_, n)| n.clone()).unwrap_or_else(|| pathx::basename(&pathx::dirname(&dir, win), win));
            (dir, name)
        })
        .collect();
    let mut class_raw = strings(&m["classDirs"]);
    class_raw.extend(strings(&m["classpath"]));
    let classpath = keep("classpath", &class_raw, CAP_CLASSPATH, exists, win, &mut warnings);
    let r_jar_raw = str_of(&m["rJar"]);
    let r_jar = if r_jar_raw.is_empty() { None } else { keep("rJar", &[r_jar_raw], 1, exists, win, &mut warnings).into_iter().next() };
    if r_jar.is_none() {
        warnings.push("R.jar missing: AndroidX / Material views need a successful build".into());
    }
    let mut activity_themes: Vec<(String, String)> = Vec::new();
    if let Some(o) = m["activityThemes"].as_object() {
        for (activity, theme) in o.iter().take(CAP_ACTIVITY_THEMES) {
            if let Some(t) = theme.as_str() {
                if PACKAGE_RE.test(activity) && THEME_RE.test(t) {
                    activity_themes.push((activity.clone(), t.to_string()));
                }
            }
        }
    }
    let app_theme = str_of(&m["appTheme"]);
    let mut payload = Payload { res_dirs, aar_res_dirs, classpath, r_jar, package_name, namespace, activity_themes, ..Default::default() };
    if !app_theme.is_empty() && THEME_RE.test(&app_theme) {
        payload.app_theme = Some(app_theme);
    }
    payload.min_sdk = sdk(&m["minSdk"]);
    payload.target_sdk = sdk(&m["targetSdk"]);
    let project_dir = str_of(&m["projectDir"]);
    if !project_dir.is_empty() && pathx::is_absolute(&project_dir, win) {
        let sample = pathx::join(win, &[&project_dir, "sampledata"]);
        if exists(&sample) {
            payload.sample_data_dirs = vec![sample];
        }
    }
    let mut project_deps: Vec<String> = Vec::new();
    for d in strings(&m["projectDeps"]) {
        if MODULE_RE.test(&d) && !project_deps.contains(&d) {
            project_deps.push(d);
        }
    }
    project_deps.truncate(CAP_PROJECT_DEPS);
    let module = str_of(&m["module"]);
    let variant = str_of(&m["variant"]);
    Ok(Parsed { module: if module.is_empty() { ":app".into() } else { module }, variant: if variant.is_empty() { "debug".into() } else { variant }, payload, project_deps, warnings })
}

pub fn parse_loose_kit(raw: &Value, dir: &str, exists: Exists, win: bool) -> Result<Payload, String> {
    if !raw.is_object() && !raw.is_array() {
        return Err("loose kit: not an object".into());
    }
    let inside = |rel: &Value| -> Option<String> {
        let rel = rel.as_str()?;
        if rel.is_empty() || rel.contains('\0') || pathx::is_absolute(rel, win) {
            return None;
        }
        let abs = pathx::resolve(dir, rel, win);
        let same = pathx::normalize(&abs, win) == pathx::normalize(dir, win);
        (!same && pathx::is_within(dir, &abs, win) && exists(&abs)).then_some(abs)
    };
    let mut aar_res_dirs = Vec::new();
    for a in raw["aarRes"].as_array().map(|a| a.iter().take(CAP_AAR_RES_DIRS)).into_iter().flatten() {
        if let Some(abs) = inside(&a["dir"]) {
            let name = str_of(&a["name"]);
            aar_res_dirs.push((abs.clone(), if name.is_empty() { pathx::basename(&pathx::dirname(&abs, win), win) } else { name }));
        }
    }
    let classpath: Vec<String> = raw["classpath"].as_array().map(|a| a.iter().take(CAP_CLASSPATH).filter_map(&inside).collect()).unwrap_or_default();
    let r_jar = inside(&raw["rJar"]);
    if classpath.is_empty() || r_jar.is_none() {
        return Err("loose kit: classes or R.jar missing".into());
    }
    let pkg = str_of(&raw["packageName"]);
    let package_name = if PACKAGE_RE.test(&pkg) { pkg } else { "dev.ccm.loosekit".to_string() };
    let theme = str_of(&raw["appTheme"]);
    Ok(Payload {
        res_dirs: Vec::new(),
        aar_res_dirs,
        classpath,
        r_jar,
        namespace: package_name.clone(),
        package_name,
        app_theme: Some(if THEME_RE.test(&theme) { theme } else { KIT_THEME.to_string() }),
        ..Default::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all(_: &str) -> bool {
        true
    }

    #[test]
    fn parses_caps_and_warnings() {
        let raw = json!({
            "namespace": "dev.t", "applicationId": "bad id", "resDirs": ["/r/a", "rel", "/r/a", "/r/missing"], "generatedResDirs": ["/r/g"],
            "aarRes": [{"dir": "/x/y/res", "name": "n"}, {"dir": "/x/z/res"}], "classDirs": ["/c"], "classpath": ["/c", "/j.jar"], "activityThemes": {"dev.t.A": "@style/T", "bad name": "@style/T", "dev.t.B": "no spaces"},
            "appTheme": "@style/App", "minSdk": 24, "targetSdk": 0, "projectDeps": [":a", "a", ":a"], "warnings": ["w"],
        });
        let miss = |p: &str| p != "/r/missing";
        let m = parse_project_model(&raw, &miss, false).unwrap();
        assert_eq!((m.module.as_str(), m.variant.as_str(), m.payload.package_name.as_str()), (":app", "debug", "dev.t"));
        assert_eq!(m.payload.res_dirs, vec!["/r/a", "/r/g"]);
        assert_eq!(m.payload.aar_res_dirs, vec![("/x/y/res".to_string(), "n".to_string()), ("/x/z/res".to_string(), "z".to_string())]);
        assert_eq!(m.payload.classpath, vec!["/c", "/j.jar"]);
        assert_eq!(m.payload.activity_themes, vec![("dev.t.A".to_string(), "@style/T".to_string())]);
        assert_eq!((m.payload.min_sdk, m.payload.target_sdk, m.payload.app_theme.as_deref(), m.project_deps), (Some(24), None, Some("@style/App"), vec![":a".to_string()]));
        assert_eq!(m.warnings, vec!["w", "resDirs: ignored non-absolute path", "resDirs: 1 missing on disk (build the project)", "R.jar missing: AndroidX / Material views need a successful build"]);
        assert_eq!(parse_project_model(&json!({"namespace": "9x"}), &all, false), Err("project model: namespace missing".into()));
        assert_eq!(parse_project_model(&json!(5), &all, false), Err("project model: not an object".into()));
    }

    #[test]
    fn payload_json_keeps_the_daemon_field_order() {
        let p = Payload { res_dirs: vec!["/a".into()], r_jar: Some("/R.jar".into()), package_name: "p".into(), namespace: "n".into(), min_sdk: Some(21), ..Default::default() };
        let keys: Vec<String> = p.to_value().as_object().unwrap().keys().cloned().collect();
        assert_eq!(keys, vec!["resDirs", "aarResDirs", "classpath", "packageName", "namespace", "activityThemes", "rJar", "minSdk"]);
    }

    #[test]
    fn gradle_args_are_validated() {
        let ok = project_model_args("app", "/o.json", "/i.gradle", "debug", true, false).unwrap();
        assert_eq!(ok, vec!["-q", "--init-script", "/i.gradle", ":app:ccmProjectModel", "-Pccm.out=/o.json", "-Pccm.variant=debug", "-Pccm.compile=true"]);
        assert!(project_model_args(":a b", "/o", "/i", "debug", false, false).is_err());
        assert!(project_model_args(":a", "/o", "/i", "1bad", false, false).is_err());
        assert!(project_model_args(":a", "o", "/i", "debug", false, false).is_err());
    }

    #[test]
    fn loose_kit_paths_stay_inside_the_kit() {
        let raw = json!({"classpath": ["a.jar", "../out.jar", "/abs.jar"], "rJar": "R.jar", "packageName": "9bad", "aarRes": [{"dir": "res/x"}, {"dir": "../res"}]});
        let k = parse_loose_kit(&raw, "/kit", &all, false).unwrap();
        assert_eq!((k.classpath, k.r_jar, k.package_name, k.app_theme), (vec!["/kit/a.jar".to_string()], Some("/kit/R.jar".to_string()), "dev.ccm.loosekit".to_string(), Some(KIT_THEME.to_string())));
        assert_eq!(k.aar_res_dirs, vec![("/kit/res/x".to_string(), "res".to_string())]);
        assert!(parse_loose_kit(&json!({"classpath": ["a.jar"]}), "/kit", &all, false).is_err());
    }
}
