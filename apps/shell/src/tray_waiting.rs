use std::sync::Mutex;
use std::time::Duration;

use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager};

use crate::state::{lock, Shell, MAIN};
use crate::window::show_main;

pub const MAX_ROWS: usize = 8;
pub const MAX_TEXT: usize = 60;
const MAX_ID: usize = 200;
const MAX_FIELD: usize = 30;
pub const PREFIX: &str = "wait:";
const FOCUS_ASK: Duration = Duration::from_millis(1500);

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Row {
    pub id: String,
    pub agent: String,
    pub project: String,
    pub question: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Waiting {
    pub total: usize,
    pub rows: Vec<Row>,
}

static CURRENT: Mutex<Option<Waiting>> = Mutex::new(None);

pub fn clip(text: &str, max: usize) -> String {
    let flat = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= max {
        return flat;
    }
    let head: String = flat.chars().take(max.saturating_sub(1)).collect();
    format!("{}…", head.trim_end())
}

pub fn parse(raw: &Value) -> Waiting {
    let list = raw.get("rows").and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[]);
    let mut rows = Vec::new();
    for item in list {
        let Some(id) = item.get("id").and_then(Value::as_str).filter(|s| !s.is_empty() && s.chars().count() <= MAX_ID) else { continue };
        if rows.iter().any(|r: &Row| r.id == id) {
            continue;
        }
        let field = |k: &str, max: usize| clip(item.get(k).and_then(Value::as_str).unwrap_or(""), max);
        rows.push(Row { id: id.to_owned(), agent: field("agent", MAX_FIELD), project: field("project", MAX_FIELD), question: field("question", MAX_TEXT) });
    }
    let total = raw.get("total").and_then(Value::as_u64).map_or(rows.len(), |n| n as usize).max(rows.len());
    rows.truncate(MAX_ROWS);
    Waiting { total, rows }
}

pub fn row_label(row: &Row) -> String {
    let who = match (row.agent.is_empty(), row.project.is_empty()) {
        (false, false) => format!("{} - {}", row.agent, row.project),
        (false, true) => row.agent.clone(),
        (true, false) => row.project.clone(),
        (true, true) => String::new(),
    };
    match (who.is_empty(), row.question.is_empty()) {
        (_, true) => who,
        (true, false) => row.question.clone(),
        (false, false) => format!("{who}: {}", row.question),
    }
}

pub fn item_id(row: &Row) -> String {
    format!("{PREFIX}{}", row.id)
}

pub fn terminal_of(menu_id: &str) -> Option<&str> {
    menu_id.strip_prefix(PREFIX).filter(|s| !s.is_empty())
}

pub fn header(template: &str, total: usize) -> String {
    template.replace("{n}", &total.to_string())
}

pub fn tooltip(template: &str, total: usize) -> String {
    if total == 0 {
        "Orrery".to_owned()
    } else {
        format!("Orrery - {}", template.replace("{n}", &total.to_string()))
    }
}

pub const HEAD_ID: &str = "wait-head";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    pub id: String,
    pub label: String,
    pub enabled: bool,
}

pub fn entries(w: &Waiting, header_template: &str) -> Vec<Entry> {
    if w.rows.is_empty() {
        return Vec::new();
    }
    let mut out = vec![Entry { id: HEAD_ID.to_owned(), label: header(header_template, w.total), enabled: false }];
    for row in &w.rows {
        let label = row_label(row);
        out.push(Entry { id: item_id(row), label: if label.is_empty() { row.id.clone() } else { label }, enabled: true });
    }
    out
}

pub fn current() -> Waiting {
    lock(&CURRENT).clone().unwrap_or_default()
}

pub fn set(raw: &Value) -> bool {
    let next = parse(raw);
    let mut slot = lock(&CURRENT);
    if slot.as_ref().unwrap_or(&Waiting::default()) == &next {
        return false;
    }
    *slot = Some(next);
    true
}

pub fn open(app: &AppHandle, terminal: String) {
    let handle = app.clone();
    std::thread::spawn(move || {
        show_main(&handle);
        let shell = handle.state::<Shell>();
        let handled = shell.ask(&handle, "focus", "ccm:focus", json!(terminal), FOCUS_ASK);
        if handled != Some(Value::Bool(true)) {
            let _ = handle.emit_to(MAIN, "ccm:select", terminal);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows(n: usize) -> Value {
        let list: Vec<Value> = (0..n).map(|i| json!({ "id": format!("t{i}"), "agent": "Claude", "project": "alpha", "question": "Run it?" })).collect();
        json!({ "total": n, "rows": list })
    }

    #[test]
    fn keeps_at_most_eight_rows_and_the_real_total() {
        let w = parse(&rows(11));
        assert_eq!(w.rows.len(), MAX_ROWS);
        assert_eq!(w.total, 11);
        assert_eq!(w.rows[0].id, "t0");
    }

    #[test]
    fn cuts_long_text_to_sixty_chars_and_joins_lines() {
        let w = parse(&json!({ "rows": [{ "id": "a", "question": format!("{}\nmore", "x".repeat(100)) }] }));
        assert_eq!(w.rows[0].question.chars().count(), MAX_TEXT);
        assert!(w.rows[0].question.ends_with('…'));
        assert!(!w.rows[0].question.contains('\n'));
        assert_eq!(clip("short  text", MAX_TEXT), "short text");
    }

    #[test]
    fn counts_characters_not_bytes() {
        let w = parse(&json!({ "rows": [{ "id": "a", "question": "ế".repeat(80) }] }));
        assert_eq!(w.rows[0].question.chars().count(), MAX_TEXT);
    }

    #[test]
    fn drops_rows_without_id_and_duplicates_and_junk_input() {
        let w = parse(&json!({ "rows": [{ "agent": "x" }, { "id": "" }, { "id": "a" }, { "id": "a" }, 5, null] }));
        assert_eq!(w.rows.len(), 1);
        assert_eq!(w.total, 1);
        assert_eq!(parse(&Value::Null), Waiting::default());
        assert_eq!(parse(&json!({ "rows": "no" })), Waiting::default());
    }

    #[test]
    fn labels_join_what_is_present() {
        let full = Row { id: "a".into(), agent: "Claude".into(), project: "alpha".into(), question: "Run it?".into() };
        assert_eq!(row_label(&full), "Claude - alpha: Run it?");
        assert_eq!(row_label(&Row { question: String::new(), ..full.clone() }), "Claude - alpha");
        assert_eq!(row_label(&Row { agent: String::new(), project: String::new(), ..full.clone() }), "Run it?");
        assert_eq!(row_label(&Row { id: "a".into(), ..Row::default() }), "");
    }

    #[test]
    fn menu_ids_round_trip_to_the_terminal() {
        let r = Row { id: "term-9".into(), ..Row::default() };
        assert_eq!(item_id(&r), "wait:term-9");
        assert_eq!(terminal_of(&item_id(&r)), Some("term-9"));
        assert_eq!(terminal_of("open"), None);
        assert_eq!(terminal_of("wait:"), None);
    }

    #[test]
    fn header_and_tooltip_carry_the_count() {
        assert_eq!(header("Waiting for you ({n})", 3), "Waiting for you (3)");
        assert_eq!(tooltip("{n} waiting", 2), "Orrery - 2 waiting");
        assert_eq!(tooltip("{n} waiting", 0), "Orrery");
    }

    #[test]
    fn menu_entries_are_a_header_then_the_rows_and_empty_when_nobody_waits() {
        assert!(entries(&Waiting::default(), "Waiting ({n})").is_empty());
        let w = parse(&rows(11));
        let e = entries(&w, "Waiting for you ({n})");
        assert_eq!(e.len(), 1 + MAX_ROWS);
        assert_eq!(e[0], Entry { id: HEAD_ID.into(), label: "Waiting for you (11)".into(), enabled: false });
        assert!(e[1..].iter().all(|x| x.enabled && terminal_of(&x.id).is_some()));
        assert_eq!(e[1].label, "Claude - alpha: Run it?");
    }

    #[test]
    fn set_reports_a_change_only_once() {
        let raw = rows(2);
        assert!(set(&raw));
        assert!(!set(&raw));
        assert_eq!(current().rows.len(), 2);
        assert!(set(&rows(3)));
        assert!(set(&Value::Null));
        assert_eq!(current(), Waiting::default());
    }
}
