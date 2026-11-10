use super::rules::{classify_entry, group_rule, CleanRequest, Rule, CLEAN_MIN_AGE_MS, DAY};
use super::service::{fail, fail_with, procs_value, AgentDataService, AgentHost, Out};
use crate::fs_tree::{read_names, walk_tree, Cancelled, EntryType, Signal};
use crate::js;
use crate::util::{fmt_bytes, is_inside_or_same, mtime_ms, path_str};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;

impl AgentDataService {
    pub fn clean(&self, req: &CleanRequest, host: &dyn AgentHost) -> Out {
        self.run("clean", fail("busy"), |signal| Ok(self.do_clean(req, signal, host).unwrap_or_else(|_| fail("cancelled"))))
    }

    fn do_clean(&self, req: &CleanRequest, signal: &Signal, host: &dyn AgentHost) -> Result<Value, Cancelled> {
        let roots = host.roots();
        let root = roots.of(req.agent).to_string();
        let rules: Vec<&'static Rule> = req.groups.iter().filter_map(|id| group_rule(req.agent, id)).collect();
        if rules.iter().any(|r| r.idle_only) {
            let procs = host.procs(req.agent);
            if !procs.is_empty() {
                return Ok(fail_with("running", &[("procs", procs_value(&procs))]));
            }
        }
        let now = host.now();
        let (plan, total) = clean_plan(Path::new(&root), req, now, Some(signal), cfg!(windows))?;
        if plan.is_empty() {
            return Ok(fail("nothing"));
        }
        let spec = json!({"kind": "clean", "agent": req.agent.as_str(), "groups": req.groups, "files": plan.len(), "size": fmt_bytes(total), "days": req.older_days});
        if host.confirm(spec).unwrap_or(1) != 0 {
            return Ok(fail("cancelled"));
        }
        let min_age = (req.older_days * DAY).max(CLEAN_MIN_AGE_MS);
        let (mut files, mut bytes, mut skipped) = (0.0, 0.0, 0.0);
        let mut dirs: HashSet<PathBuf> = HashSet::new();
        for (p, size) in &plan {
            if signal.load(Ordering::SeqCst) {
                break;
            }
            let fresh = fs::symlink_metadata(p).ok().filter(|m| m.is_file()).is_none_or(|m| host.now() - mtime_ms(&m) < min_age);
            if fresh {
                skipped += 1.0;
                continue;
            }
            if fs::remove_file(p).is_ok() {
                files += 1.0;
                bytes += size;
                if let Some(d) = p.parent() {
                    dirs.insert(d.to_path_buf());
                }
            } else {
                skipped += 1.0;
            }
            self.progress(host, json!({"op": "clean", "agent": req.agent.as_str(), "files": files, "bytes": bytes, "totalBytes": total, "phase": "delete"}), false);
        }
        let tops: HashSet<PathBuf> = rules.iter().flat_map(|r| r.names.iter()).map(|n| Path::new(&root).join(n)).collect();
        let mut dirs: Vec<PathBuf> = dirs.into_iter().collect();
        dirs.sort_by_key(|d| std::cmp::Reverse(js::len(&path_str(d))));
        let root_path = PathBuf::from(&root);
        for d in dirs {
            let mut dir = d;
            while is_inside_or_same(&root, &path_str(&dir)) && dir != root_path && !tops.contains(&dir) {
                if fs::remove_dir(&dir).is_err() {
                    break;
                }
                match dir.parent() {
                    Some(up) => dir = up.to_path_buf(),
                    None => break,
                }
            }
        }
        Ok(json!({"ok": true, "files": files, "bytes": bytes, "skipped": skipped}))
    }
}

pub fn clean_plan(root: &Path, req: &CleanRequest, now: f64, signal: Option<&Signal>, case_insensitive: bool) -> Result<(Vec<(PathBuf, f64)>, f64), Cancelled> {
    let cutoff = now - (req.older_days * DAY).max(CLEAN_MIN_AGE_MS);
    let mut files = Vec::new();
    let mut bytes = 0.0;
    for name in read_names(root) {
        let rule = classify_entry(req.agent, &name, case_insensitive);
        if !rule.cleanable || !req.groups.contains(&rule.id) {
            continue;
        }
        let p = root.join(&name);
        let Ok(meta) = fs::symlink_metadata(&p) else { continue };
        if meta.file_type().is_symlink() {
            continue;
        }
        if meta.is_file() {
            if mtime_ms(&meta) < cutoff {
                bytes += meta.len() as f64;
                files.push((p, meta.len() as f64));
            }
            continue;
        }
        if !meta.is_dir() {
            continue;
        }
        walk_tree(
            &p,
            |e| {
                if e.kind == EntryType::File && e.mtime_ms < cutoff {
                    files.push((p.join(&e.rel), e.size));
                    bytes += e.size;
                }
                Ok(())
            },
            signal,
        )?;
    }
    Ok((files, bytes))
}
