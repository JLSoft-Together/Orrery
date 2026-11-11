use super::{text, Control, Hub, Mode, Out, Viewer};
use serde_json::{json, Map, Value};
use std::sync::atomic::Ordering;

fn resized(size: (u16, u16)) -> Out {
    text(json!({ "t": "resized", "cols": size.0, "rows": size.1 }))
}

pub(super) fn attach_mode(msg: &Map<String, Value>) -> (Mode, Option<(u16, u16)>) {
    let mode = if msg.get("mode").and_then(Value::as_str) == Some("view") { Mode::View } else { Mode::Own };
    let dim = |k: &str| match msg.get(k) {
        None | Some(Value::Null) => None,
        v => Some(crate::session::clamp_dim(v, 0)).filter(|n| *n > 0),
    };
    (mode, dim("cols").zip(dim("rows")))
}

impl<C: Control> Hub<C> {
    pub(super) fn fanout(&self, key: u64, skip: Option<u64>, out: Out) {
        let Some(s) = self.sessions.get(&key) else { return };
        let mut targets = s.viewers.iter().map(|v| v.conn).filter(|c| Some(*c) != skip).peekable();
        while let Some(c) = targets.next() {
            if targets.peek().is_none() {
                self.send(Some(c), out);
                return;
            }
            self.send(Some(c), out.clone());
        }
    }

    pub(super) fn leave_all(&mut self, conn: u64, except: Option<u64>) {
        let keys: Vec<u64> = self.sessions.iter().filter(|(k, s)| Some(**k) != except && s.viewers.iter().any(|v| v.conn == conn)).map(|(k, _)| *k).collect();
        for k in keys {
            self.leave(k, conn);
        }
    }

    pub(super) fn leave(&mut self, key: u64, conn: u64) {
        let Some(s) = self.sessions.get_mut(&key) else { return };
        let Some(i) = s.viewers.iter().position(|v| v.conn == conn) else { return };
        s.viewers.remove(i);
        if let Some(c) = self.conns.get(&conn) {
            if c.meter.bound_to(&s.gate) {
                c.meter.bind(None);
            }
        }
        if s.input_owner == Some(conn) {
            let conns = &self.conns;
            let live = |v: &&Viewer| conns.get(&v.conn).is_none_or(|c| !c.overflowed.load(Ordering::Relaxed));
            s.input_owner = s.viewers.iter().filter(live).find(|v| v.mode == Mode::Own).or_else(|| s.viewers.iter().find(live)).map(|v| v.conn);
            if let Some(o) = s.input_owner {
                self.fit_owner(key, o);
            }
        }
        self.rebind(key);
    }

    pub(super) fn join(&mut self, key: u64, conn: u64, mode: Mode, size: Option<(u16, u16)>) -> Vec<u64> {
        let Some(s) = self.sessions.get_mut(&key) else { return Vec::new() };
        s.viewers.retain(|v| v.conn != conn);
        let replaced: Vec<u64> = if mode == Mode::Own { s.viewers.iter().filter(|v| v.mode == Mode::Own).map(|v| v.conn).collect() } else { Vec::new() };
        if s.input_owner.is_some_and(|o| replaced.contains(&o)) {
            s.input_owner = None;
        }
        s.viewers.retain(|v| !replaced.contains(&v.conn));
        s.viewers.push(Viewer { conn, mode, size });
        let owner_views = s.input_owner.is_some_and(|o| s.viewers.iter().any(|v| v.conn == o && v.mode == Mode::View));
        let takes = mode == Mode::Own && (s.input_owner.is_none() || owner_views);
        for o in &replaced {
            if let Some(c) = self.conns.get(o) {
                if c.meter.bound_to(&s.gate) {
                    c.meter.bind(None);
                }
            }
        }
        if takes {
            s.input_owner = Some(conn);
            self.fit_owner(key, conn);
        }
        replaced
    }

    pub(super) fn claim(&mut self, key: u64, conn: u64) {
        let Some(s) = self.sessions.get_mut(&key) else { return };
        if s.input_owner == Some(conn) || s.exit_code.is_some() || !s.viewers.iter().any(|v| v.conn == conn) {
            return;
        }
        s.input_owner = Some(conn);
        self.fit_owner(key, conn);
        self.rebind(key);
    }

    pub(super) fn resize_from(&mut self, key: u64, conn: u64, size: (u16, u16)) {
        let Some(s) = self.sessions.get_mut(&key) else { return };
        let Some(v) = s.viewers.iter_mut().find(|v| v.conn == conn) else { return };
        v.size = Some(size);
        if s.input_owner.is_some_and(|o| o != conn) || s.exit_code.is_some() {
            return;
        }
        s.proc.resize(size.0, size.1);
        if s.size != size {
            s.size = size;
            self.fanout(key, Some(conn), resized(size));
        }
    }

    fn fit_owner(&mut self, key: u64, owner: u64) {
        let Some(s) = self.sessions.get_mut(&key) else { return };
        let Some(size) = s.viewers.iter().find(|v| v.conn == owner).and_then(|v| v.size) else { return };
        if size == s.size || s.exit_code.is_some() {
            return;
        }
        s.proc.resize(size.0, size.1);
        s.size = size;
        self.fanout(key, Some(owner), resized(size));
    }

    pub(super) fn rebind(&self, key: u64) {
        let Some(s) = self.sessions.get(&key) else { return };
        for v in &s.viewers {
            let Some(c) = self.conns.get(&v.conn) else { continue };
            let binds = (v.mode == Mode::Own || s.input_owner == Some(v.conn)) && !c.overflowed.load(Ordering::Relaxed);
            if binds && !c.meter.bound_to(&s.gate) {
                c.meter.bind(Some(s.gate.clone()));
            } else if !binds && c.meter.bound_to(&s.gate) {
                c.meter.bind(None);
            }
        }
    }

    pub(super) fn size_note(&self, key: u64) -> Option<Out> {
        self.sessions.get(&key).map(|s| resized(s.size))
    }
}
