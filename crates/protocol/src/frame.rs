use serde::{Serialize, de::DeserializeOwned};
use std::fmt;

pub const MAX_FRAME: usize = 4 << 20;

#[derive(Debug)]
pub enum FrameError {
    TooLarge(usize),
    Malformed(postcard::Error),
}

impl fmt::Display for FrameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooLarge(n) => write!(f, "frame body of {n} bytes exceeds {MAX_FRAME}"),
            Self::Malformed(e) => write!(f, "malformed frame: {e}"),
        }
    }
}

impl std::error::Error for FrameError {}

pub fn encode<T: Serialize>(msg: &T) -> Result<Vec<u8>, FrameError> {
    let mut out = postcard::to_extend(msg, vec![0; 4]).map_err(FrameError::Malformed)?;
    let len = out.len() - 4;
    if len > MAX_FRAME {
        return Err(FrameError::TooLarge(len));
    }
    out[..4].copy_from_slice(&(len as u32).to_be_bytes());
    Ok(out)
}

/// `Ok(None)` means `buf` does not yet hold a whole frame. The size cap is
/// checked on the header alone, before any body bytes arrive.
pub fn decode<T: DeserializeOwned>(buf: &[u8]) -> Result<Option<(T, usize)>, FrameError> {
    let Some(header) = buf.first_chunk::<4>() else {
        return Ok(None);
    };
    let len = u32::from_be_bytes(*header) as usize;
    if len > MAX_FRAME {
        return Err(FrameError::TooLarge(len));
    }
    let Some(body) = buf.get(4..4 + len) else {
        return Ok(None);
    };
    let msg = postcard::from_bytes(body).map_err(FrameError::Malformed)?;
    Ok(Some((msg, 4 + len)))
}
