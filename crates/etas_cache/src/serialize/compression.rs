use std::io::Cursor;

use crate::{CacheError, CacheResult, CompressionKind};

pub fn compress_payload(payload: &[u8], compression: CompressionKind) -> CacheResult<Vec<u8>> {
    match compression {
        CompressionKind::None => Ok(payload.to_vec()),
        CompressionKind::Zstd => {
            zstd::stream::encode_all(Cursor::new(payload), 0).map_err(Into::into)
        }
    }
}

pub fn decompress_payload(payload: &[u8], compression: CompressionKind) -> CacheResult<Vec<u8>> {
    match compression {
        CompressionKind::None => Ok(payload.to_vec()),
        CompressionKind::Zstd => zstd::stream::decode_all(Cursor::new(payload)).map_err(Into::into),
    }
    .map_err(|error: CacheError| error)
}
