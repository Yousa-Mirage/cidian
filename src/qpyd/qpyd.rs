use std::collections::BTreeMap;
use std::path::Path;

use zlib_rs::{InflateConfig, ReturnCode, decompress_slice};

use crate::utils::{decode_utf16, read_file, read_u32_at, read_u64_at, slice_at, validate_count};
use crate::{Dictionary, Entry, Error, Format, Metadata, Result};

const MAGIC: &[u8; 8] = b"\x09\xa6\x1e\x7d\x01\x00\x00\x00";
const HEADER_LEN: usize = 0x48;
const FILETIME_OFFSET: usize = 0x18;
const VERSION_OFFSET: usize = 0x28;
const INFO_OFFSET_OFFSET: usize = 0x2c;
const INFO_SIZE_OFFSET: usize = 0x30;
const COMPRESSED_OFFSET_OFFSET: usize = 0x38;
const COMPRESSED_SIZE_OFFSET: usize = 0x3c;
const DECOMPRESSED_SIZE_OFFSET: usize = 0x40;
const ENTRY_COUNT_OFFSET: usize = 0x44;
const INDEX_RECORD_LEN: usize = 10;
const FORMAT: Format = Format::Qpyd;

struct Header {
    filetime_raw: u64,
    version: u32,
    info_offset: usize,
    info_size: usize,
    compressed_offset: usize,
    compressed_size: usize,
    decompressed_size: usize,
    entry_count: usize,
}

/// Parses a QPYD dictionary from memory.
///
/// Entries are returned in source index order. Apostrophes in each stored
/// pinyin code delimit the components returned in [`Entry::code`]. Text is not
/// otherwise normalized, sorted, or deduplicated.
///
/// # Examples
///
/// ```no_run
/// let bytes = std::fs::read("dictionary.qpyd")?;
/// let dictionary = cidian::qpyd::parse(&bytes)?;
///
/// for entry in dictionary.entries {
///     println!("{}\t{}", entry.word, entry.code.join(" "));
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn parse(data: &[u8]) -> Result<Dictionary> {
    let header = parse_header(data)?;
    // Each entry consumes one fixed-size index record in the decompressed data.
    // Reject impossible counts before parsing metadata or inflating the payload.
    validate_count(
        "entry index",
        header.entry_count as u64,
        header.decompressed_size / INDEX_RECORD_LEN,
        FORMAT,
    )?;
    let metadata = parse_metadata(data, &header)?;
    let decompressed = decompress_entries(data, &header)?;
    let entries = parse_entries(&decompressed, header.entry_count)?;

    Ok(Dictionary { metadata, entries })
}

/// Parses a QPYD dictionary from a file.
///
/// The result is identical to reading the file and passing its bytes to
/// [`parse`]. The file name does not alter dictionary metadata.
///
/// # Examples
///
/// ```no_run
/// let dictionary = cidian::qpyd::parse_file("dictionary.qpyd")?;
/// println!("{} entries", dictionary.entries.len());
/// # Ok::<(), cidian::Error>(())
/// ```
pub fn parse_file(path: impl AsRef<Path>) -> Result<Dictionary> {
    let data = read_file(path, FORMAT)?;
    parse(&data)
}

/// Reads the fixed QPYD header, validates its magic, and extracts its fields.
///
/// The version and raw file time are preserved for metadata consumers; this
/// function does not reject a version merely because it is unfamiliar.
fn parse_header(data: &[u8]) -> Result<Header> {
    let header = slice_at(data, 0, HEADER_LEN, FORMAT)?;
    if &header[..MAGIC.len()] != MAGIC {
        return Err(Error::InvalidMagic {
            format: FORMAT,
            found: header[..MAGIC.len()].to_vec(),
        });
    }

    Ok(Header {
        filetime_raw: read_u64_at(header, FILETIME_OFFSET, FORMAT)?,
        version: read_u32_at(header, VERSION_OFFSET, FORMAT)?,
        info_offset: read_u32_at(header, INFO_OFFSET_OFFSET, FORMAT)? as usize,
        info_size: read_u32_at(header, INFO_SIZE_OFFSET, FORMAT)? as usize,
        compressed_offset: read_u32_at(header, COMPRESSED_OFFSET_OFFSET, FORMAT)? as usize,
        compressed_size: read_u32_at(header, COMPRESSED_SIZE_OFFSET, FORMAT)? as usize,
        decompressed_size: read_u32_at(header, DECOMPRESSED_SIZE_OFFSET, FORMAT)? as usize,
        entry_count: read_u32_at(header, ENTRY_COUNT_OFFSET, FORMAT)? as usize,
    })
}

/// Decodes the UTF-16LE information section into the common metadata model.
fn parse_metadata(data: &[u8], header: &Header) -> Result<Metadata> {
    let bytes = slice_at(data, header.info_offset, header.info_size, FORMAT)?;
    let info = decode_utf16(bytes, header.info_offset, "dictionary information", FORMAT)?;
    let mut metadata = Metadata::default();

    for line in info.trim_end_matches('\0').lines() {
        let Some((key, value)) = line.split_once(": ") else {
            continue;
        };

        match key {
            "Name" => metadata.name = optional_text(value),
            "Type" => metadata.category = optional_text(value),
            "Intro" => metadata.description = optional_text(value),
            "FirstType" => insert_extra(&mut metadata.extra, "first_type", value),
            "Example" => insert_extra(&mut metadata.extra, "example", value),
            _ => insert_extra(&mut metadata.extra, key, value),
        }
    }

    metadata
        .extra
        .insert("version".to_owned(), header.version.to_string());
    metadata
        .extra
        .insert("filetime_raw".to_owned(), header.filetime_raw.to_string());

    Ok(metadata)
}

/// Decompresses the QPYD entry section and verifies its declared size.
fn decompress_entries(data: &[u8], header: &Header) -> Result<Vec<u8>> {
    let compressed = slice_at(
        data,
        header.compressed_offset,
        header.compressed_size,
        FORMAT,
    )?;

    let mut output = Vec::new();
    output
        .try_reserve_exact(header.decompressed_size)
        .map_err(|source| Error::AllocationFailed {
            format: FORMAT,
            field: "decompressed data",
            requested: header.decompressed_size,
            source,
        })?;
    output.resize(header.decompressed_size, 0);

    let (actual, status) = {
        let (decompressed, status) =
            decompress_slice(&mut output, compressed, InflateConfig::default());
        (decompressed.len(), status)
    };

    if status != ReturnCode::Ok {
        return Err(Error::InvalidCompression {
            format: FORMAT,
            offset: header.compressed_offset,
            status: status as i32,
        });
    }

    if actual != header.decompressed_size {
        return Err(Error::SizeMismatch {
            format: FORMAT,
            field: "decompressed data",
            expected: header.decompressed_size as u64,
            actual: actual as u64,
        });
    }

    Ok(output)
}

/// Parses the fixed-size entry index followed by offset-addressed payloads.
fn parse_entries(data: &[u8], entry_count: usize) -> Result<Vec<Entry>> {
    let index_size = entry_count * INDEX_RECORD_LEN;
    let index = &data[..index_size];

    let mut entries = Vec::new();
    entries
        .try_reserve_exact(entry_count)
        .map_err(|source| Error::AllocationFailed {
            format: FORMAT,
            field: "dictionary entries",
            requested: entry_count,
            source,
        })?;

    for record in index.chunks_exact(INDEX_RECORD_LEN) {
        let code_len = usize::from(record[0]);
        let word_len = usize::from(record[1]);
        let payload_offset =
            u32::from_le_bytes([record[6], record[7], record[8], record[9]]) as usize;

        if payload_offset < index_size || payload_offset > data.len() {
            return Err(Error::InvalidOffset {
                format: FORMAT,
                field: "entry payload",
                offset: payload_offset,
                minimum: index_size,
                maximum: data.len(),
            });
        }

        let payload = slice_at(data, payload_offset, code_len + word_len, FORMAT)?;
        let code =
            std::str::from_utf8(&payload[..code_len]).map_err(|source| Error::InvalidUtf8 {
                format: FORMAT,
                field: "dictionary code",
                offset: payload_offset,
                source,
            })?;
        let word_offset = payload_offset + code_len;
        let word = decode_utf16(&payload[code_len..], word_offset, "dictionary word", FORMAT)?;

        entries.push(Entry {
            word,
            code: code.split('\'').map(str::to_owned).collect(),
            weight: None,
        });
    }

    Ok(entries)
}

/// Converts an empty metadata value to `None` while preserving non-empty text.
fn optional_text(value: &str) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value.to_owned())
    }
}

/// Retains a source-specific metadata field in the common `extra` map.
fn insert_extra(extra: &mut BTreeMap<String, String>, key: &str, value: &str) {
    extra.insert(key.to_owned(), value.to_owned());
}
