use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use prost::Message;

use crate::MetadataArtifactError;

pub const PACKAGE_METADATA_FILE: &str = ".etas/package.etasmeta";
pub const MAGIC: &[u8; 8] = b"ETASMETA";
pub const ARTIFACT_SCHEMA_VERSION: u32 = 5;
pub const COMPRESSION_ZSTD: u8 = 1;
const SECTION_TABLE_ENTRY_SIZE: usize = 2 + 1 + 8 + 8 + 8 + 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u16)]
pub enum MetadataSectionKind {
    Exports = 1,
    TypeContracts = 2,
    EffectContracts = 3,
    ActionContracts = 4,
    ToolContracts = 5,
    TraceSpecContracts = 6,
    StdContracts = 7,
    PackageGraph = 8,
    PublicSymbols = 9,
}

impl MetadataSectionKind {
    pub fn from_u16(value: u16) -> Result<Self, MetadataArtifactError> {
        match value {
            1 => Ok(Self::Exports),
            2 => Ok(Self::TypeContracts),
            3 => Ok(Self::EffectContracts),
            4 => Ok(Self::ActionContracts),
            5 => Ok(Self::ToolContracts),
            6 => Ok(Self::TraceSpecContracts),
            7 => Ok(Self::StdContracts),
            8 => Ok(Self::PackageGraph),
            9 => Ok(Self::PublicSymbols),
            _ => Err(MetadataArtifactError::invalid(
                PACKAGE_METADATA_FILE,
                format!("package metadata artifact has unknown section kind `{value}`"),
            )),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetadataArtifactHeader {
    pub artifact_schema_version: u32,
    pub compiler_version: String,
    pub package_id: String,
    pub package_version: String,
    pub source_payload_hash: String,
    pub manifest_hash: String,
    pub dependency_lock_hash: String,
    pub created_target: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetadataArtifactInfo {
    pub header: MetadataArtifactHeader,
    pub artifact_hash: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EncodedMetadataSection {
    pub kind: MetadataSectionKind,
    pub payload: Vec<u8>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodedMetadataArtifact {
    pub header: MetadataArtifactHeader,
    pub sections: BTreeMap<MetadataSectionKind, Vec<u8>>,
}

pub fn package_metadata_artifact_path(package_root: &Path) -> PathBuf {
    package_root.join(PACKAGE_METADATA_FILE)
}

pub fn section_from_message<M: Message>(
    kind: MetadataSectionKind,
    message: M,
) -> EncodedMetadataSection {
    EncodedMetadataSection {
        kind,
        payload: message.encode_to_vec(),
    }
}

pub fn encode_metadata_artifact(
    header: &MetadataArtifactHeader,
    mut sections: Vec<EncodedMetadataSection>,
) -> Result<Vec<u8>, MetadataArtifactError> {
    sections.sort_by_key(|section| section.kind);
    let mut header_bytes = Vec::new();
    push_u32(&mut header_bytes, header.artifact_schema_version);
    push_string(&mut header_bytes, &header.compiler_version)?;
    push_string(&mut header_bytes, &header.package_id)?;
    push_string(&mut header_bytes, &header.package_version)?;
    push_string(&mut header_bytes, &header.source_payload_hash)?;
    push_string(&mut header_bytes, &header.manifest_hash)?;
    push_string(&mut header_bytes, &header.dependency_lock_hash)?;
    push_string(&mut header_bytes, &header.created_target)?;

    let table_len = checked_mul(sections.len(), SECTION_TABLE_ENTRY_SIZE)?;
    let payload_start = MAGIC.len() + 4 + header_bytes.len() + 4 + table_len;
    let mut offset = payload_start as u64;
    let mut table = Vec::with_capacity(table_len);
    let mut payloads = Vec::new();
    for section in sections {
        let compressed = zstd::bulk::compress(&section.payload, 0).map_err(|source| {
            MetadataArtifactError::compression(
                PACKAGE_METADATA_FILE,
                format!("metadata section compression failed: {source}"),
            )
        })?;
        push_u16(&mut table, section.kind as u16);
        table.push(COMPRESSION_ZSTD);
        push_u64(&mut table, offset);
        push_u64(&mut table, compressed.len() as u64);
        push_u64(&mut table, section.payload.len() as u64);
        table.extend_from_slice(blake3::hash(&section.payload).as_bytes());
        offset = checked_add_u64(offset, compressed.len() as u64)?;
        payloads.push(compressed);
    }

    let mut artifact = Vec::new();
    artifact.extend_from_slice(MAGIC);
    push_u32(&mut artifact, header_bytes.len() as u32);
    artifact.extend_from_slice(&header_bytes);
    push_u32(&mut artifact, payloads.len() as u32);
    artifact.extend_from_slice(&table);
    for payload in payloads {
        artifact.extend_from_slice(&payload);
    }
    Ok(artifact)
}

pub fn decode_metadata_artifact(
    path: &Path,
    bytes: &[u8],
) -> Result<DecodedMetadataArtifact, MetadataArtifactError> {
    let mut cursor = Cursor::new(bytes);
    let magic = cursor.take(MAGIC.len())?;
    if magic != MAGIC {
        return Err(MetadataArtifactError::invalid(
            path,
            "package metadata artifact magic is invalid",
        ));
    }
    let header_len = cursor.read_u32()? as usize;
    let header = decode_header(cursor.take(header_len)?)?;
    let section_count = cursor.read_u32()? as usize;
    let mut entries = Vec::new();
    for _ in 0..section_count {
        entries.push(SectionEntry {
            kind: MetadataSectionKind::from_u16(cursor.read_u16()?)?,
            compression: cursor.read_u8()?,
            offset: cursor.read_u64()?,
            compressed_len: cursor.read_u64()?,
            uncompressed_len: cursor.read_u64()?,
            uncompressed_hash: cursor.take_array::<32>()?,
        });
    }
    validate_section_layout(path, bytes.len(), cursor.position, &entries)?;
    let mut sections = BTreeMap::new();
    for entry in entries {
        let kind = entry.kind;
        let section = decode_section(path, bytes, &entry)?;
        sections.insert(kind, section);
    }
    Ok(DecodedMetadataArtifact { header, sections })
}

pub fn write_metadata_artifact_file(
    path: &Path,
    bytes: &[u8],
) -> Result<(), MetadataArtifactError> {
    let parent = path.parent().ok_or_else(|| MetadataArtifactError::Io {
        path: path.to_path_buf(),
        message: "metadata artifact path has no parent".to_owned(),
    })?;
    fs::create_dir_all(parent).map_err(|source| MetadataArtifactError::Io {
        path: parent.to_path_buf(),
        message: source.to_string(),
    })?;

    let temp = unique_temp_path(parent, "package-etasmeta-write");
    let mut file = File::create(&temp).map_err(|source| MetadataArtifactError::Io {
        path: temp.clone(),
        message: source.to_string(),
    })?;
    file.write_all(bytes)
        .map_err(|source| MetadataArtifactError::Io {
            path: temp.clone(),
            message: source.to_string(),
        })?;
    file.sync_all()
        .map_err(|source| MetadataArtifactError::Io {
            path: temp.clone(),
            message: source.to_string(),
        })?;
    drop(file);
    fs::rename(&temp, path).map_err(|source| MetadataArtifactError::Io {
        path: path.to_path_buf(),
        message: source.to_string(),
    })?;
    Ok(())
}

pub fn validate_artifact_schema(
    header: &MetadataArtifactHeader,
) -> Result<(), MetadataArtifactError> {
    if header.artifact_schema_version != ARTIFACT_SCHEMA_VERSION {
        return Err(MetadataArtifactError::invalid(
            PACKAGE_METADATA_FILE,
            format!(
                "package metadata artifact schema version `{}` is not supported; expected `{ARTIFACT_SCHEMA_VERSION}`",
                header.artifact_schema_version
            ),
        ));
    }
    Ok(())
}

pub fn blake3_hash(bytes: &[u8]) -> String {
    format!("blake3:{}", blake3::hash(bytes).to_hex())
}

pub fn source_payload_checksum(
    package_root: &Path,
    source_root: &Path,
) -> Result<String, MetadataArtifactError> {
    let mut files = Vec::new();
    collect_files(source_root, &mut files)?;
    files.sort();
    hash_files(package_root, &files)
}

pub fn file_checksum(path: &Path) -> Result<String, MetadataArtifactError> {
    if !path.exists() {
        return Ok("blake3:missing".to_owned());
    }
    let bytes = fs::read(path).map_err(|source| MetadataArtifactError::Io {
        path: path.to_path_buf(),
        message: source.to_string(),
    })?;
    Ok(blake3_hash(&bytes))
}

pub fn optional_file_checksum(path: &Path) -> Result<String, MetadataArtifactError> {
    if path.exists() {
        file_checksum(path)
    } else {
        Ok("blake3:missing".to_owned())
    }
}

fn decode_header(bytes: &[u8]) -> Result<MetadataArtifactHeader, MetadataArtifactError> {
    let mut cursor = Cursor::new(bytes);
    Ok(MetadataArtifactHeader {
        artifact_schema_version: cursor.read_u32()?,
        compiler_version: cursor.read_string()?,
        package_id: cursor.read_string()?,
        package_version: cursor.read_string()?,
        source_payload_hash: cursor.read_string()?,
        manifest_hash: cursor.read_string()?,
        dependency_lock_hash: cursor.read_string()?,
        created_target: cursor.read_string()?,
    })
}

fn decode_section(
    path: &Path,
    bytes: &[u8],
    entry: &SectionEntry,
) -> Result<Vec<u8>, MetadataArtifactError> {
    if entry.compression != COMPRESSION_ZSTD {
        return Err(MetadataArtifactError::invalid(
            path,
            format!(
                "metadata section {:?} uses unsupported compression {}",
                entry.kind, entry.compression
            ),
        ));
    }
    let start = usize::try_from(entry.offset).map_err(|_| {
        MetadataArtifactError::invalid(path, "metadata section offset does not fit usize")
    })?;
    let compressed_len = usize::try_from(entry.compressed_len).map_err(|_| {
        MetadataArtifactError::invalid(path, "metadata section length does not fit usize")
    })?;
    let end = start
        .checked_add(compressed_len)
        .ok_or_else(|| MetadataArtifactError::invalid(path, "metadata section bounds overflow"))?;
    let compressed = bytes.get(start..end).ok_or_else(|| {
        MetadataArtifactError::invalid(path, "metadata section bounds are outside artifact")
    })?;
    let uncompressed_len = usize::try_from(entry.uncompressed_len).map_err(|_| {
        MetadataArtifactError::invalid(
            path,
            "metadata section uncompressed length does not fit usize",
        )
    })?;
    let section = zstd::bulk::decompress(compressed, uncompressed_len).map_err(|source| {
        MetadataArtifactError::compression(
            path,
            format!("metadata section decompression failed: {source}"),
        )
    })?;
    if blake3::hash(&section).as_bytes() != &entry.uncompressed_hash {
        return Err(MetadataArtifactError::invalid(
            path,
            format!("metadata section {:?} hash mismatch", entry.kind),
        ));
    }
    Ok(section)
}

fn validate_section_layout(
    path: &Path,
    artifact_len: usize,
    payload_start: usize,
    entries: &[SectionEntry],
) -> Result<(), MetadataArtifactError> {
    let mut kinds = Vec::new();
    let mut ranges = Vec::new();
    for entry in entries {
        if kinds.contains(&entry.kind) {
            return Err(MetadataArtifactError::invalid(
                path,
                format!(
                    "metadata artifact contains duplicate {:?} section",
                    entry.kind
                ),
            ));
        }
        kinds.push(entry.kind);

        let start = usize::try_from(entry.offset).map_err(|_| {
            MetadataArtifactError::invalid(path, "metadata section offset does not fit usize")
        })?;
        let compressed_len = usize::try_from(entry.compressed_len).map_err(|_| {
            MetadataArtifactError::invalid(path, "metadata section length does not fit usize")
        })?;
        let end = start.checked_add(compressed_len).ok_or_else(|| {
            MetadataArtifactError::invalid(path, "metadata section bounds overflow")
        })?;
        ranges.push((start, end, entry.kind));
    }

    ranges.sort_by_key(|(start, _, _)| *start);
    let mut expected_start = payload_start;
    for (start, end, kind) in ranges {
        if start != expected_start {
            return Err(MetadataArtifactError::invalid(
                path,
                format!(
                    "metadata section {:?} starts at byte {start}, expected {expected_start}",
                    kind
                ),
            ));
        }
        expected_start = end;
    }
    if expected_start != artifact_len {
        let trailing = artifact_len - expected_start;
        return Err(MetadataArtifactError::invalid(
            path,
            format!(
                "metadata artifact has {trailing} undeclared trailing byte{}",
                if trailing == 1 { "" } else { "s" }
            ),
        ));
    }
    Ok(())
}

fn collect_files(root: &Path, files: &mut Vec<PathBuf>) -> Result<(), MetadataArtifactError> {
    if !root.exists() {
        return Ok(());
    }
    let entries = fs::read_dir(root).map_err(|source| MetadataArtifactError::Io {
        path: root.to_path_buf(),
        message: source.to_string(),
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| MetadataArtifactError::Io {
            path: root.to_path_buf(),
            message: source.to_string(),
        })?;
        let path = entry.path();
        let metadata = entry
            .metadata()
            .map_err(|source| MetadataArtifactError::Io {
                path: path.clone(),
                message: source.to_string(),
            })?;
        if metadata.is_dir() {
            collect_files(&path, files)?;
        } else if metadata.is_file() {
            files.push(path);
        }
    }
    Ok(())
}

fn hash_files(package_root: &Path, files: &[PathBuf]) -> Result<String, MetadataArtifactError> {
    let mut hasher = blake3::Hasher::new();
    for file in files {
        let relative = file.strip_prefix(package_root).unwrap_or(file);
        hasher.update(relative.to_string_lossy().as_bytes());
        hasher.update(&[0]);
        let bytes = fs::read(file).map_err(|source| MetadataArtifactError::Io {
            path: file.clone(),
            message: source.to_string(),
        })?;
        hasher.update(&bytes);
        hasher.update(&[0]);
    }
    Ok(format!("blake3:{}", hasher.finalize().to_hex()))
}

fn unique_temp_path(parent: &Path, prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    parent.join(format!("{prefix}-{nanos}.tmp"))
}

fn push_u16(bytes: &mut Vec<u8>, value: u16) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_u32(bytes: &mut Vec<u8>, value: u32) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_u64(bytes: &mut Vec<u8>, value: u64) {
    bytes.extend_from_slice(&value.to_le_bytes());
}

fn push_string(bytes: &mut Vec<u8>, value: &str) -> Result<(), MetadataArtifactError> {
    let len = u32::try_from(value.len()).map_err(|_| MetadataArtifactError::HeaderStringTooLong)?;
    push_u32(bytes, len);
    bytes.extend_from_slice(value.as_bytes());
    Ok(())
}

fn checked_mul(left: usize, right: usize) -> Result<usize, MetadataArtifactError> {
    left.checked_mul(right)
        .ok_or(MetadataArtifactError::SizeOverflow)
}

fn checked_add_u64(left: u64, right: u64) -> Result<u64, MetadataArtifactError> {
    left.checked_add(right)
        .ok_or(MetadataArtifactError::SizeOverflow)
}

struct SectionEntry {
    kind: MetadataSectionKind,
    compression: u8,
    offset: u64,
    compressed_len: u64,
    uncompressed_len: u64,
    uncompressed_hash: [u8; 32],
}

struct Cursor<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn take(&mut self, len: usize) -> Result<&'a [u8], MetadataArtifactError> {
        let end = self.position.checked_add(len).ok_or_else(|| {
            MetadataArtifactError::invalid(
                PACKAGE_METADATA_FILE,
                "metadata artifact cursor overflow",
            )
        })?;
        let slice = self.bytes.get(self.position..end).ok_or_else(|| {
            MetadataArtifactError::invalid(
                PACKAGE_METADATA_FILE,
                "metadata artifact ended unexpectedly",
            )
        })?;
        self.position = end;
        Ok(slice)
    }

    fn take_array<const N: usize>(&mut self) -> Result<[u8; N], MetadataArtifactError> {
        let mut array = [0; N];
        array.copy_from_slice(self.take(N)?);
        Ok(array)
    }

    fn read_u8(&mut self) -> Result<u8, MetadataArtifactError> {
        Ok(self.take(1)?[0])
    }

    fn read_u16(&mut self) -> Result<u16, MetadataArtifactError> {
        Ok(u16::from_le_bytes(self.take_array()?))
    }

    fn read_u32(&mut self) -> Result<u32, MetadataArtifactError> {
        Ok(u32::from_le_bytes(self.take_array()?))
    }

    fn read_u64(&mut self) -> Result<u64, MetadataArtifactError> {
        Ok(u64::from_le_bytes(self.take_array()?))
    }

    fn read_string(&mut self) -> Result<String, MetadataArtifactError> {
        let len = self.read_u32()? as usize;
        let bytes = self.take(len)?;
        String::from_utf8(bytes.to_vec()).map_err(|source| {
            MetadataArtifactError::invalid(
                PACKAGE_METADATA_FILE,
                format!("metadata header string is not UTF-8: {source}"),
            )
        })
    }
}
