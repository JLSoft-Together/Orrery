use crate::run::android::detect::strip_comments;
use crate::run::android::jsre::dynamic;
use crate::run::android::pathx;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Default)]
pub struct Module {
    pub path: String,
    pub dir: String,
    pub kind: String,
    pub manifest: Option<String>,
    pub guessed: bool,
}

impl Module {
    pub fn is_app(&self) -> bool {
        self.kind == "application" || self.guessed
    }
}

#[derive(Debug, Clone, Default)]
pub struct Project {
    pub dir: String,
    pub name: String,
    pub modules: Vec<Module>,
}

impl Project {
    pub fn from_value(v: &Value) -> Project {
        let text = |x: &Value| x.as_str().unwrap_or("").to_string();
        let modules = v["modules"]
            .as_array()
            .map(|a| {
                a.iter()
                    .map(|m| Module { path: text(&m["path"]), dir: text(&m["dir"]), kind: text(&m["type"]), manifest: m["manifest"].as_str().map(String::from), guessed: m["guessed"] == Value::Bool(true) })
                    .collect()
            })
            .unwrap_or_default();
        Project { dir: text(&v["dir"]), name: text(&v["name"]), modules }
    }
}

fn camel(s: &str) -> String {
    let cs: Vec<char> = s.chars().collect();
    let mut out = String::new();
    let mut i = 0;
    while i < cs.len() {
        if cs[i] == '-' || cs[i] == '_' {
            let mut j = i;
            while j < cs.len() && (cs[j] == '-' || cs[j] == '_') {
                j += 1;
            }
            if j < cs.len() && cs[j].is_ascii_alphanumeric() {
                out.extend(cs[j].to_uppercase());
                i = j + 1;
            } else {
                out.extend(&cs[i..j]);
                i = j;
            }
            continue;
        }
        out.push(cs[i]);
        i += 1;
    }
    out
}

fn accessor_of(module_path: &str) -> String {
    let parts: Vec<String> = module_path.trim_start_matches(':').split(':').map(camel).collect();
    format!("projects.{}", parts.join("."))
}

fn esc(s: &str) -> String {
    let mut o = String::new();
    for c in s.chars() {
        if ".*+?^${}()|[]\\".contains(c) {
            o.push('\\');
        }
        o.push(c);
    }
    o
}

pub fn dep_matcher(lib_path: &str) -> impl Fn(&str) -> bool {
    let by_path = dynamic(&format!(r#"project\s*\(\s*(?:path\s*[:=]\s*)?["']{}["']"#, esc(lib_path)));
    let by_accessor = dynamic(&format!(r"\b{}\b(?!\.)", esc(&accessor_of(lib_path))));
    move |text| by_path.as_ref().is_some_and(|r| r.find(text).is_some()) || by_accessor.as_ref().is_some_and(|r| r.find(text).is_some())
}

pub fn depends_on(build_script: &str, lib_path: &str) -> bool {
    dep_matcher(lib_path)(&strip_comments(build_script))
}

pub fn read_build_script(dir: &str) -> Option<String> {
    ["build.gradle.kts", "build.gradle"].iter().find_map(|name| std::fs::read(pathx::join(pathx::host_win(), &[dir, name])).ok().map(|b| String::from_utf8_lossy(&b).into_owned()))
}

pub type Graph = BTreeMap<String, Vec<String>>;
type Matcher = Box<dyn Fn(&str) -> bool>;

pub fn script_graph(project: &Project) -> Graph {
    let matchers: Vec<(String, Matcher)> = project.modules.iter().map(|m| (m.path.clone(), Box::new(dep_matcher(&m.path)) as Matcher)).collect();
    let mut graph = Graph::new();
    for m in &project.modules {
        let stripped = read_build_script(&m.dir).map(|t| strip_comments(&t)).unwrap_or_default();
        let deps = if stripped.is_empty() { Vec::new() } else { matchers.iter().filter(|(p, test)| *p != m.path && test(&stripped)).map(|(p, _)| p.clone()).collect() };
        graph.insert(m.path.clone(), deps);
    }
    graph
}

pub fn closure(graph: &Graph, from: &str) -> Vec<String> {
    let mut queue = vec![from.to_string()];
    let mut i = 0;
    while i < queue.len() {
        for next in graph.get(&queue[i]).map(Vec::as_slice).unwrap_or(&[]) {
            if !queue.contains(next) {
                queue.push(next.clone());
            }
        }
        i += 1;
    }
    queue
}

pub fn reaches(graph: &Graph, from: &str, to: &str) -> bool {
    from != to && closure(graph, from).iter().any(|m| m == to)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_path_and_type_safe_accessors_only() {
        assert!(depends_on("implementation(project(\":core:ui-kit\"))", ":core:ui-kit"));
        assert!(depends_on("api project(path: ':core:ui-kit')", ":core:ui-kit"));
        assert!(depends_on("implementation(projects.core.uiKit)", ":core:ui-kit"));
        assert!(!depends_on("implementation(projects.core.uiKit.extra)", ":core:ui-kit"));
        assert!(!depends_on("// implementation(project(\":lib\"))", ":lib"));
        assert!(!depends_on("implementation(project(\":library\"))", ":lib"));
    }

    #[test]
    fn closure_follows_edges_breadth_first() {
        let mut g = Graph::new();
        g.insert(":app".into(), vec![":a".into(), ":b".into()]);
        g.insert(":a".into(), vec![":c".into()]);
        g.insert(":b".into(), vec![":c".into()]);
        assert_eq!(closure(&g, ":app"), vec![":app", ":a", ":b", ":c"]);
        assert!(reaches(&g, ":app", ":c") && !reaches(&g, ":a", ":b") && !reaches(&g, ":a", ":a"));
    }
}
