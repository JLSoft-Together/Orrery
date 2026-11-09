use serde::Serialize;

pub const TCP_LISTEN: u32 = 2;
pub const TCP_ESTABLISHED: u32 = 5;

const ROW4_SIZE: usize = 24;
const ROW6_SIZE: usize = 56;

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct TcpRow {
    pub pid: u32,
    pub state: u32,
    pub address: String,
    pub port: u16,
}

fn u32_le(b: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}

fn u16_be(b: &[u8], off: usize) -> u16 {
    u16::from_be_bytes([b[off], b[off + 1]])
}

pub fn ipv6_text(b: &[u8; 16]) -> String {
    let groups: Vec<u16> = (0..8).map(|i| u16_be(b, i * 2)).collect();
    let (mut best_start, mut best_len) = (None, 0);
    let mut i = 0;
    while i < 8 {
        if groups[i] != 0 {
            i += 1;
            continue;
        }
        let mut j = i;
        while j < 8 && groups[j] == 0 {
            j += 1;
        }
        if j - i > best_len && j - i > 1 {
            best_start = Some(i);
            best_len = j - i;
        }
        i = j;
    }
    let hex: Vec<String> = groups.iter().map(|g| format!("{g:x}")).collect();
    match best_start {
        None => hex.join(":"),
        Some(s) => format!("{}::{}", hex[..s].join(":"), hex[s + best_len..].join(":")),
    }
}

fn rows(buf: &[u8], size: usize, row: impl Fn(&[u8]) -> TcpRow) -> Vec<TcpRow> {
    if buf.len() < 4 {
        return Vec::new();
    }
    let n = u32_le(buf, 0) as usize;
    let mut out = Vec::new();
    for i in 0..n {
        let off = 4 + i * size;
        if off + size > buf.len() {
            break;
        }
        out.push(row(&buf[off..off + size]));
    }
    out
}

pub fn parse_tcp_table4(buf: &[u8]) -> Vec<TcpRow> {
    rows(buf, ROW4_SIZE, |r| TcpRow { state: u32_le(r, 0), address: format!("{}.{}.{}.{}", r[4], r[5], r[6], r[7]), port: u16_be(r, 8), pid: u32_le(r, 20) })
}

pub fn parse_tcp_table6(buf: &[u8]) -> Vec<TcpRow> {
    rows(buf, ROW6_SIZE, |r| {
        let mut a = [0u8; 16];
        a.copy_from_slice(&r[..16]);
        TcpRow { address: ipv6_text(&a), port: u16_be(r, 20), state: u32_le(r, 48), pid: u32_le(r, 52) }
    })
}

fn split_host_port(text: &str) -> Option<(String, u16)> {
    let at = text.rfind(':').filter(|&i| i > 0)?;
    let port: u16 = text[at + 1..].parse().ok().filter(|&p| p > 0)?;
    let mut host = &text[..at];
    host = host.strip_prefix('[').unwrap_or(host);
    host = host.strip_suffix(']').unwrap_or(host);
    let host = host.split('%').next().unwrap_or("");
    Some((if host == "*" { "0.0.0.0".to_owned() } else { host.to_owned() }, port))
}

pub fn parse_lsof(stdout: &str) -> Vec<TcpRow> {
    let mut out = Vec::new();
    let mut pid = 0u32;
    let mut local: Option<(String, u16)> = None;
    for line in stdout.split('\n') {
        let line = line.strip_suffix('\r').unwrap_or(line);
        let mut chars = line.chars();
        let Some(tag) = chars.next() else { continue };
        let value = chars.as_str();
        match tag {
            'p' => {
                pid = value.parse().unwrap_or(0);
                local = None;
            }
            'f' => local = None,
            'n' => local = split_host_port(value.split("->").next().unwrap_or("")),
            'T' if value.starts_with("ST=") && pid != 0 => {
                let Some((address, port)) = local.take() else { continue };
                let state = match &value[3..] {
                    "LISTEN" => TCP_LISTEN,
                    "ESTABLISHED" => TCP_ESTABLISHED,
                    _ => continue,
                };
                out.push(TcpRow { pid, state, address, port });
            }
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row4(state: u32, addr: [u8; 4], port: u16, pid: u32) -> Vec<u8> {
        let mut r = state.to_le_bytes().to_vec();
        r.extend(addr);
        r.extend(port.to_be_bytes());
        r.extend([0, 0]);
        r.extend([0u8; 8]);
        r.extend(pid.to_le_bytes());
        r
    }

    #[test]
    fn parses_tcp_table4_like_the_collector() {
        let mut buf = 2u32.to_le_bytes().to_vec();
        buf.extend(row4(TCP_LISTEN, [127, 0, 0, 1], 5173, 4242));
        buf.extend(row4(TCP_ESTABLISHED, [0, 0, 0, 0], 443, 77));
        let rows = parse_tcp_table4(&buf);
        assert_eq!(rows[0], TcpRow { pid: 4242, state: 2, address: "127.0.0.1".into(), port: 5173 });
        assert_eq!(rows[1].port, 443);
        assert_eq!(parse_tcp_table4(&buf[..30]).len(), 1);
        assert!(parse_tcp_table4(&[0, 0]).is_empty());
    }

    #[test]
    fn parses_tcp_table6_and_compresses_zero_runs() {
        let mut r = vec![0u8; ROW6_SIZE];
        r[15] = 1;
        r[20..22].copy_from_slice(&8080u16.to_be_bytes());
        r[48..52].copy_from_slice(&2u32.to_le_bytes());
        r[52..56].copy_from_slice(&99u32.to_le_bytes());
        let mut buf = 1u32.to_le_bytes().to_vec();
        buf.extend(r);
        assert_eq!(parse_tcp_table6(&buf), vec![TcpRow { pid: 99, state: 2, address: "::1".into(), port: 8080 }]);
        assert!(parse_tcp_table6(&[0, 0, 0, 0]).is_empty());
        let mut a = [0u8; 16];
        assert_eq!(ipv6_text(&a), "::");
        a[0] = 0xfe;
        a[1] = 0x80;
        a[15] = 0x2a;
        assert_eq!(ipv6_text(&a), "fe80::2a");
        let one_zero: [u8; 16] = [0, 1, 0, 0, 0, 2, 0, 3, 0, 4, 0, 5, 0, 6, 0, 7];
        assert_eq!(ipv6_text(&one_zero), "1:0:2:3:4:5:6:7");
        let mapped: [u8; 16] = [0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0xff, 0xff, 127, 0, 0, 1];
        assert_eq!(ipv6_text(&mapped), "::ffff:7f00:1");
    }

    #[test]
    fn parses_lsof_fields_like_the_collector() {
        let out = "p501\nf12\nn127.0.0.1:5173\nTST=LISTEN\nf13\nn[::1]:5173\nTST=LISTEN\nf14\nn127.0.0.1:5173->127.0.0.1:61000\nTST=ESTABLISHED\n\
                   f15\nn*:9000\nTST=LISTEN\nf16\nn[fe80::1%lo0]:22\nTST=LISTEN\nf17\nn127.0.0.1:1\nTST=CLOSE_WAIT\np0\nf1\nn1.2.3.4:80\nTST=LISTEN\r\n";
        let rows = parse_lsof(out);
        assert_eq!(
            rows,
            vec![
                TcpRow { pid: 501, state: 2, address: "127.0.0.1".into(), port: 5173 },
                TcpRow { pid: 501, state: 2, address: "::1".into(), port: 5173 },
                TcpRow { pid: 501, state: 5, address: "127.0.0.1".into(), port: 5173 },
                TcpRow { pid: 501, state: 2, address: "0.0.0.0".into(), port: 9000 },
                TcpRow { pid: 501, state: 2, address: "fe80::1".into(), port: 22 },
            ]
        );
        assert!(parse_lsof("p1\nnbad\nTST=LISTEN\nn:80\nTST=LISTEN\n").is_empty());
    }
}
