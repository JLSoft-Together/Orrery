use std::cell::RefCell;
use std::collections::HashMap;

const DAY_MS: i64 = 86_400_000;
const SLOT_MS: i64 = 900_000;

pub fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

pub fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { yoe + era * 400 + 1 } else { yoe + era * 400 }, m, d)
}

fn day_string(days: i64) -> String {
    let (y, m, d) = civil_from_days(days);
    format!("{y}-{m:02}-{d:02}")
}

thread_local! {
    static OFFSETS: RefCell<HashMap<i64, i64>> = RefCell::new(HashMap::new());
}

pub fn local_offset_ms(utc_ms: i64) -> i64 {
    let slot = utc_ms.div_euclid(SLOT_MS);
    OFFSETS.with(|c| *c.borrow_mut().entry(slot).or_insert_with(|| sys::offset_at(slot * SLOT_MS)))
}

pub fn local_day_hour(utc_ms: i64) -> (String, u32) {
    let local = utc_ms + local_offset_ms(utc_ms);
    let days = local.div_euclid(DAY_MS);
    let hour = local.rem_euclid(DAY_MS) / 3_600_000;
    (day_string(days), hour as u32)
}

fn digits(b: &[u8], at: usize, n: usize) -> Option<i64> {
    let s = b.get(at..at + n)?;
    s.iter().all(u8::is_ascii_digit).then(|| s.iter().fold(0i64, |a, d| a * 10 + i64::from(d - b'0')))
}

pub fn parse_iso(s: &str) -> Option<i64> {
    let b = s.as_bytes();
    let year = digits(b, 0, 4)?;
    let (mut month, mut day, mut at) = (1, 1, 4);
    if b.get(at) == Some(&b'-') {
        month = digits(b, at + 1, 2)?;
        at += 3;
        if b.get(at) == Some(&b'-') {
            day = digits(b, at + 1, 2)?;
            at += 3;
        }
    }
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let date = days_from_civil(year, month, day) * DAY_MS;
    if at == b.len() {
        return Some(date);
    }
    if !matches!(b.get(at), Some(b'T' | b' ')) {
        return None;
    }
    let hour = digits(b, at + 1, 2)?;
    if b.get(at + 3) != Some(&b':') {
        return None;
    }
    let minute = digits(b, at + 4, 2)?;
    at += 6;
    let (mut second, mut ms) = (0, 0);
    if b.get(at) == Some(&b':') {
        second = digits(b, at + 1, 2)?;
        at += 3;
        if b.get(at) == Some(&b'.') {
            let start = at + 1;
            let mut end = start;
            while b.get(end).is_some_and(u8::is_ascii_digit) {
                end += 1;
            }
            if end == start {
                return None;
            }
            let take = (end - start).min(3);
            ms = digits(b, start, take)? * 10i64.pow(3 - take as u32);
            at = end;
        }
    }
    if hour > 24 || minute > 59 || second > 59 || (hour == 24 && (minute | second | ms) != 0) {
        return None;
    }
    let clock = date + hour * 3_600_000 + minute * 60_000 + second * 1000 + ms;
    match b.get(at) {
        None => {
            let guess = clock - local_offset_ms(clock);
            Some(clock - local_offset_ms(guess))
        }
        Some(b'Z') if at + 1 == b.len() => Some(clock),
        Some(&sign @ (b'+' | b'-')) if at + 6 == b.len() && b[at + 3] == b':' => {
            let oh = digits(b, at + 1, 2)?;
            let om = digits(b, at + 4, 2)?;
            if oh > 23 || om > 59 {
                return None;
            }
            let off = (oh * 60 + om) * 60_000;
            Some(if sign == b'+' { clock - off } else { clock + off })
        }
        _ => None,
    }
}

pub fn local_day(iso: &str) -> Option<String> {
    parse_iso(iso).map(|t| local_day_hour(t).0)
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_millis() as i64)
}

pub fn today(now: i64) -> String {
    local_day_hour(now).0
}

pub fn last_days(days: u32, today: &str) -> Vec<String> {
    let parts: Vec<i64> = today.split('-').filter_map(|p| p.parse().ok()).collect();
    let &[y, m, d] = parts.as_slice() else { return Vec::new() };
    let base = days_from_civil(y + (m - 1).div_euclid(12), (m - 1).rem_euclid(12) + 1, 1) + d - 1;
    (0..i64::from(days)).rev().map(|i| day_string(base - i)).collect()
}

#[cfg(windows)]
mod sys {
    use std::sync::OnceLock;
    use windows_sys::Win32::Foundation::{FILETIME, SYSTEMTIME};
    use windows_sys::Win32::System::Time::{FileTimeToSystemTime, GetDynamicTimeZoneInformation, SystemTimeToTzSpecificLocalTimeEx, DYNAMIC_TIME_ZONE_INFORMATION};

    struct Tz(DYNAMIC_TIME_ZONE_INFORMATION);
    unsafe impl Send for Tz {}
    unsafe impl Sync for Tz {}

    fn zone() -> &'static Tz {
        static ZONE: OnceLock<Tz> = OnceLock::new();
        ZONE.get_or_init(|| {
            let mut z: DYNAMIC_TIME_ZONE_INFORMATION = unsafe { std::mem::zeroed() };
            unsafe { GetDynamicTimeZoneInformation(&mut z) };
            Tz(z)
        })
    }

    fn ms_of(st: &SYSTEMTIME) -> i64 {
        let days = super::days_from_civil(i64::from(st.wYear), i64::from(st.wMonth), i64::from(st.wDay));
        days * super::DAY_MS + i64::from(st.wHour) * 3_600_000 + i64::from(st.wMinute) * 60_000 + i64::from(st.wSecond) * 1000 + i64::from(st.wMilliseconds)
    }

    pub fn offset_at(utc_ms: i64) -> i64 {
        let ticks = utc_ms.saturating_mul(10_000).saturating_add(116_444_736_000_000_000);
        if ticks < 0 {
            return 0;
        }
        let ft = FILETIME { dwLowDateTime: ticks as u32, dwHighDateTime: (ticks >> 32) as u32 };
        let mut utc: SYSTEMTIME = unsafe { std::mem::zeroed() };
        let mut local: SYSTEMTIME = unsafe { std::mem::zeroed() };
        if unsafe { FileTimeToSystemTime(&ft, &mut utc) } == 0 {
            return 0;
        }
        if unsafe { SystemTimeToTzSpecificLocalTimeEx(&zone().0, &utc, &mut local) } == 0 {
            return 0;
        }
        ms_of(&local) - ms_of(&utc)
    }
}

#[cfg(unix)]
mod sys {
    pub fn offset_at(utc_ms: i64) -> i64 {
        let t = utc_ms.div_euclid(1000) as libc::time_t;
        let mut tm: libc::tm = unsafe { std::mem::zeroed() };
        if unsafe { libc::localtime_r(&t, &mut tm) }.is_null() {
            return 0;
        }
        tm.tm_gmtoff as i64 * 1000
    }
}

#[cfg(not(any(windows, unix)))]
mod sys {
    pub fn offset_at(_utc_ms: i64) -> i64 {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_iso_like_date_parse() {
        assert_eq!(parse_iso("2026-10-07T09:00:00.000Z"), Some(1_791_363_600_000));
        assert_eq!(parse_iso("2026-10-07T09:00:00.1234Z"), Some(1_791_363_600_123));
        assert_eq!(parse_iso("2026-10-07T09:00:00+07:00"), Some(1_791_363_600_000 - 7 * 3_600_000));
        assert_eq!(parse_iso("2026-10-07"), Some(1_791_331_200_000));
        assert_eq!(parse_iso("2026-13-07"), None);
        assert_eq!(parse_iso("yesterday"), None);
        assert_eq!(parse_iso("2026-10-07T09:00:00.000Zx"), None);
        assert_eq!(parse_iso("2026-10-07T25:00Z"), None);
    }

    #[test]
    fn civil_round_trips_and_lists_days() {
        for z in [-800_000, -1, 0, 1, 20_000, 2_000_000] {
            let (y, m, d) = civil_from_days(z);
            assert_eq!(days_from_civil(y, m, d), z);
        }
        assert_eq!(last_days(3, "2026-03-01"), vec!["2026-02-27", "2026-02-28", "2026-03-01"]);
        assert_eq!(last_days(2, "2024-01-01"), vec!["2023-12-31", "2024-01-01"]);
        assert!(last_days(0, "2024-01-01").is_empty());
    }

    #[test]
    fn local_day_follows_the_offset() {
        let t = parse_iso("2026-10-07T23:30:00.000Z").unwrap();
        let (day, hour) = local_day_hour(t);
        let local = t + local_offset_ms(t);
        assert_eq!(day, day_string(local.div_euclid(DAY_MS)));
        assert_eq!(i64::from(hour), local.rem_euclid(DAY_MS) / 3_600_000);
    }
}
