pub const KINFO_SIZE: usize = 648;
const OFF_START_SEC: usize = 0;
const OFF_PID: usize = 40;
const OFF_COMM: usize = 243;
const COMM_LEN: usize = 17;
const OFF_PPID: usize = 560;

#[derive(Debug, PartialEq)]
pub struct Kinfo {
    pub pid: u32,
    pub ppid: u32,
    pub start_sec: i64,
    pub comm: String,
}

fn i32_at(b: &[u8], off: usize) -> i32 {
    i32::from_ne_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}

pub fn parse_kinfo(buf: &[u8]) -> Vec<Kinfo> {
    buf.as_chunks::<KINFO_SIZE>()
        .0
        .iter()
        .filter_map(|r| {
            let pid = u32::try_from(i32_at(r, OFF_PID)).ok()?;
            let ppid = u32::try_from(i32_at(r, OFF_PPID)).ok()?;
            let mut sec = [0u8; 8];
            sec.copy_from_slice(&r[OFF_START_SEC..OFF_START_SEC + 8]);
            let comm = &r[OFF_COMM..OFF_COMM + COMM_LEN];
            let len = comm.iter().position(|&c| c == 0).unwrap_or(COMM_LEN);
            Some(Kinfo { pid, ppid, start_sec: i64::from_ne_bytes(sec), comm: String::from_utf8_lossy(&comm[..len]).into_owned() })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(pid: i32, ppid: i32, sec: i64, comm: &[u8]) -> Vec<u8> {
        let mut r = vec![0xAAu8; KINFO_SIZE];
        r[OFF_START_SEC..OFF_START_SEC + 8].copy_from_slice(&sec.to_ne_bytes());
        r[OFF_PID..OFF_PID + 4].copy_from_slice(&pid.to_ne_bytes());
        r[OFF_PPID..OFF_PPID + 4].copy_from_slice(&ppid.to_ne_bytes());
        r[OFF_COMM..OFF_COMM + COMM_LEN].fill(0);
        r[OFF_COMM..OFF_COMM + comm.len()].copy_from_slice(comm);
        r
    }

    #[test]
    fn parses_kinfo_proc_records_at_their_offsets() {
        let mut buf = record(1, 0, 1_791_428_482, b"launchd");
        buf.extend(record(512, 1, 1_791_428_500, b"abcdefghijklmnopq"));
        buf.extend(record(-1, 1, 0, b"bad"));
        buf.extend([0u8; 10]);
        let rows = parse_kinfo(&buf);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], Kinfo { pid: 1, ppid: 0, start_sec: 1_791_428_482, comm: "launchd".into() });
        assert_eq!(rows[1].comm, "abcdefghijklmnopq");
        const { assert!(OFF_PPID + 4 <= KINFO_SIZE && OFF_COMM + COMM_LEN <= KINFO_SIZE) };
    }
}
