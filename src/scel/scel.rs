use std::collections::BTreeMap;
use std::path::Path;

use crate::utils::{
    Reader, decode_fixed_utf16, ensure_even, read_file, read_u32_at, slice_at, validate_count,
};
use crate::{Dictionary, Entry, Error, Format, Metadata, Result};

const HEADER_LEN: usize = 12;
const RECORD_COUNT_OFFSET: usize = 0x120;
const TOTAL_WORDS_OFFSET: usize = 0x124;
const NAME_RANGE: std::ops::Range<usize> = 0x130..0x338;
const CATEGORY_RANGE: std::ops::Range<usize> = 0x338..0x540;
const DESCRIPTION_RANGE: std::ops::Range<usize> = 0x540..0x0d40;
const EXAMPLE_RANGE: std::ops::Range<usize> = 0x0d40..0x1540;
const PINYIN_COUNT_OFFSET: usize = 0x1540;
const PINYIN_HEADER_LEN: usize = 4;
const PINYIN_TABLE_OFFSET: usize = 0x1544;
const ENGLISH_CODE_COUNT: u16 = 26;
const FORMAT: Format = Format::Scel;

type CodeTable = Vec<Option<String>>;

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

    let record_count = read_u32_at(data, RECORD_COUNT_OFFSET, FORMAT)?;
    let total_words = read_u32_at(data, TOTAL_WORDS_OFFSET, FORMAT)?;
    let pinyin_count = read_pinyin_count(data)?;

    let mut reader = Reader::at(data, PINYIN_TABLE_OFFSET, FORMAT);
    let code_table = parse_pinyin_table(&mut reader, pinyin_count)?;
    let entries = parse_word_table(
        &mut reader,
        record_count,
        total_words,
        pinyin_count,
        &code_table,
    )?;

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
    let data = read_file(path, FORMAT)?;
    parse(&data)
}

/// Validates the fixed SCEL header before any format-specific offsets are read.
///
/// The header contains the SCEL magic, either the DCS or ECS variant marker,
/// and a five-byte version marker. Unsupported values are reported with the
/// corresponding structured error.
fn validate_header(data: &[u8]) -> Result<()> {
    let header = slice_at(data, 0, HEADER_LEN, FORMAT)?;

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
    let name = decode_fixed_utf16(data, NAME_RANGE, "dictionary name", FORMAT)?;
    let category = decode_fixed_utf16(data, CATEGORY_RANGE, "dictionary category", FORMAT)?;
    let description =
        decode_fixed_utf16(data, DESCRIPTION_RANGE, "dictionary description", FORMAT)?;
    let example = decode_fixed_utf16(data, EXAMPLE_RANGE, "dictionary example", FORMAT)?;

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
fn parse_pinyin_table(reader: &mut Reader<'_>, pinyin_count: u16) -> Result<CodeTable> {
    // Every pinyin record needs at least an index and a byte length.
    validate_count(
        "pinyin table",
        u64::from(pinyin_count),
        reader.remaining() / 4,
        FORMAT,
    )?;

    let mut code_table = vec![None; usize::from(pinyin_count)];

    for _ in 0..pinyin_count {
        let index = reader.read_u16()?;
        let byte_len = reader.read_u16()? as usize;
        let text = reader.read_utf16(byte_len, "pinyin table entry")?;
        let table_index = usize::from(index);

        if table_index >= code_table.len() {
            code_table.resize_with(table_index + 1, || None);
        }

        if code_table[table_index].replace(text).is_some() {
            return Err(Error::DuplicateCodeIndex {
                format: FORMAT,
                index: u64::from(index),
            });
        }
    }

    Ok(code_table)
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
    pinyin_count: u16,
    code_table: &CodeTable,
) -> Result<Vec<Entry>> {
    // Each group starts with word_count and pinyin_bytes_len.
    validate_count(
        "word group",
        u64::from(record_count),
        reader.remaining() / 4,
        FORMAT,
    )?;

    let mut entries = Vec::new();

    for _ in 0..record_count {
        let word_count = reader.read_u16()?;
        let pinyin_byte_len = reader.read_u16()? as usize;
        ensure_even("pinyin indices", reader.position(), pinyin_byte_len, FORMAT)?;

        let indices_offset = reader.position();
        let indices = reader.read_bytes(pinyin_byte_len)?;
        let mut codes = indices
            .chunks_exact(2)
            .enumerate()
            .map(|(index_position, pair)| {
                let index = u16::from_le_bytes([pair[0], pair[1]]);
                resolve_code_index(
                    code_table,
                    pinyin_count,
                    index,
                    indices_offset + index_position * 2,
                )
            })
            .collect::<Result<Vec<_>>>()?;

        for word_index in 0..word_count {
            let word_byte_len = reader.read_u16()? as usize;
            let word = reader.read_utf16(word_byte_len, "dictionary word")?;

            let extension_len = reader.read_u16()? as usize;
            let extension = reader.read_bytes(extension_len)?;
            // The first extension value is exposed as the optional source
            // weight. The remaining extension bytes are format-specific.
            let weight = extension
                .get(..2)
                .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]) as u32);
            // All words in a group share the same code. Move it into the last
            // word instead of cloning data that would immediately be dropped.
            let code = if word_index + 1 == word_count {
                std::mem::take(&mut codes)
            } else {
                codes.clone()
            };

            entries.push(Entry { word, code, weight });
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

/// Resolves a word-table code index through the pinyin table or SCEL's
/// implicit lowercase English alphabet that immediately follows it.
fn resolve_code_index(
    code_table: &CodeTable,
    pinyin_count: u16,
    index: u16,
    offset: usize,
) -> Result<String> {
    if let Some(code) = code_table.get(usize::from(index)).and_then(Option::as_ref) {
        return Ok(code.clone());
    }

    // Some SCEL dictionaries omit Latin letters from the pinyin table and
    // encode a-z as the 26 indices immediately following the table count.
    // This follows SCEL's standard layout, where the declared count is also
    // the base index of the implicit alphabet. Explicit table entries still
    // take precedence so their stored identifiers are preserved.
    if pinyin_count != 0 {
        if let Some(english_offset) = index.checked_sub(pinyin_count) {
            if english_offset < ENGLISH_CODE_COUNT {
                return Ok(char::from(b'a' + english_offset as u8).to_string());
            }
        }
    }

    Err(Error::InvalidCodeIndex {
        format: FORMAT,
        index: u64::from(index),
        offset,
    })
}

/// Reads the pinyin-table count while validating its complete header.
///
/// The four-byte area at `0x1540` contains a little-endian `u16` count followed
/// by an undocumented `u16`. Only the count controls parsing, but validating
/// the complete area guarantees that the table reader can start at `0x1544`.
fn read_pinyin_count(data: &[u8]) -> Result<u16> {
    let bytes = slice_at(data, PINYIN_COUNT_OFFSET, PINYIN_HEADER_LEN, FORMAT)?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}
