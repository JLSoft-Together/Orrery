#![forbid(unsafe_code)]

mod error;
pub mod frame;
mod keys;
pub mod pair;
pub mod relay;
pub mod session;
mod transport;
mod wire;

#[cfg(test)]
mod vectors;

pub use error::{Error, Result};
pub use keys::{StaticKeypair, KEY_LEN};
pub use transport::{Transport, MAX_PLAINTEXT};

pub const PROTO: u16 = 1;
pub const MIN_PROTO: u16 = 1;
pub const PROLOGUE_PREFIX: &[u8] = b"orrery-remote/";

pub fn prologue(proto: u16) -> Vec<u8> {
    let mut out = Vec::with_capacity(PROLOGUE_PREFIX.len() + 2);
    out.extend_from_slice(PROLOGUE_PREFIX);
    out.extend_from_slice(&proto.to_be_bytes());
    out
}
