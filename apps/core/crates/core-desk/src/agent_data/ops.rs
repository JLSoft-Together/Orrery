use super::moves::MoveRecord;
use super::rules::{moved_name, Agent};
use super::service::{dirname, exists, fail, fail_with, io_err, join, procs_value, AgentDataService, AgentHost, Out};
use super::tree::{aborted, copy_times, copy_tree, link_target, make_dir_link, verify_copy, Halt};
use crate::fs_tree::{read_names, remove_link, remove_tree, tree_size, walk_tree, EntryType, Signal};
use crate::util::{basename, fmt_bytes, is_inside_or_same, path_str};
use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

fn sorted_names(dir: &str) -> Vec<String> {
    let mut v = read_names(Path::new(dir));
    v.sort();
    v
}

fn same_parent(a: &str, b: &str) -> bool {
    is_inside_or_same(&path_str(&dirname(a)), &path_str(&dirname(b))) && is_inside_or_same(&path_str(&dirname(b)), &path_str(&dirname(a)))
}

fn backup_of_source(rec: &MoveRecord) -> bool {
    same_parent(&rec.source, &rec.backup) && basename(&rec.backup).starts_with(&format!("{}.orrery-moved-", basename(&rec.source)))
}

fn target_of_source(rec: &MoveRecord) -> bool {
    let (a, b) = (basename(&rec.target), basename(&rec.source));
    if cfg!(windows) {
        a.to_lowercase() == b.to_lowercase()
    } else {
        a == b
    }
}

impl AgentDataService {
    pub fn move_out(&self, agent: Agent, host: &dyn AgentHost) -> Out {
        self.run("move", fail("busy"), |signal| self.do_move(agent, signal, host))
    }

    fn do_move(&self, agent: Agent, signal: &Signal, host: &dyn AgentHost) -> Out {
        let source = host.roots().of(agent).to_string();
        if !exists(&source) {
            return Ok(fail("not-found"));
        }
        if link_target(Path::new(&source)).is_some() {
            return Ok(fail("already-moved"));
        }
        if self.store().get(agent).is_some() {
            return Ok(fail("invalid"));
        }
        let procs = host.procs(agent);
        if !procs.is_empty() {
            return Ok(fail_with("running", &[("procs", procs_value(&procs))]));
        }
        let Some(parent) = host.pick_folder("target") else { return Ok(fail("cancelled")) };
        let target = path_str(&join(&parent, &basename(&source)));
        if is_inside_or_same(&source, &parent) || is_inside_or_same(&parent, &source) {
            return Ok(fail("target-inside"));
        }
        if exists(&target) {
            return Ok(fail_with("target-exists", &[("path", json!(target))]));
        }
        if !host.is_local(&parent) {
            return Ok(fail("not-local"));
        }
        let Ok(size) = tree_size(Path::new(&source), Some(signal), |_| {}) else { return Ok(fail("cancelled")) };
        if host.free_bytes(&parent).is_some_and(|free| free < size.bytes * 1.02 + 256.0 * 1024.0 * 1024.0) {
            return Ok(fail_with("no-space", &[("bytes", json!(size.bytes))]));
        }
        let at = host.now();
        let backup = path_str(&dirname(&source).join(moved_name(&basename(&source), at)));
        let spec = json!({"kind": "move", "from": source, "to": target, "size": fmt_bytes(size.bytes), "original": basename(&backup)});
        if host.confirm(spec).unwrap_or(1) != 0 {
            return Ok(fail("cancelled"));
        }
        let rec = MoveRecord { agent, source: source.clone(), target: target.clone(), backup: backup.clone(), at, phase: "copying", original_deleted: false };
        self.store().set(rec.clone()).map_err(io_err)?;
        let discard = |error: &str, extra: &[(&str, Value)], remove: bool| -> Out {
            if remove {
                let _ = remove_tree(Path::new(&target), None, |_| {});
            }
            self.store().drop_agent(agent).map_err(io_err)?;
            Ok(fail_with(error, extra))
        };
        let mut copied = 0.0;
        let copy = copy_tree(Path::new(&source), Path::new(&target), Some(signal), |n| {
            copied += n;
            self.progress(host, json!({"op": "move", "agent": agent.as_str(), "files": 0, "bytes": copied, "totalBytes": size.bytes, "phase": "copy"}), false);
        });
        let copy = match copy {
            Ok(c) => c,
            Err(Halt::Cancelled) => return discard("cancelled", &[], true),
            Err(Halt::Failed) => return discard("failed", &[], false),
        };
        self.store().set(rec.with_phase("copied")).map_err(io_err)?;
        let stats = json!({"op": "move", "agent": agent.as_str(), "files": copy.files, "bytes": copy.bytes, "totalBytes": size.bytes});
        let with_phase = |phase: &str| {
            let mut v = stats.clone();
            v["phase"] = json!(phase);
            v
        };
        self.progress(host, with_phase("verify"), true);
        match verify_copy(Path::new(&source), Path::new(&target), &copy.skipped, Some(signal)) {
            Err(Halt::Cancelled) => return discard("cancelled", &[], true),
            Err(Halt::Failed) => return discard("verify-failed", &[], true),
            Ok(check) if !check.ok => return discard("verify-failed", &[("path", json!(check.mismatch))], true),
            Ok(_) => {}
        }
        let late = host.procs(agent);
        if !late.is_empty() {
            return discard("running", &[("procs", procs_value(&late))], true);
        }
        self.progress(host, with_phase("switch"), true);
        self.store().set(rec.with_phase("switching")).map_err(io_err)?;
        host.stop_collector();
        let switched = (|| -> Out {
            if fs::rename(&source, &backup).is_err() {
                return discard("locked", &[], true);
            }
            if make_dir_link(Path::new(&target), Path::new(&source)).is_err() {
                let _ = fs::rename(&backup, &source);
                return discard("failed", &[], true);
            }
            if sorted_names(&source) != sorted_names(&backup) {
                let _ = remove_link(Path::new(&source));
                let _ = fs::rename(&backup, &source);
                return discard("verify-failed", &[], true);
            }
            self.store().set(rec.with_phase("linked")).map_err(io_err)?;
            Ok(Value::Null)
        })();
        host.start_collector();
        match switched {
            Ok(Value::Null) => Ok(json!({"ok": true, "files": copy.files, "bytes": copy.bytes, "skipped": copy.skipped.len(), "path": target})),
            other => other,
        }
    }

    pub fn undo(&self, agent: Agent, host: &dyn AgentHost) -> Out {
        self.run("undo", fail("busy"), |signal| self.do_undo(agent, signal, host))
    }

    fn do_undo(&self, agent: Agent, signal: &Signal, host: &dyn AgentHost) -> Out {
        let Some(rec) = self.store().get(agent).filter(|r| r.phase == "linked") else { return Ok(fail("not-moved")) };
        if link_target(Path::new(&rec.source)).is_none() {
            return Ok(fail("not-moved"));
        }
        let procs = host.procs(agent);
        if !procs.is_empty() {
            return Ok(fail_with("running", &[("procs", procs_value(&procs))]));
        }
        let has_original = !rec.original_deleted && exists(&rec.backup);
        let mut newer: Vec<PathBuf> = Vec::new();
        let mut total = 0.0;
        let walked = walk_tree(
            Path::new(&rec.target),
            |e| {
                if e.kind != EntryType::File {
                    return Ok(());
                }
                total += e.size;
                if has_original && (e.mtime_ms > rec.at || !Path::new(&rec.backup).join(&e.rel).exists()) {
                    newer.push(e.rel.clone());
                }
                Ok(())
            },
            Some(signal),
        );
        if walked.is_err() {
            return Ok(fail("cancelled"));
        }
        if !has_original && host.free_bytes(&path_str(&dirname(&rec.source))).is_some_and(|free| free < total * 1.02 + 256.0 * 1024.0 * 1024.0) {
            return Ok(fail_with("no-space", &[("bytes", json!(total))]));
        }
        let offer = has_original && !newer.is_empty();
        let buttons: i64 = if offer { 3 } else { 2 };
        let spec = json!({"kind": "undo", "folder": rec.source, "target": rec.target, "newFiles": newer.len(), "originalDeleted": !has_original, "copyBack": offer});
        let answer = host.confirm(spec).unwrap_or(buttons - 1);
        if answer == buttons - 1 || !(0..buttons).contains(&answer) {
            return Ok(fail("cancelled"));
        }
        let copy_back = offer && answer == 0;
        self.store().set(rec.with_phase("undoing")).map_err(io_err)?;
        host.stop_collector();
        let (source, target, backup) = (Path::new(&rec.source), Path::new(&rec.target), Path::new(&rec.backup));
        let mut done = (0.0, 0.0);
        let result = (|| -> Result<(), ()> {
            remove_link(source).map_err(|_| ())?;
            if has_original {
                fs::rename(backup, source).map_err(|_| ())?;
                if copy_back {
                    for rel in &newer {
                        let (from, to) = (target.join(rel), source.join(rel));
                        let ok = to.parent().is_none_or(|d| fs::create_dir_all(d).is_ok()) && fs::copy(&from, &to).is_ok();
                        if ok {
                            if let Ok(st) = fs::metadata(&from) {
                                copy_times(&st, &to);
                            }
                            done.0 += 1.0;
                        } else {
                            done.1 += 1.0;
                        }
                    }
                }
            } else {
                let mut copied = 0.0;
                let copy = copy_tree(target, source, Some(signal), |n| {
                    copied += n;
                    self.progress(host, json!({"op": "undo", "agent": agent.as_str(), "files": 0, "bytes": copied, "totalBytes": total, "phase": "copy"}), false);
                })
                .map_err(|_| ())?;
                let check = verify_copy(target, source, &copy.skipped, Some(signal)).map_err(|_| ())?;
                if !check.ok {
                    return Err(());
                }
                done = (copy.files, copy.skipped.len() as f64);
            }
            self.store().drop_agent(agent).map_err(|_| ())
        })();
        if result.is_err() {
            if link_target(source).is_none() {
                if has_original && source.exists() {
                    let _ = fs::rename(source, backup);
                } else if !has_original {
                    let _ = remove_tree(source, None, |_| {});
                }
                let _ = make_dir_link(target, source);
            }
            let saved = self.store().set(rec.with_phase("linked")).map_err(io_err);
            host.start_collector();
            saved?;
            return Ok(fail(if aborted(Some(signal)) { "cancelled" } else { "failed" }));
        }
        host.start_collector();
        Ok(json!({"ok": true, "files": done.0, "skipped": done.1, "path": rec.target}))
    }

    pub fn delete_original(&self, agent: Agent, host: &dyn AgentHost) -> Out {
        self.run("delete", fail("busy"), |signal| self.do_delete(agent, signal, host))
    }

    fn do_delete(&self, agent: Agent, signal: &Signal, host: &dyn AgentHost) -> Out {
        let Some(rec) = self.store().get(agent).filter(|r| r.phase == "linked" && !r.original_deleted && exists(&r.backup)) else { return Ok(fail("not-moved")) };
        if is_inside_or_same(&rec.backup, &rec.target) || link_target(Path::new(&rec.backup)).is_some() || !backup_of_source(&rec) {
            return Ok(fail("invalid"));
        }
        let Ok(size) = tree_size(Path::new(&rec.backup), Some(signal), |_| {}) else { return Ok(fail("cancelled")) };
        let spec = json!({"kind": "delete", "folder": rec.backup, "size": fmt_bytes(size.bytes)});
        if host.confirm(spec).unwrap_or(1) != 0 {
            return Ok(fail("cancelled"));
        }
        let mut done = 0.0;
        let removed = remove_tree(Path::new(&rec.backup), Some(signal), |n| {
            done += n;
            self.progress(host, json!({"op": "delete", "agent": agent.as_str(), "files": 0, "bytes": done, "totalBytes": size.bytes, "phase": "delete"}), false);
        });
        let Ok(r) = removed else { return Ok(fail("cancelled")) };
        if !exists(&rec.backup) {
            self.store().set(MoveRecord { original_deleted: true, ..rec.clone() }).map_err(io_err)?;
        }
        if r.failed > 0.0 {
            return Ok(fail_with("failed", &[("files", json!(r.files)), ("bytes", json!(r.bytes)), ("skipped", json!(r.failed))]));
        }
        Ok(json!({"ok": true, "files": r.files, "bytes": r.bytes}))
    }

    pub fn discard(&self, agent: Agent, host: &dyn AgentHost) -> Out {
        self.run("delete", fail("busy"), |signal| {
            let Some(rec) = self.store().get(agent).filter(|r| r.phase == "interrupted") else { return Ok(fail("not-moved")) };
            let live = link_target(Path::new(&rec.source)).is_some() || !exists(&rec.source);
            if live || is_inside_or_same(&rec.target, &rec.source) || !target_of_source(&rec) {
                return Ok(fail("invalid"));
            }
            if exists(&rec.target) {
                if host.confirm(json!({"kind": "discard", "folder": rec.target})).unwrap_or(1) != 0 {
                    return Ok(fail("cancelled"));
                }
                if remove_tree(Path::new(&rec.target), Some(signal), |_| {}).is_err() {
                    return Ok(fail("cancelled"));
                }
            }
            self.store().drop_agent(agent).map_err(io_err)?;
            self.recovered.lock().unwrap().retain(|a| *a != agent);
            Ok(json!({"ok": true}))
        })
    }
}
