use crate::tcp::{TcpRow, TCP_ESTABLISHED, TCP_LISTEN};

#[derive(Debug, PartialEq)]
pub struct Stat {
    pub comm: String,
    pub ppid: u32,
    pub utime: u64,
    pub stime: u64,
    pub start_ticks: u64,
    pub rss_pages: u64,
}

pub fn parse_stat(text: &str) -> Option<Stat> {
    let open = text.find('(')?;
    let close = text.rfind(')')?;
    if close < open {
        return None;
    }
    let comm = text[open + 1..close].to_owned();
    let f: Vec<&str> = text[close + 1..].split_whitespace().collect();
    let num = |i: usize| f.get(i).and_then(|s| s.parse::<u64>().ok());
    Some(Stat { comm, ppid: u32::try_from(num(1)?).ok()?, utime: num(11)?, stime: num(12)?, start_ticks: num(19)?, rss_pages: num(21).unwrap_or(0) })
}

pub fn parse_btime(text: &str) -> Option<i64> {
    text.lines().find_map(|l| l.strip_prefix("btime ")).and_then(|v| v.trim().parse().ok())
}

fn hex_bytes(hex: &str, out: &mut [u8]) -> Option<()> {
    if hex.len() != out.len() * 2 {
        return None;
    }
    for (i, word) in out.chunks_mut(4).enumerate() {
        let v = u32::from_str_radix(hex.get(i * 8..i * 8 + 8)?, 16).ok()?;
        word.copy_from_slice(&v.to_ne_bytes());
    }
    Some(())
}

pub fn parse_net_tcp(text: &str, v6: bool) -> Vec<(u64, TcpRow)> {
    let mut out = Vec::new();
    for line in text.lines().skip(1) {
        let f: Vec<&str> = line.split_whitespace().collect();
        let (Some(local), Some(st), Some(inode)) = (f.get(1), f.get(3), f.get(9)) else { continue };
        let state = match *st {
            "0A" => TCP_LISTEN,
            "01" => TCP_ESTABLISHED,
            _ => continue,
        };
        let Ok(inode) = inode.parse::<u64>() else { continue };
        let Some((addr, port)) = local.split_once(':') else { continue };
        let Ok(port) = u16::from_str_radix(port, 16) else { continue };
        let address = if v6 {
            let mut b = [0u8; 16];
            if hex_bytes(addr, &mut b).is_none() {
                continue;
            }
            crate::tcp::ipv6_text(&b)
        } else {
            let mut b = [0u8; 4];
            if hex_bytes(addr, &mut b).is_none() {
                continue;
            }
            format!("{}.{}.{}.{}", b[0], b[1], b[2], b[3])
        };
        if inode != 0 {
            out.push((inode, TcpRow { pid: 0, state, address, port }));
        }
    }
    out
}

pub fn socket_inode(link: &str) -> Option<u64> {
    link.strip_prefix("socket:[")?.strip_suffix(']')?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_stat_with_spaces_and_parens_in_comm() {
        let s = "4242 (my (odd) proc) S 17 4242 4242 0 -1 4194304 100 0 0 0 250 50 0 0 20 0 1 0 98765 1000000 321 18446744073709551615";
        let st = parse_stat(s).unwrap();
        assert_eq!(st, Stat { comm: "my (odd) proc".into(), ppid: 17, utime: 250, stime: 50, start_ticks: 98765, rss_pages: 321 });
        assert!(parse_stat("12 (x) S").is_none());
        assert!(parse_stat("garbage").is_none());
    }

    #[test]
    fn parses_btime() {
        assert_eq!(parse_btime("cpu  1 2 3\nbtime 1791428482\nprocesses 9\n"), Some(1_791_428_482));
        assert_eq!(parse_btime("cpu 1\n"), None);
    }

    #[test]
    fn parses_proc_net_tcp_v4_and_v6() {
        let lo = u32::from_ne_bytes([127, 0, 0, 1]);
        let v4 = format!(
            "  sl  local_address rem_address   st tx_queue rx_queue tr tm->when retrnsmt   uid  timeout inode\n\
             0: {lo:08X}:1435 00000000:0000 0A 00000000:00000000 00:00000000 00000000  1000        0 55501 1 0\n\
             1: {lo:08X}:1435 {lo:08X}:EE48 01 00000000:00000000 00:00000000 00000000  1000        0 55502 1 0\n\
             2: {lo:08X}:0016 00000000:0000 06 00000000:00000000 00:00000000 00000000  1000        0 55503 1 0\n\
             3: {lo:08X}:0017 00000000:0000 0A 00000000:00000000 00:00000000 00000000  1000        0 0 1 0\n"
        );
        let rows = parse_net_tcp(&v4, false);
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0], (55501, TcpRow { pid: 0, state: TCP_LISTEN, address: "127.0.0.1".into(), port: 5173 }));
        assert_eq!(rows[1].1.state, TCP_ESTABLISHED);
        let w = |b: [u8; 4]| format!("{:08X}", u32::from_ne_bytes(b));
        let v6addr = format!("{}{}{}{}", w([0, 0, 0, 0]), w([0, 0, 0, 0]), w([0, 0, 0, 0]), w([0, 0, 0, 1]));
        let v6 = format!("header\n 0: {v6addr}:1F90 00000000000000000000000000000000:0000 0A 0 0 0 1000 0 777 1\n 1: XYZ:1F90 0:0 0A 0 0 0 0 0 778\n");
        assert_eq!(parse_net_tcp(&v6, true), vec![(777, TcpRow { pid: 0, state: TCP_LISTEN, address: "::1".into(), port: 8080 })]);
    }

    #[test]
    fn socket_links() {
        assert_eq!(socket_inode("socket:[55501]"), Some(55501));
        assert_eq!(socket_inode("pipe:[1]"), None);
        assert_eq!(socket_inode("/dev/null"), None);
    }
}
