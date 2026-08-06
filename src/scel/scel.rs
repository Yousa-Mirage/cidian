use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::Path;

use crate::{Dictionary, Entry, Error, Format, Metadata, Result};

const HEADER_LEN: usize = 12;
const RECORD_COUNT_OFFSET: usize = 0x120;
const TOTAL_WORDS_OFFSET: usize = 0x124;
const NAME_RANGE: std::ops::Range<usize> = 0x130..0x338;
const CATEGORY_RANGE: std::ops::Range<usize> = 0x338..0x540;
const DESCRIPTION_RANGE: std::ops::Range<usize> = 0x540..0x0d40;
const EXAMPLE_RANGE: std::ops::Range<usize> = 0x0d40..0x1540;
const PINYIN_COUNT_OFFSET: usize = 0x1540;
const PINYIN_TABLE_OFFSET: usize = 0x1544;
const FORMAT: Format = Format::Scel;

/// Parses a SCEL dictionary from memory.
///
/// Entries are returned in source order and are not normalized, sorted, or
/// deduplicated.
///
/// # Examples
///
/// ```no_run
/// let bytes = std::fs::read("dictionary.scel")?;
/// let dictionary = cidian::scel::parse(&bytes)?;
///
/// for entry in dictionary.entries {
///     println!("{}\t{}", entry.word, entry.code.join(" "));
/// }
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn parse(data: &[u8]) -> Result<Dictionary> {
    validate_header(data)?;

    let metadata = parse_metadata(data)?;

    let record_count = read_u32_at(data, RECORD_COUNT_OFFSET)?;
    let total_words = read_u32_at(data, TOTAL_WORDS_OFFSET)?;
    let pinyin_count = read_u32_at(data, PINYIN_COUNT_OFFSET)?;

    let mut reader = Reader::new(data, PINYIN_TABLE_OFFSET);
    let pinyin = parse_pinyin_table(&mut reader, pinyin_count)?;
    let entries = parse_word_table(&mut reader, record_count, total_words, &pinyin)?;

    // SCEL files may contain trailing sections such as DELTBL. The main table
    // is complete once record_count groups have been read, so trailing bytes
    // must never be interpreted as additional word groups.
    Ok(Dictionary { metadata, entries })
}

/// Parses a SCEL dictionary from a SCEL dictionary file.
///
/// Entries are returned in source order and are not normalized, sorted, or
/// deduplicated.
///
/// # Examples
///
/// ```no_run
/// let dictionary = cidian::scel::parse_file("dictionary.scel")?;
/// println!("{} entries", dictionary.entries.len());
/// # Ok::<(), cidian::Error>(())
/// ```
pub fn parse_file(path: impl AsRef<Path>) -> Result<Dictionary> {
    let path = path.as_ref();
    let data = fs::read(path).map_err(|source| Error::Io {
        format: Format::Scel,
        path: path.to_owned(),
        source,
    })?;
    parse(&data)
}

fn validate_header(data: &[u8]) -> Result<()> {
    // The first 12 bytes identify the SCEL container, its variant, and its
    // version. Validate them before reading any offsets from the header.
    let header = slice_at(data, 0, HEADER_LEN)?;

    let magic = [header[0], header[1], header[2], header[3]];
    if magic != [0x40, 0x15, 0x00, 0x00] {
        return Err(Error::InvalidMagic {
            format: FORMAT,
            found: magic.to_vec(),
        });
    }

    let variant = [header[4], header[5], header[6]];
    if variant != *b"DCS" && variant != *b"ECS" {
        return Err(Error::UnsupportedVariant {
            format: FORMAT,
            found: variant.to_vec(),
        });
    }

    let version = [header[7], header[8], header[9], header[10], header[11]];
    if version != [0x01, 0x01, 0x00, 0x00, 0x00] {
        return Err(Error::UnsupportedVersion {
            format: FORMAT,
            found: version.to_vec(),
        });
    }

    Ok(())
}

/// Parses the fixed-width metadata fields at the beginning of a SCEL file.
///
/// Empty fields become `None`. SCEL's example field has no common model field,
/// so it is retained in `Metadata::extra`.
fn parse_metadata(data: &[u8]) -> Result<Metadata> {
    let name = parse_fixed_utf16(data, NAME_RANGE, "dictionary name")?;
    let category = parse_fixed_utf16(data, CATEGORY_RANGE, "dictionary category")?;
    let description = parse_fixed_utf16(data, DESCRIPTION_RANGE, "dictionary description")?;
    let example = parse_fixed_utf16(data, EXAMPLE_RANGE, "dictionary example")?;

    let mut extra = BTreeMap::new();
    if let Some(example) = example {
        extra.insert("example".to_owned(), example);
    }

    Ok(Metadata {
        name,
        category,
        description,
        extra,
    })
}

/// Parses the variable-length pinyin table and indexes entries by their
/// stored SCEL identifier. The identifiers are not assumed to be contiguous.
fn parse_pinyin_table(reader: &mut Reader<'_>, pinyin_count: u32) -> Result<HashMap<u16, String>> {
    // Every pinyin record needs at least an index and a byte length.
    validate_count("pinyin table", pinyin_count, reader.remaining() / 4)?;

    let mut pinyin = HashMap::with_capacity(pinyin_count as usize);
    for _ in 0..pinyin_count {
        let index = reader.read_u16()?;
        let byte_len = reader.read_u16()? as usize;
        let text = reader.read_utf16(byte_len, "pinyin table entry")?;

        if pinyin.insert(index, text).is_some() {
            return Err(Error::DuplicateCodeIndex {
                format: FORMAT,
                index: u64::from(index),
            });
        }
    }

    Ok(pinyin)
}

/// Parses the main SCEL word table into the common [`Entry`] model.
///
/// Each group contains one code sequence followed by one or more words that
/// share that sequence. The declared group and total-word counts are both
/// validated while parsing.
fn parse_word_table(
    reader: &mut Reader<'_>,
    record_count: u32,
    total_words: u32,
    pinyin: &HashMap<u16, String>,
) -> Result<Vec<Entry>> {
    // Each group starts with word_count and pinyin_bytes_len.
    validate_count("word group", record_count, reader.remaining() / 4)?;

    let mut entries = Vec::new();

    for _ in 0..record_count {
        let word_count = reader.read_u16()?;
        let pinyin_byte_len = reader.read_u16()? as usize;
        ensure_even("pinyin indices", reader.position(), pinyin_byte_len)?;

        let indices_offset = reader.position();
        let indices = reader.read_bytes(pinyin_byte_len)?;
        let syllables = indices
            .chunks_exact(2)
            .enumerate()
            .map(|(index_position, pair)| {
                let index = u16::from_le_bytes([pair[0], pair[1]]);

                pinyin.get(&index).cloned().ok_or(Error::InvalidCodeIndex {
                    format: FORMAT,
                    index: u64::from(index),
                    offset: indices_offset + index_position * 2,
                })
            })
            .collect::<Result<Vec<_>>>()?;

        for _ in 0..word_count {
            let word_byte_len = reader.read_u16()? as usize;
            let word = reader.read_utf16(word_byte_len, "dictionary word")?;

            let extension_len = reader.read_u16()? as usize;
            let extension = reader.read_bytes(extension_len)?;
            // The first extension value is exposed as the optional source
            // weight. The remaining extension bytes are format-specific.
            let weight = extension
                .get(..2)
                .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]) as u32);

            entries.push(Entry {
                word,
                code: syllables.clone(),
                weight,
            });
        }
    }

    if entries.len() != total_words as usize {
        return Err(Error::CountMismatch {
            format: FORMAT,
            field: "word",
            expected: u64::from(total_words),
            actual: entries.len() as u64,
        });
    }

    Ok(entries)
}

/// Decodes a fixed-width, NUL-terminated UTF-16LE metadata field.
///
/// The returned string excludes the terminator and unused padding. A field
/// whose first code unit is NUL is treated as absent.
fn parse_fixed_utf16(
    data: &[u8],
    range: std::ops::Range<usize>,
    field: &'static str,
) -> Result<Option<String>> {
    let offset = range.start;
    let bytes = slice_at(data, offset, range.len())?;
    let used_len = bytes
        .chunks_exact(2)
        .position(|pair| pair == [0, 0])
        .map_or(bytes.len(), |index| index * 2);

    if used_len == 0 {
        return Ok(None);
    }

    decode_utf16(&bytes[..used_len], offset, field).map(Some)
}

/// Decodes a byte slice as strict little-endian UTF-16.
///
/// Unlike a lossy decoder, malformed surrogate sequences are returned as a
/// structured parsing error so callers can distinguish corrupted input.
fn decode_utf16(bytes: &[u8], offset: usize, field: &'static str) -> Result<String> {
    ensure_even(field, offset, bytes.len())?;

    std::char::decode_utf16(
        bytes
            .chunks_exact(2)
            .map(|pair| u16::from_le_bytes([pair[0], pair[1]])),
    )
    .collect::<std::result::Result<String, _>>()
    .map_err(|source| Error::InvalidUtf16 {
        format: FORMAT,
        field,
        offset,
        source,
    })
}

/// Ensures that a UTF-16LE field contains complete two-byte code units.
#[allow(clippy::manual_is_multiple_of)]
fn ensure_even(field: &'static str, offset: usize, length: usize) -> Result<()> {
    if length % 2 == 0 {
        Ok(())
    } else {
        Err(Error::OddUtf16ByteLength {
            format: FORMAT,
            field,
            offset,
            length,
        })
    }
}

/// Checks whether a declared record count can fit in the remaining input.
///
/// The caller supplies the minimum number of bytes required by one record;
/// detailed bounds checks still happen when individual records are read.
fn validate_count(field: &'static str, count: u32, maximum: usize) -> Result<()> {
    if u64::from(count) <= maximum as u64 {
        Ok(())
    } else {
        Err(Error::InvalidCount {
            format: FORMAT,
            field,
            count: u64::from(count),
            maximum: maximum as u64,
        })
    }
}

/// Reads a little-endian `u32` from an absolute byte offset.
fn read_u32_at(data: &[u8], offset: usize) -> Result<u32> {
    let bytes = slice_at(data, offset, 4)?;
    Ok(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

/// Returns a bounded slice or a structured end-of-input error.
fn slice_at(data: &[u8], offset: usize, length: usize) -> Result<&[u8]> {
    let available = data.len().saturating_sub(offset);
    let end = offset
        .checked_add(length)
        .filter(|&end| end <= data.len())
        .ok_or(Error::UnexpectedEof {
            format: FORMAT,
            offset,
            needed: length,
            available,
        })?;

    Ok(&data[offset..end])
}

struct Reader<'a> {
    data: &'a [u8],
    position: usize,
}

impl<'a> Reader<'a> {
    /// Creates a reader positioned at an absolute byte offset.
    fn new(data: &'a [u8], position: usize) -> Self {
        debug_assert!(position <= data.len());
        Self { data, position }
    }

    /// Returns the current absolute byte offset.
    fn position(&self) -> usize {
        self.position
    }

    /// Returns the number of unread bytes.
    fn remaining(&self) -> usize {
        self.data.len() - self.position
    }

    /// Reads a little-endian `u16` and advances the reader by two bytes.
    fn read_u16(&mut self) -> Result<u16> {
        let bytes = self.read_bytes(2)?;
        Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
    }

    /// Reads a length-delimited UTF-16LE field and advances the reader.
    fn read_utf16(&mut self, length: usize, field: &'static str) -> Result<String> {
        let offset = self.position;
        let bytes = self.read_bytes(length)?;
        decode_utf16(bytes, offset, field)
    }

    /// Reads exactly `length` bytes and advances the reader.
    fn read_bytes(&mut self, length: usize) -> Result<&'a [u8]> {
        let bytes = slice_at(self.data, self.position, length)?;
        self.position += length;
        Ok(bytes)
    }
}
