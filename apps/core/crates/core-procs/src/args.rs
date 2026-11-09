fn nul_split(b: &[u8]) -> impl Iterator<Item = &[u8]> {
    b.split(|&c| c == 0)
}

pub fn parse_procargs2(b: &[u8]) -> Option<String> {
    let head: [u8; 4] = b.get(..4)?.try_into().ok()?;
    let argc = i32::from_ne_bytes(head);
    if argc <= 0 {
        return None;
    }
    let rest = &b[4..];
    let path_end = rest.iter().position(|&c| c == 0)?;
    let after = &rest[path_end..];
    let start = after.iter().position(|&c| c != 0)?;
    let args: Vec<String> = nul_split(&after[start..]).take(argc as usize).map(|a| String::from_utf8_lossy(a).into_owned()).collect();
    let joined = args.join(" ");
    (!joined.trim().is_empty()).then_some(joined)
}

pub fn join_cmdline(b: &[u8]) -> Option<String> {
    let b = b.strip_suffix(&[0]).unwrap_or(b);
    if b.is_empty() {
        return None;
    }
    let args: Vec<String> = nul_split(b).map(|a| String::from_utf8_lossy(a).into_owned()).collect();
    Some(args.join(" "))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn procargs(argc: i32, body: &[u8]) -> Vec<u8> {
        let mut b = argc.to_ne_bytes().to_vec();
        b.extend_from_slice(body);
        b
    }

    #[test]
    fn procargs2_skips_exec_path_and_padding_and_stops_at_argc() {
        let b = procargs(3, b"/usr/local/bin/node\0\0\0\0node\0/tmp/a b.js\0--x\0PATH=/bin\0HOME=/h\0");
        assert_eq!(parse_procargs2(&b).as_deref(), Some("node /tmp/a b.js --x"));
        assert_eq!(parse_procargs2(&procargs(1, b"/bin/zsh\0-zsh")).as_deref(), Some("-zsh"));
        assert!(parse_procargs2(&procargs(0, b"/bin/x\0\0x\0")).is_none());
        assert!(parse_procargs2(&procargs(2, b"/bin/x")).is_none());
        assert!(parse_procargs2(&procargs(2, b"/bin/x\0\0\0")).is_none());
        assert!(parse_procargs2(&[1, 0]).is_none());
    }

    #[test]
    fn proc_cmdline_joins_with_spaces() {
        assert_eq!(join_cmdline(b"node\0-e\0setInterval(()=>{}, 1000)\0").as_deref(), Some("node -e setInterval(()=>{}, 1000)"));
        assert_eq!(join_cmdline(b"bash").as_deref(), Some("bash"));
        assert!(join_cmdline(b"").is_none());
        assert!(join_cmdline(b"\0").is_none());
    }
}
