mod common;
mod dir;
mod hook;
pub mod outbox;
mod outtail;
mod status;

#[cfg(test)]
mod tests;

use dir::{read_dir, Kind};
use serde_json::{Map, Value};

#[derive(Default)]
pub struct SideFiles;

type Reply = Result<Map<String, Value>, &'static str>;

fn dir_reply(m: &Map<String, Value>, kind: Kind) -> Reply {
    let dir = m
        .get("dir")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or("dir")?;
    let now = m
        .get("now")
        .and_then(Value::as_f64)
        .filter(|n| n.is_finite())
        .ok_or("now")?;
    let records = read_dir(dir, now, kind)?;
    Ok([("records".to_owned(), Value::Array(records))]
        .into_iter()
        .collect())
}

impl SideFiles {
    pub fn handle(&mut self, op: &str, m: &Map<String, Value>) -> Option<Reply> {
        Some(match op {
            "sidefiles.hooks" => dir_reply(m, Kind::Hooks),
            "sidefiles.statusline" => dir_reply(m, Kind::Status),
            "sidefiles.outbox" => outtail::tail(m),
            _ => return None,
        })
    }
}
