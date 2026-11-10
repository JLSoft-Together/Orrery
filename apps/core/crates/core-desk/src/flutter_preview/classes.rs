use super::mask::{line_at, sub};
use super::scan::{has_required_params, header, is_private, re, top_scopes, PreviewHit};
use regex::Regex;
use std::collections::HashSet;
use std::sync::OnceLock;

pub const WIDGET_BASES: &[&str] = &[
    "Widget", "StatelessWidget", "StatefulWidget", "InheritedWidget", "InheritedNotifier", "InheritedModel", "InheritedTheme", "ProxyWidget", "ParentDataWidget",
    "RenderObjectWidget", "LeafRenderObjectWidget", "SingleChildRenderObjectWidget", "MultiChildRenderObjectWidget", "ImplicitlyAnimatedWidget", "AnimatedWidget",
    "ConsumerWidget", "ConsumerStatefulWidget", "HookWidget", "StatefulHookWidget", "HookConsumerWidget", "StatefulHookConsumerWidget",
];

#[derive(Debug, Clone, PartialEq)]
pub struct ClassCtor {
    pub name: String,
    pub required: bool,
}

#[derive(Debug, Clone)]
pub struct ClassDecl {
    pub name: String,
    pub line: u32,
    pub super_name: Option<String>,
    pub abstract_: bool,
    pub ctors: Vec<ClassCtor>,
}

fn member_starts(masked: &str, open: usize, close: usize) -> Vec<usize> {
    let mut starts = vec![open + 1];
    let mut depth = 0i32;
    let b = masked.as_bytes();
    let mut k = open + 1;
    while k + 1 < close {
        match b[k] {
            b'(' | b'[' | b'{' => depth += 1,
            b')' | b']' | b'}' => {
                depth -= 1;
                if depth == 0 && b[k] == b'}' {
                    starts.push(k + 1);
                }
            }
            b';' if depth == 0 => starts.push(k + 1),
            _ => {}
        }
        k += 1;
    }
    starts
}

pub fn scan_dart_classes(src: &str, masked: &str) -> Vec<ClassDecl> {
    static HEAD: OnceLock<Regex> = OnceLock::new();
    static EXTENDS: OnceLock<Regex> = OnceLock::new();
    let head_re = re(&HEAD, r"\b((?:(?:abstract|sealed|base|final|interface|mixin)\s+)*)class\s+([A-Za-z_$][A-Za-z0-9_$]*)");
    let ext_re = re(&EXTENDS, r"^\s*extends\s+(?:[A-Za-z_][A-Za-z0-9_]*\.)?([A-Za-z_$][A-Za-z0-9_$]*)");
    let mut out = Vec::new();
    for scope in top_scopes(masked) {
        let head = sub(masked, scope.head, scope.open);
        let Some(m) = head_re.captures(head) else { continue };
        let whole = m.get(0).unwrap();
        let hb = head.as_bytes();
        let mut k = whole.end();
        while k < hb.len() && hb[k].is_ascii_whitespace() {
            k += 1;
        }
        if hb.get(k) == Some(&b'<') {
            let mut angle = 0;
            while k < hb.len() {
                if hb[k] == b'<' {
                    angle += 1;
                } else if hb[k] == b'>' {
                    angle -= 1;
                    if angle == 0 {
                        k += 1;
                        break;
                    }
                }
                k += 1;
            }
        }
        let name = m[2].to_string();
        let body = sub(masked, 0, scope.close.saturating_sub(1));
        let mut ctors = Vec::new();
        for at in member_starts(masked, scope.open, scope.close) {
            if let Some(h) = header(body, at) {
                if h.name == name || h.name.starts_with(&format!("{name}.")) {
                    ctors.push(ClassCtor { name: h.name, required: has_required_params(&h.params) });
                }
            }
        }
        out.push(ClassDecl {
            line: line_at(src, scope.head + whole.start()),
            super_name: ext_re.captures(sub(head, k, head.len())).map(|e| e[1].to_string()),
            abstract_: m[1].split_whitespace().any(|w| w == "abstract" || w == "sealed"),
            name,
            ctors,
        });
    }
    out
}

pub fn widget_names<'a>(decls: impl IntoIterator<Item = &'a ClassDecl>) -> HashSet<String> {
    let mut widgets: HashSet<String> = WIDGET_BASES.iter().map(|s| s.to_string()).collect();
    let list: Vec<&ClassDecl> = decls.into_iter().collect();
    let mut grew = true;
    while grew {
        grew = false;
        for d in &list {
            if !widgets.contains(&d.name) && d.super_name.as_ref().is_some_and(|s| widgets.contains(s)) {
                widgets.insert(d.name.clone());
                grew = true;
            }
        }
    }
    widgets
}

pub fn is_web_only(src: &str) -> bool {
    static R: OnceLock<Regex> = OnceLock::new();
    re(&R, r#"(?m)^\s*(?:import|export)\s+['"](?:dart:(?:html|js|js_util|js_interop|js_interop_unsafe|ui_web|indexed_db|svg|web_audio|web_gl)|package:web/)"#).is_match(src)
}

pub struct FileFlags {
    pub part: bool,
    pub web: bool,
}

pub fn class_hit(decl: &ClassDecl, file: &FileFlags, previews: &[PreviewHit]) -> Option<PreviewHit> {
    let name = &decl.name;
    if decl.abstract_ {
        return None;
    }
    if previews.iter().any(|h| &h.symbol == name || h.symbol.starts_with(&format!("{name}."))) {
        return None;
    }
    let hit = |call: String, skipped: Option<&'static str>| PreviewHit {
        line: decl.line,
        symbol: name.clone(),
        label: name.clone(),
        call: if skipped.is_some() { String::new() } else { call },
        kind: Some("class"),
        skipped,
    };
    if file.part {
        return Some(hit(String::new(), Some("part")));
    }
    if is_private(name) {
        return Some(hit(String::new(), Some("private")));
    }
    if file.web {
        return Some(hit(String::new(), Some("web")));
    }
    if decl.ctors.is_empty() {
        return Some(hit(format!("{name}()"), None));
    }
    let public: Vec<&ClassCtor> = decl.ctors.iter().filter(|c| !is_private(&c.name)).collect();
    if public.is_empty() {
        return Some(hit(String::new(), Some("ctor")));
    }
    let pick = public.iter().find(|c| &c.name == name && !c.required).or_else(|| public.iter().find(|c| !c.required));
    Some(match pick {
        Some(c) => hit(format!("{}()", c.name), None),
        None => hit(String::new(), Some("params")),
    })
}
