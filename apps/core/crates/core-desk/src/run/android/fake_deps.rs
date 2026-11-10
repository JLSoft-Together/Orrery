use super::deps::EnvDeps;
use std::collections::BTreeMap;
use std::sync::Mutex;

#[derive(Default)]
pub struct FakeDeps {
    pub files: Mutex<BTreeMap<String, String>>,
    pub modes: BTreeMap<String, u32>,
    pub java: BTreeMap<String, String>,
    pub real: BTreeMap<String, String>,
    pub calls: Mutex<Vec<String>>,
    pub list_sep: Option<char>,
    pub key: Option<u64>,
}

impl FakeDeps {
    pub fn new(files: &[(&str, &str)]) -> Self {
        FakeDeps { files: Mutex::new(files.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()), ..Default::default() }
    }

    pub fn java(mut self, file: &str, v: &str) -> Self {
        self.java.insert(file.into(), v.into());
        self
    }

    pub fn listing(mut self, sep: char) -> Self {
        self.list_sep = Some(sep);
        self
    }

    pub fn add(&self, file: &str, text: &str) {
        self.files.lock().unwrap().insert(file.into(), text.into());
    }

    pub fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }
}

impl EnvDeps for FakeDeps {
    fn is_file(&self, file: &str) -> bool {
        self.files.lock().unwrap().contains_key(file)
    }

    fn read_text(&self, file: &str) -> Option<String> {
        self.files.lock().unwrap().get(file).cloned()
    }

    fn mode(&self, file: &str) -> Option<u32> {
        self.is_file(file).then(|| self.modes.get(file).copied().unwrap_or(0o755))
    }

    fn exec(&self, file: &str, args: &[&str], _timeout_ms: u64) -> Option<(String, String)> {
        let mut line = vec![file.to_string()];
        line.extend(args.iter().map(|a| a.to_string()));
        self.calls.lock().unwrap().push(line.join(" "));
        self.java.get(file).map(|v| (String::new(), format!("openjdk version \"{v}\" 2024-01-16")))
    }

    fn realpath(&self, file: &str) -> String {
        self.real.get(file).cloned().unwrap_or_else(|| file.to_string())
    }

    fn list_dirs(&self, dir: &str) -> Option<Vec<String>> {
        let sep = self.list_sep?;
        let prefix = format!("{dir}{sep}");
        let mut out: Vec<String> = Vec::new();
        for f in self.files.lock().unwrap().keys() {
            if let Some(rest) = f.strip_prefix(&prefix) {
                let name = rest.split(sep).next().unwrap_or("").to_string();
                if !out.contains(&name) {
                    out.push(name);
                }
            }
        }
        Some(out)
    }

    fn cache_key(&self) -> Option<u64> {
        self.key
    }
}
