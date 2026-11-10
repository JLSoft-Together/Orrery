use std::collections::HashSet;

use core_store::extra::SensitiveSettings;
use core_transcript::rules::{CmdHit, Touch};

use super::paths::{glob_matches, is_inside, is_root, norm_path, resolve_target, same, short_path, slice_from};
use super::settings::{rule_group, CUSTOM_PREFIX, OUTSIDE_RULE, SECRET_FILE_GLOBS};
use core_transcript::jsstr::units;

#[derive(Debug, Clone, PartialEq)]
pub struct SensitiveHit {
    pub rule: String,
    pub path: Option<String>,
    pub file: Option<String>,
}

pub struct ClassifyContext<'a> {
    pub project: Option<&'a str>,
    pub home: &'a str,
    pub win: bool,
    pub safe_dirs: &'a [String],
    pub settings: &'a SensitiveSettings,
    pub project_key: Option<&'a str>,
}

fn file_hits(file: &str, ctx: &ClassifyContext) -> Vec<SensitiveHit> {
    let f = norm_path(file, ctx.win);
    let name = f.rsplit('/').next().unwrap_or("").to_owned();
    let project = ctx.project.filter(|p| !p.is_empty()).map(|p| norm_path(p, ctx.win));
    let in_project = project.as_ref().map(|p| is_inside(&f, p, ctx.win));
    let rel = match (&project, in_project) {
        (Some(p), Some(true)) => {
            let rest = slice_from(&f, units(p));
            rest.strip_prefix('/').unwrap_or(&rest).to_owned()
        }
        _ => f.clone(),
    };
    let mut out: Vec<String> = Vec::new();
    for glob in SECRET_FILE_GLOBS {
        if glob_matches(glob, &name, true) {
            out.push(glob.to_owned());
        }
    }
    for glob in &ctx.settings.globs {
        let target = if glob.contains('/') { &rel } else { &name };
        if glob_matches(glob, target, ctx.win) {
            out.push(format!("{CUSTOM_PREFIX}{glob}"));
        }
    }
    let safe = ctx.safe_dirs.iter().any(|d| is_inside(&f, &norm_path(d, ctx.win), ctx.win));
    if in_project == Some(false) && !safe {
        out.push(OUTSIDE_RULE.to_owned());
    }
    let path = short_path(&f, project.as_deref(), ctx.home, ctx.win);
    out.into_iter().map(|rule| SensitiveHit { rule, path: Some(path.clone()), file: Some(f.clone()) }).collect()
}

fn dir_of(abs: &str) -> String {
    if let Some(i) = abs.rfind('/') {
        let seg = &abs[i + 1..];
        if seg.contains(['*', '?']) {
            let dir = &abs[..i];
            return if dir.is_empty() { "/".to_owned() } else { dir.to_owned() };
        }
    }
    if abs.is_empty() {
        "/".to_owned()
    } else {
        abs.to_owned()
    }
}

fn cmd_hits(hit: &CmdHit, ctx: &ClassifyContext) -> Vec<SensitiveHit> {
    let Some(targets) = &hit.targets else {
        return vec![SensitiveHit { rule: hit.rule.to_owned(), path: None, file: None }];
    };
    let project = ctx.project.filter(|p| !p.is_empty()).map(|p| norm_path(p, ctx.win));
    let home = if ctx.home.is_empty() { String::new() } else { norm_path(ctx.home, ctx.win) };
    for target in targets {
        let Some(abs) = resolve_target(target, project.as_deref(), ctx.home, ctx.win) else { continue };
        let dir = dir_of(&abs);
        let danger = is_root(&dir)
            || (!home.is_empty() && (same(&dir, &home, ctx.win) || is_inside(&home, &dir, ctx.win)))
            || project.as_ref().is_some_and(|p| !is_inside(&abs, p, ctx.win) && !ctx.safe_dirs.iter().any(|d| is_inside(&abs, &norm_path(d, ctx.win), ctx.win)));
        if danger {
            return vec![SensitiveHit { rule: hit.rule.to_owned(), path: Some(short_path(&abs, project.as_deref(), ctx.home, ctx.win)), file: None }];
        }
    }
    Vec::new()
}

pub fn classify_touch(touch: &Touch, ctx: &ClassifyContext) -> Vec<SensitiveHit> {
    let raw: Vec<SensitiveHit> = match touch {
        Touch::File(f) if !f.is_empty() => file_hits(f, ctx),
        Touch::File(_) => Vec::new(),
        Touch::Cmds(cmds) => cmds.iter().flat_map(|h| cmd_hits(h, ctx)).collect(),
    };
    let ignored: HashSet<&str> = ctx.project_key.filter(|k| !k.is_empty()).and_then(|k| ctx.settings.ignore.get(k)).map(|l| l.iter().map(String::as_str).collect()).unwrap_or_default();
    let mut seen: HashSet<String> = HashSet::new();
    raw.into_iter()
        .filter(|h| {
            let Some(group) = rule_group(&h.rule) else { return false };
            if !ctx.settings.groups.get(group).copied().unwrap_or(false) || ignored.contains(h.rule.as_str()) || seen.contains(&h.rule) {
                return false;
            }
            seen.insert(h.rule.clone());
            true
        })
        .collect()
}
