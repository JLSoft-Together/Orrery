use super::{Draft, Toaster};

pub const GROUP_WINDOW_MS: f64 = 8_000.0;
pub const GROUP_KEY: &str = "group:all";
const NAMES_SHOWN: usize = 4;

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Member {
    pub id: String,
    pub name: String,
    pub waiting: bool,
    pub url: String,
}

#[derive(Debug, Default)]
pub(crate) struct Group {
    pub started: f64,
    pub members: Vec<Member>,
}

impl Group {
    pub fn add(&mut self, now: f64, m: Member) -> usize {
        if !self.members.is_empty() && now - self.started > GROUP_WINDOW_MS {
            self.members.clear();
        }
        if self.members.is_empty() {
            self.started = now;
        }
        self.members.retain(|x| x.id != m.id);
        self.members.push(m);
        self.members.len()
    }

    pub fn counts(&self) -> (usize, usize) {
        let waiting = self.members.iter().filter(|m| m.waiting).count();
        (self.members.len() - waiting, waiting)
    }

    pub fn lead(&self) -> Option<&Member> {
        self.members.iter().find(|m| m.waiting).or(self.members.first())
    }

    pub fn names(&self) -> Vec<&str> {
        self.members.iter().map(|m| m.name.as_str()).collect()
    }

    pub fn clear(&mut self) {
        self.members.clear();
    }
}

impl Toaster {
    pub(crate) fn show_grouped(&mut self, key: &str, id: &str, name: &str, waiting: bool, d: Draft) {
        let now = self.now();
        let member = Member { id: id.to_string(), name: name.to_string(), waiting, url: d.url.clone() };
        if self.group.add(now, member) < 2 {
            self.show(key, d);
            return;
        }
        let keys: Vec<String> = self.group.members.iter().flat_map(|m| [format!("done:{}", m.id), format!("waiting:{}", m.id)]).collect();
        for k in keys {
            self.hide_key(&k);
        }
        let text = self.text();
        let (done, waits) = self.group.counts();
        let names = self.group.names();
        let shown = names.iter().take(NAMES_SHOWN).copied().collect::<Vec<_>>().join(", ");
        let more = names.len().saturating_sub(NAMES_SHOWN);
        let body = if more > 0 { format!("{shown}{}", text.group_more(more)) } else { shown };
        let url = self.group.lead().map_or_else(|| self.link_for(None), |m| m.url.clone());
        let merged = Draft { title: text.group_title(done, waits), body, url, open: text.open(), persistent: waits > 0, snooze: false };
        self.show(GROUP_KEY, merged);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(id: &str, waiting: bool) -> Member {
        Member { id: id.into(), name: id.into(), waiting, url: format!("u:{id}") }
    }

    #[test]
    fn members_join_inside_the_window_and_restart_after_it() {
        let mut g = Group::default();
        assert_eq!(g.add(0.0, m("a", false)), 1);
        assert_eq!(g.add(7_999.0, m("b", false)), 2);
        assert_eq!(g.add(8_001.0, m("c", true)), 1);
        assert_eq!(g.started, 8_001.0);
    }

    #[test]
    fn the_same_session_counts_once() {
        let mut g = Group::default();
        g.add(0.0, m("a", false));
        assert_eq!(g.add(1_000.0, m("a", true)), 1);
        assert_eq!(g.counts(), (0, 1));
    }

    #[test]
    fn a_waiting_session_leads() {
        let mut g = Group::default();
        g.add(0.0, m("a", false));
        g.add(1.0, m("b", true));
        g.add(2.0, m("c", false));
        assert_eq!(g.lead().map(|x| x.id.as_str()), Some("b"));
        assert_eq!(g.counts(), (2, 1));
    }
}
