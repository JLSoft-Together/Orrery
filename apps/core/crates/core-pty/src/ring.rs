pub const RING_BYTES: usize = 256 * 1024;

pub struct Ring {
    buf: Vec<u8>,
    head: usize,
    cap: usize,
    dropped: bool,
}

impl Default for Ring {
    fn default() -> Ring {
        Ring::new(RING_BYTES)
    }
}

impl Ring {
    pub fn new(cap: usize) -> Ring {
        Ring { buf: Vec::new(), head: 0, cap: cap.max(1), dropped: false }
    }

    pub fn len(&self) -> usize {
        self.buf.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buf.is_empty()
    }

    pub fn push(&mut self, data: &[u8]) {
        if data.is_empty() {
            return;
        }
        if data.len() >= self.cap {
            self.dropped |= data.len() > self.cap || !self.buf.is_empty();
            self.buf.clear();
            self.buf.extend_from_slice(&data[data.len() - self.cap..]);
            self.head = 0;
            return;
        }
        let mut rest = data;
        if self.buf.len() < self.cap {
            let take = (self.cap - self.buf.len()).min(rest.len());
            self.buf.extend_from_slice(&rest[..take]);
            rest = &rest[take..];
        }
        if rest.is_empty() {
            return;
        }
        self.dropped = true;
        let first = (self.cap - self.head).min(rest.len());
        self.buf[self.head..self.head + first].copy_from_slice(&rest[..first]);
        let second = rest.len() - first;
        self.buf[..second].copy_from_slice(&rest[first..]);
        self.head = (self.head + rest.len()) % self.cap;
    }

    pub fn snapshot(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.buf.len());
        out.extend_from_slice(&self.buf[self.head..]);
        out.extend_from_slice(&self.buf[..self.head]);
        if self.dropped {
            let skip = out.iter().take(3).take_while(|b| (**b & 0xC0) == 0x80).count();
            out.drain(..skip);
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn feed(ring: &mut Ring, all: &mut Vec<u8>, data: &[u8]) {
        ring.push(data);
        all.extend_from_slice(data);
    }

    #[test]
    fn keeps_everything_until_full() {
        let mut r = Ring::new(8);
        r.push(b"abc");
        r.push(b"de");
        assert_eq!(r.snapshot(), b"abcde");
        r.push(b"fgh");
        assert_eq!(r.snapshot(), b"abcdefgh");
        assert_eq!(r.len(), 8);
    }

    #[test]
    fn wraps_and_keeps_the_exact_tail() {
        let mut r = Ring::new(8);
        let mut all = Vec::new();
        for chunk in [&b"0123"[..], b"4567", b"89", b"abcde", b"f", b"ghijklmnopq", b"rs"] {
            feed(&mut r, &mut all, chunk);
            let tail = &all[all.len().saturating_sub(8)..];
            assert_eq!(r.snapshot(), tail);
        }
    }

    #[test]
    fn many_sizes_match_a_naive_tail() {
        let cap = 37;
        let mut r = Ring::new(cap);
        let mut all = Vec::new();
        let mut n = 1usize;
        for i in 0..400u32 {
            n = (n * 7 + 3) % 53;
            let chunk: Vec<u8> = (0..n).map(|k| b'a' + ((i as usize + k) % 26) as u8).collect();
            feed(&mut r, &mut all, &chunk);
            assert_eq!(r.snapshot(), &all[all.len().saturating_sub(cap)..]);
        }
    }

    #[test]
    fn replay_starts_on_a_utf8_boundary_after_wrap() {
        for cut in 0..12 {
            let mut r = Ring::new(16);
            let mut all = Vec::new();
            feed(&mut r, &mut all, &vec![b'x'; cut]);
            for _ in 0..6 {
                feed(&mut r, &mut all, "é中😀".as_bytes());
            }
            let snap = r.snapshot();
            assert!(snap.is_empty() || (snap[0] & 0xC0) != 0x80, "cut={cut} snap={snap:?}");
            assert!(all.ends_with(&snap));
            assert!(snap.len() >= 13);
            assert!(std::str::from_utf8(&snap).is_ok(), "cut={cut}");
        }
    }

    #[test]
    fn no_skip_before_anything_was_dropped() {
        let mut r = Ring::new(16);
        r.push(&[0x82, 0xAC, b'a']);
        assert_eq!(r.snapshot(), vec![0x82, 0xAC, b'a']);
    }

    #[test]
    fn skips_at_most_three_continuation_bytes() {
        let mut r = Ring::new(4);
        r.push(&[b'z', 0x80, 0x80, 0x80, 0x80]);
        assert_eq!(r.snapshot(), vec![0x80]);
    }

    #[test]
    fn oversize_push_keeps_its_tail() {
        let mut r = Ring::new(4);
        r.push(b"ab");
        r.push(b"0123456789");
        assert_eq!(r.snapshot(), b"6789");
        r.push(b"x");
        assert_eq!(r.snapshot(), b"789x");
        let mut exact = Ring::new(4);
        exact.push(b"abcd");
        assert_eq!(exact.snapshot(), b"abcd");
    }

    #[test]
    fn default_is_256_kib() {
        let mut r = Ring::default();
        let all: Vec<u8> = (0..(RING_BYTES + 1000)).map(|i| (i % 251) as u8 & 0x7F).collect();
        r.push(&all[..1000]);
        r.push(&all[1000..]);
        let snap = r.snapshot();
        assert_eq!(snap.len(), RING_BYTES);
        assert_eq!(snap, &all[1000..]);
    }
}
