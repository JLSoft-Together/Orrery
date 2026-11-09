pub const FILETIME_EPOCH_MS: i64 = 11_644_473_600_000;

const DAYS: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];
const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

pub fn filetime_ms(ft: u64) -> i64 {
    (ft / 10_000) as i64 - FILETIME_EPOCH_MS
}

fn civil(days: i64) -> (i64, usize, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    (y, (m - 1) as usize, d)
}

pub fn lstart(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let (y, m, d) = civil(days);
    let wd = (days.rem_euclid(7) + 4) % 7;
    format!("{} {} {:>2} {:02}:{:02}:{:02} {}", DAYS[wd as usize], MONTHS[m], d, rem / 3600, rem % 3600 / 60, rem % 60, y)
}

pub fn unix_start(secs: i64) -> (Option<i64>, Option<String>) {
    (secs.checked_mul(1000), Some(lstart(secs)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lstart_is_ps_lstart_in_utc_with_padded_day() {
        assert_eq!(lstart(0), "Thu Jan  1 00:00:00 1970");
        assert_eq!(lstart(1_791_428_482), "Thu Oct  8 03:01:22 2026");
        assert_eq!(lstart(951_782_400), "Tue Feb 29 00:00:00 2000");
        assert_eq!(lstart(1_735_689_599), "Tue Dec 31 23:59:59 2024");
        assert_eq!(unix_start(1_791_428_482).0, Some(1_791_428_482_000));
    }

    #[test]
    fn filetime_ms_matches_the_collector() {
        assert_eq!(filetime_ms(116_444_736_000_000_000), 0);
        assert_eq!(filetime_ms(134_358_400_013_081_891), 1_791_366_401_308);
    }
}
