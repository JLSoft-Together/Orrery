use std::fs;
use std::path::Path;

use serde_json::{json, Value};

use super::json_file::read_json;
use super::localtime::Zone;
use crate::time::iso_of_ms;

pub fn first_start_today(file: &Path, now_ms: i64, zone: Zone) -> String {
    let today = zone.local_date(now_ms);
    if let Some(Value::Object(raw)) = read_json(file) {
        if let (Some(Value::String(date)), Some(Value::String(at))) = (raw.get("date"), raw.get("at")) {
            if *date == today && zone.parse_iso(at).is_some() {
                return at.clone();
            }
        }
    }
    let at = iso_of_ms(now_ms);
    let written = file.parent().map_or(Ok(()), fs::create_dir_all).and_then(|()| fs::write(file, json!({ "date": today, "at": at }).to_string()));
    let _ = written;
    at
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::persist::json_file::temp_dir;
    use crate::persist::localtime::parse_iso;

    #[test]
    fn keeps_the_first_start_of_the_day_across_restarts_and_resets_on_a_new_day() {
        let dir = temp_dir("daystart");
        let file = dir.join("day-start.json");
        let z = Zone::Fixed(7 * 3_600_000);
        let morning = parse_iso("2026-10-08T08:12:00", z).unwrap();
        let first = first_start_today(&file, morning, z);
        assert_eq!(first, iso_of_ms(morning));
        assert_eq!(first_start_today(&file, morning + 3 * 3_600_000, z), first);
        let tomorrow = parse_iso("2026-10-09T09:00:00", z).unwrap();
        assert_eq!(first_start_today(&file, tomorrow, z), iso_of_ms(tomorrow));
        let saved: Value = serde_json::from_str(&fs::read_to_string(&file).unwrap()).unwrap();
        assert_eq!(saved["date"], "2026-10-09");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn unreadable_or_invalid_state_starts_now() {
        let dir = temp_dir("daystart-bad");
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("day-start.json");
        let z = Zone::Fixed(0);
        let now = parse_iso("2026-10-08T08:12:00Z", z).unwrap();
        fs::write(&file, "null").unwrap();
        assert_eq!(first_start_today(&file, now, z), iso_of_ms(now));
        fs::write(&file, r#"{"date":"2026-10-08","at":"soon"}"#).unwrap();
        assert_eq!(first_start_today(&file, now, z), iso_of_ms(now));
        let _ = fs::remove_dir_all(&dir);
    }
}
