use std::cell::RefCell;
use std::collections::HashMap;

pub const DAY_MS: i64 = 86_400_000;
const SLOT_MS: i64 = 900_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zone {
    Local,
    Fixed(i64),
}

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

thread_local! {
    static OFFSETS: RefCell<HashMap<i64, i64>> = RefCell::new(HashMap::new());
}

impl Zone {
    pub fn offset_ms(self, utc_ms: i64) -> i64 {
        match self {
            Zone::Fixed(v) => v,
            Zone::Local => {
                let slot = utc_ms.div_euclid(SLOT_MS);
                OFFSETS.with(|c| *c.borrow_mut().entry(slot).or_insert_with(|| sys::offset_at(slot * SLOT_MS)))
            }
        }
    }

    pub fn local_date(self, utc_ms: i64) -> String {
        let local = utc_ms + self.offset_ms(utc_ms);
        let (y, m, d) = civil_from_days(local.div_euclid(DAY_MS));
        format!("{y}-{m:02}-{d:02}")
    }

    pub fn from_local(self, civil_ms: i64) -> i64 {
        let guess = civil_ms - self.offset_ms(civil_ms);
        civil_ms - self.offset_ms(guess)
    }

    pub fn midnight(self, utc_ms: i64) -> i64 {
        let local = utc_ms + self.offset_ms(utc_ms);
        self.from_local(local.div_euclid(DAY_MS) * DAY_MS)
    }

    pub fn parse_iso(self, s: &str) -> Option<i64> {
        parse_iso(s, self)
    }
}

fn digits(b: &[u8], at: usize, n: usize) -> Option<i64> {
    let s = b.get(at..at + n)?;
    s.iter().all(u8::is_ascii_digit).then(|| s.iter().fold(0i64, |a, d| a * 10 + i64::from(d - b'0')))
}

pub fn parse_iso(s: &str, zone: Zone) -> Option<i64> {
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
        None => Some(zone.from_local(clock)),
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
        let z = Zone::Fixed(0);
        assert_eq!(parse_iso("2026-10-07T09:00:00.000Z", z), Some(1_791_363_600_000));
        assert_eq!(parse_iso("2026-10-07T09:00:00+07:00", z), Some(1_791_363_600_000 - 7 * 3_600_000));
        assert_eq!(parse_iso("2026-10-07", z), Some(1_791_331_200_000));
        assert_eq!(parse_iso("2026-10-07T09:00:00", Zone::Fixed(7 * 3_600_000)), Some(1_791_363_600_000 - 7 * 3_600_000));
        assert_eq!(parse_iso("2026-13-07", z), None);
        assert_eq!(parse_iso("soon", z), None);
    }

    #[test]
    fn local_dates_and_midnight_follow_the_zone() {
        let z = Zone::Fixed(7 * 3_600_000);
        let t = parse_iso("2026-10-07T20:30:00.000Z", z).unwrap();
        assert_eq!(z.local_date(t), "2026-10-08");
        assert_eq!(z.midnight(t), parse_iso("2026-10-07T17:00:00.000Z", z).unwrap());
        let local = Zone::Local;
        let m = local.midnight(t);
        assert!(m <= t && t - m < DAY_MS + 3_600_000);
        assert_eq!(local.local_date(m), local.local_date(t));
    }
}
