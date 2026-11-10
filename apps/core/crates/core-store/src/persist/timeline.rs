use std::path::PathBuf;

use indexmap::IndexMap;
use serde_json::{json, Value};

use super::json_file::JsonFile;
use super::localtime::Zone;
use super::{system_clock, Clock};
use crate::extra::{DayTimeline, LaneStatus, TimelineLane, TimelineSegment};
use crate::model::TerminalStatus;
use crate::num::JsNum;
use crate::time::iso_of_ms;

const MAX_SEGMENTS: usize = 2000;

#[derive(Debug, Clone)]
struct Segment {
    from: f64,
    to: Option<f64>,
    status: LaneStatus,
}

#[derive(Debug, Clone)]
struct Lane {
    title: String,
    segments: Vec<Segment>,
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LaneTotals {
    pub working: f64,
    pub waiting: f64,
    pub idle: f64,
}

fn lane_status_of(status: TerminalStatus) -> Option<LaneStatus> {
    match status {
        TerminalStatus::Working => Some(LaneStatus::Working),
        TerminalStatus::Waiting => Some(LaneStatus::Waiting),
        TerminalStatus::Idle => Some(LaneStatus::Idle),
        _ => None,
    }
}

fn parse_segment(s: &Value) -> Option<Segment> {
    let from = s.get("from")?.as_f64()?;
    let to = match s.get("to") {
        None => None,
        Some(v) => Some(v.as_f64()?),
    };
    let status = match s.get("status")?.as_str()? {
        "working" => LaneStatus::Working,
        "waiting" => LaneStatus::Waiting,
        "idle" => LaneStatus::Idle,
        _ => return None,
    };
    Some(Segment { from, to, status })
}

pub struct StatusTimeline {
    day: String,
    lanes: IndexMap<String, Lane>,
    file: Option<JsonFile>,
    zone: Zone,
    clock: Clock,
}

impl StatusTimeline {
    pub fn new(file: Option<PathBuf>) -> Self {
        Self::with_clock(file, Zone::Local, system_clock())
    }

    pub fn with_clock(file: Option<PathBuf>, zone: Zone, clock: Clock) -> Self {
        let day = zone.local_date(clock());
        Self { day, lanes: IndexMap::new(), file: file.map(|f| JsonFile::new(f, 30_000)), zone, clock }
    }

    fn now(&self) -> i64 {
        (self.clock)()
    }

    pub fn load(&mut self) {
        let Some(Value::Object(raw)) = self.file.as_ref().and_then(JsonFile::read) else { return };
        let (Some(Value::String(day)), Some(Value::Object(lanes))) = (raw.get("day"), raw.get("lanes")) else { return };
        if *day != self.day {
            return;
        }
        let saved_at = raw.get("savedAt").and_then(Value::as_f64).map_or(0.0, |n| n.min(self.now() as f64));
        for (id, lane) in lanes {
            let (Some(Value::String(title)), Some(Value::Array(segments))) = (lane.get("title"), lane.get("segments")) else { continue };
            let mut segments: Vec<Segment> = segments.iter().filter_map(parse_segment).collect();
            if let Some(last) = segments.last_mut() {
                if last.to.is_none() {
                    last.to = Some(last.from.max(saved_at));
                }
            }
            self.lanes.insert(id.clone(), Lane { title: title.clone(), segments });
        }
    }

    fn roll(&mut self, now_ms: i64) {
        let today = self.zone.local_date(now_ms);
        if today == self.day {
            return;
        }
        let start = self.zone.midnight(now_ms) as f64;
        let mut carried: IndexMap<String, Lane> = IndexMap::new();
        for (id, lane) in &self.lanes {
            if let Some(last) = lane.segments.last() {
                if last.to.is_none() {
                    carried.insert(id.clone(), Lane { title: lane.title.clone(), segments: vec![Segment { from: start, to: None, status: last.status }] });
                }
            }
        }
        self.lanes = carried;
        self.day = today;
    }

    pub fn observe(&mut self, id: &str, title: &str, status: TerminalStatus, since: Option<&str>) {
        let now_ms = self.now();
        self.roll(now_ms);
        let next = lane_status_of(status);
        if !self.lanes.contains_key(id) {
            if next.is_none() {
                return;
            }
            self.lanes.insert(id.to_string(), Lane { title: title.to_string(), segments: Vec::new() });
        }
        let midnight = self.zone.midnight(now_ms) as f64;
        let since_ms = since.and_then(|s| self.zone.parse_iso(s)).map(|n| n as f64);
        let Some(lane) = self.lanes.get_mut(id) else { return };
        lane.title = title.to_string();
        let last = lane.segments.last();
        let open = last.filter(|l| l.to.is_none());
        if open.is_some_and(|o| Some(o.status) == next) {
            return;
        }
        if open.is_none() && next.is_none() {
            return;
        }
        let now = now_ms as f64;
        let mut at = since_ms.unwrap_or(now);
        let floor = at.max(midnight).max(open.map_or(0.0, |o| o.from)).max(last.and_then(|l| l.to).unwrap_or(0.0));
        at = now.min(floor);
        if let Some(o) = lane.segments.last_mut().filter(|l| l.to.is_none()) {
            o.to = Some(at);
        }
        if let Some(status) = next {
            lane.segments.push(Segment { from: at, to: None, status });
        }
        if lane.segments.len() > MAX_SEGMENTS {
            let drop = lane.segments.len() - MAX_SEGMENTS;
            lane.segments.drain(0..drop);
        }
        if let Some(f) = &mut self.file {
            f.schedule(now_ms);
        }
    }

    pub fn totals(&self, id: &str) -> LaneTotals {
        let mut out = LaneTotals::default();
        let now = self.now() as f64;
        for s in self.lanes.get(id).map_or(&[][..], |l| &l.segments[..]) {
            let span = s.to.unwrap_or(now) - s.from;
            match s.status {
                LaneStatus::Working => out.working += span,
                LaneStatus::Waiting => out.waiting += span,
                LaneStatus::Idle => out.idle += span,
            }
        }
        out
    }

    pub fn data(&mut self) -> DayTimeline {
        let now_ms = self.now();
        self.roll(now_ms);
        DayTimeline {
            day: self.day.clone(),
            lanes: self
                .lanes
                .iter()
                .map(|(id, lane)| TimelineLane {
                    id: id.clone(),
                    title: lane.title.clone(),
                    segments: lane
                        .segments
                        .iter()
                        .map(|s| TimelineSegment { from: iso_of_ms(s.from as i64), status: s.status, to: s.to.map(|t| iso_of_ms(t as i64)) })
                        .collect(),
                })
                .collect(),
        }
    }

    fn serialize(&self) -> Value {
        let mut lanes = serde_json::Map::new();
        for (id, lane) in &self.lanes {
            let segments: Vec<Value> = lane
                .segments
                .iter()
                .map(|s| {
                    let mut o = serde_json::Map::new();
                    o.insert("from".into(), json!(JsNum(s.from)));
                    o.insert("status".into(), json!(s.status.as_str()));
                    if let Some(t) = s.to {
                        o.insert("to".into(), json!(JsNum(t)));
                    }
                    Value::Object(o)
                })
                .collect();
            lanes.insert(id.clone(), json!({ "title": lane.title, "segments": segments }));
        }
        json!({ "day": self.day, "savedAt": self.now(), "lanes": lanes })
    }

    pub fn next_flush_at(&self) -> Option<i64> {
        self.file.as_ref().and_then(JsonFile::next_due)
    }

    pub fn poll(&mut self, now: i64) -> bool {
        let state = self.serialize();
        self.file.as_mut().is_some_and(|f| f.poll(now, || state))
    }

    pub fn save(&mut self) -> bool {
        let state = self.serialize();
        self.file.as_mut().is_some_and(|f| f.flush(|| state))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persist::json_file::temp_dir;
    use std::fs;
    use std::sync::atomic::{AtomicI64, Ordering};
    use std::sync::Arc;

    const T0: i64 = 1_800_000_000_000;

    fn rig(file: Option<PathBuf>, start: i64) -> (StatusTimeline, Arc<AtomicI64>) {
        let t = Arc::new(AtomicI64::new(start));
        let c = t.clone();
        (StatusTimeline::with_clock(file, Zone::Fixed(0), Box::new(move || c.load(Ordering::SeqCst))), t)
    }

    fn segs(tl: &mut StatusTimeline) -> Vec<(String, Option<String>, &'static str)> {
        tl.data().lanes.into_iter().flat_map(|l| l.segments).map(|s| (s.from, s.to, s.status.as_str())).collect()
    }

    #[test]
    fn status_changes_add_and_close_segments() {
        let (mut tl, t) = rig(None, T0);
        tl.observe("a", "A", TerminalStatus::Stale, None);
        assert!(tl.data().lanes.is_empty());
        tl.observe("a", "A", TerminalStatus::Working, None);
        t.fetch_add(60_000, Ordering::SeqCst);
        tl.observe("a", "A2", TerminalStatus::Working, None);
        tl.observe("a", "A2", TerminalStatus::Waiting, Some(&iso_of_ms(T0 + 30_000)));
        t.fetch_add(60_000, Ordering::SeqCst);
        tl.observe("a", "A2", TerminalStatus::Error, None);
        let s = segs(&mut tl);
        assert_eq!(s.len(), 2);
        assert_eq!((s[0].0.clone(), s[0].1.clone(), s[0].2), (iso_of_ms(T0), Some(iso_of_ms(T0 + 30_000)), "working"));
        assert_eq!((s[1].1.clone(), s[1].2), (Some(iso_of_ms(T0 + 120_000)), "waiting"));
        assert_eq!(tl.data().lanes[0].title, "A2");
        let totals = tl.totals("a");
        assert_eq!((totals.working, totals.waiting, totals.idle), (30_000.0, 90_000.0, 0.0));
    }

    #[test]
    fn rolls_at_midnight_carrying_open_segments() {
        let (mut tl, t) = rig(None, 1_800_000_000_000);
        tl.observe("a", "A", TerminalStatus::Working, None);
        tl.observe("b", "B", TerminalStatus::Idle, None);
        tl.observe("b", "B", TerminalStatus::Stale, None);
        t.fetch_add(86_400_000, Ordering::SeqCst);
        let data = tl.data();
        assert_eq!(data.day, "2027-01-16");
        assert_eq!(data.lanes.len(), 1);
        assert_eq!(data.lanes[0].segments[0].from, iso_of_ms(1_800_057_600_000));
    }

    #[test]
    fn persists_and_closes_open_segments_at_the_saved_time() {
        let dir = temp_dir("timeline");
        let file = dir.join("timeline.json");
        let (mut tl, t) = rig(Some(file.clone()), T0);
        tl.observe("a", "A", TerminalStatus::Working, None);
        assert_eq!(tl.next_flush_at(), Some(T0 + 30_000));
        t.fetch_add(45_000, Ordering::SeqCst);
        assert!(tl.poll(T0 + 45_000));
        let (mut again, _) = rig(Some(file.clone()), T0 + 600_000);
        again.load();
        assert_eq!(segs(&mut again), [(iso_of_ms(T0), Some(iso_of_ms(T0 + 45_000)), "working")]);
        fs::write(&file, r#"{"day":"2027-01-15","lanes":{"x":{"title":"X","segments":[{"from":1,"to":null,"status":"idle"},{"from":2,"status":"idle"},{"from":3,"status":"bogus"}]},"y":null,"z":{"title":3}}}"#).unwrap();
        let (mut odd, _) = rig(Some(file), T0);
        odd.load();
        assert_eq!(odd.data().lanes.len(), 1);
        assert_eq!(odd.data().lanes[0].segments.len(), 1);
        let _ = fs::remove_dir_all(&dir);
    }
}
