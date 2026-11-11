use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    Noise,
    KeyMismatch,
    Unauthorized,
    ProtoTooOld { min: u16 },
    ProtoTooNew { max: u16 },
    BadMessage,
    BadPayload,
    BadKey,
    BadQr,
    BadToken,
    Expired,
    UnknownKind(u8),
    BadFrame,
    TooLarge,
    Closed,
    Rng,
}

pub type Result<T> = std::result::Result<T, Error>;

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Noise => f.write_str("noise handshake or decryption failed"),
            Error::KeyMismatch => f.write_str("desktop key does not match the pairing code"),
            Error::Unauthorized => f.write_str("device is not paired"),
            Error::ProtoTooOld { min } => write!(f, "protocol too old, minimum {min}"),
            Error::ProtoTooNew { max } => write!(f, "protocol too new, maximum {max}"),
            Error::BadMessage => f.write_str("malformed message"),
            Error::BadPayload => f.write_str("malformed handshake payload"),
            Error::BadKey => f.write_str("malformed key"),
            Error::BadQr => f.write_str("malformed pairing code"),
            Error::BadToken => f.write_str("invalid relay token"),
            Error::Expired => f.write_str("expired"),
            Error::UnknownKind(kind) => write!(f, "unknown frame kind {kind}"),
            Error::BadFrame => f.write_str("malformed frame"),
            Error::TooLarge => f.write_str("message too large"),
            Error::Closed => f.write_str("transport closed after a failure"),
            Error::Rng => f.write_str("random generator failed"),
        }
    }
}

impl std::error::Error for Error {}

impl From<snow::Error> for Error {
    fn from(_: snow::Error) -> Self {
        Error::Noise
    }
}
