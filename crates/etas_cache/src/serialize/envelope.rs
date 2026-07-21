use crate::{
    ArtifactFingerprint, ArtifactKey, ArtifactKindKey, ArtifactUnitKey, CacheError, CacheNamespace,
    CacheResult, ContentHash,
};

pub const ENVELOPE_MAGIC: [u8; 8] = *b"APLCACHE";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PayloadCodec {
    Bincode2,
    Postcard,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompressionKind {
    None,
    Zstd,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactEnvelopeHeader {
    pub magic: [u8; 8],
    pub cache_schema_version: u32,
    pub compiler_version: String,
    pub key: ArtifactKey,
    pub fingerprint: ArtifactFingerprint,
    pub codec: PayloadCodec,
    pub compression: CompressionKind,
    pub uncompressed_len: u64,
    pub stored_len: u64,
    pub payload_hash: ContentHash,
}

impl ArtifactEnvelopeHeader {
    pub fn encode(&self) -> CacheResult<Vec<u8>> {
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&self.magic);
        push_u32(&mut bytes, self.cache_schema_version);
        push_string(&mut bytes, &self.compiler_version)?;
        push_string(&mut bytes, self.key.namespace.as_str())?;
        push_string(&mut bytes, self.key.kind.as_str())?;
        push_string(&mut bytes, self.key.unit.as_str())?;
        bytes.extend_from_slice(self.fingerprint.as_bytes());
        bytes.push(self.codec.to_byte());
        bytes.push(self.compression.to_byte());
        push_u64(&mut bytes, self.uncompressed_len);
        push_u64(&mut bytes, self.stored_len);
        bytes.extend_from_slice(self.payload_hash.as_bytes());
        Ok(bytes)
    }

    pub fn decode(bytes: &[u8]) -> CacheResult<Self> {
        let mut cursor = HeaderCursor::new(bytes);
        let magic = cursor.read_array::<8>()?;
        if magic != ENVELOPE_MAGIC {
            return Err(CacheError::InvalidEnvelope(
                "cache envelope magic does not match APLCACHE".to_owned(),
            ));
        }
        let cache_schema_version = cursor.read_u32()?;
        let compiler_version = cursor.read_string()?;
        let namespace = CacheNamespace::new(cursor.read_string()?);
        let kind = ArtifactKindKey::new(cursor.read_string()?);
        let unit = ArtifactUnitKey::new(cursor.read_string()?);
        let fingerprint = ArtifactFingerprint::new(cursor.read_array::<32>()?);
        let codec = PayloadCodec::from_byte(cursor.read_u8()?)?;
        let compression = CompressionKind::from_byte(cursor.read_u8()?)?;
        let uncompressed_len = cursor.read_u64()?;
        let stored_len = cursor.read_u64()?;
        let payload_hash = ContentHash::new(cursor.read_array::<32>()?);
        cursor.finish()?;
        Ok(Self {
            magic,
            cache_schema_version,
            compiler_version,
            key: ArtifactKey {
                namespace,
                kind,
                unit,
            },
            fingerprint,
            codec,
            compression,
            uncompressed_len,
            stored_len,
            payload_hash,
        })
    }
}

impl PayloadCodec {
    fn to_byte(self) -> u8 {
        match self {
            PayloadCodec::Bincode2 => 1,
            PayloadCodec::Postcard => 2,
        }
    }

    fn from_byte(value: u8) -> CacheResult<Self> {
        match value {
            1 => Ok(PayloadCodec::Bincode2),
            2 => Ok(PayloadCodec::Postcard),
            other => Err(CacheError::InvalidEnvelope(format!(
                "unknown payload codec tag {other}"
            ))),
        }
    }
}

impl CompressionKind {
    fn to_byte(self) -> u8 {
        match self {
            CompressionKind::None => 0,
            CompressionKind::Zstd => 1,
        }
    }

    fn from_byte(value: u8) -> CacheResult<Self> {
        match value {
            0 => Ok(CompressionKind::None),
            1 => Ok(CompressionKind::Zstd),
            other => Err(CacheError::InvalidEnvelope(format!(
                "unknown compression tag {other}"
            ))),
        }
    }
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_string(bytes: &mut Vec<u8>, value: &str) -> CacheResult<()> {
    let len = u32::try_from(value.len()).map_err(|_| {
        CacheError::InvalidEnvelope("string field is too large for cache header".to_owned())
    })?;
    push_u32(bytes, len);
    bytes.extend_from_slice(value.as_bytes());
    Ok(())
}

struct HeaderCursor<'a> {
    bytes: &'a [u8],
    offset: usize,
}

impl<'a> HeaderCursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, offset: 0 }
    }

    fn read_array<const N: usize>(&mut self) -> CacheResult<[u8; N]> {
        let end = self.offset.checked_add(N).ok_or_else(|| {
            CacheError::InvalidEnvelope("cache header offset overflow".to_owned())
        })?;
        let Some(slice) = self.bytes.get(self.offset..end) else {
            return Err(CacheError::InvalidEnvelope(
                "cache header ended unexpectedly".to_owned(),
            ));
        };
        self.offset = end;
        let mut array = [0u8; N];
        array.copy_from_slice(slice);
        Ok(array)
    }

    fn read_u8(&mut self) -> CacheResult<u8> {
        Ok(self.read_array::<1>()?[0])
    }

    fn read_u32(&mut self) -> CacheResult<u32> {
        Ok(u32::from_le_bytes(self.read_array::<4>()?))
    }

    fn read_u64(&mut self) -> CacheResult<u64> {
        Ok(u64::from_le_bytes(self.read_array::<8>()?))
    }

    fn read_string(&mut self) -> CacheResult<String> {
        let len = usize::try_from(self.read_u32()?).map_err(|_| {
            CacheError::InvalidEnvelope("string length cannot fit usize".to_owned())
        })?;
        let end = self.offset.checked_add(len).ok_or_else(|| {
            CacheError::InvalidEnvelope("cache header string offset overflow".to_owned())
        })?;
        let Some(slice) = self.bytes.get(self.offset..end) else {
            return Err(CacheError::InvalidEnvelope(
                "cache header string ended unexpectedly".to_owned(),
            ));
        };
        self.offset = end;
        String::from_utf8(slice.to_vec()).map_err(|error| {
            CacheError::InvalidEnvelope(format!("cache header string is not utf8: {error}"))
        })
    }

    fn finish(self) -> CacheResult<()> {
        if self.offset == self.bytes.len() {
            return Ok(());
        }
        Err(CacheError::InvalidEnvelope(
            "cache header has trailing bytes".to_owned(),
        ))
    }
}
