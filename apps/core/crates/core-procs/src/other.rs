use crate::tcp::TcpRow;
use crate::{Proc, Usage};

pub fn table() -> Option<Vec<Proc>> {
    None
}

pub fn parents() -> Option<Vec<(u32, u32)>> {
    None
}

pub fn usage(_pid: u32) -> Option<Usage> {
    None
}

pub fn cmdline(_pid: u32) -> Option<String> {
    None
}

pub fn ports() -> Option<Vec<TcpRow>> {
    None
}
