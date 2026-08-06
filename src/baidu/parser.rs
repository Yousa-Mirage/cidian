use std::collections::BTreeMap;
use std::ops::Range;
use std::path::Path;

use crate::utils::{Reader, decode_fixed_utf16, read_file, read_u32_at, slice_at, validate_count};
use crate::{Dictionary, Entry, Error, Format, Metadata, Result};

const MAGIC: &[u8; 8] = b"biptbdsw";
const SUPPORTED_VERSION: u32 = 1;
const HEADER_LEN: usize = 0x350;
const VERSION_OFFSET: usize = 0x08;
const REGULAR_OFFSET_OFFSET: usize = 0x40;
const REGULAR_SIZE_OFFSET: usize = 0x44;
const ENGLISH_OFFSET_OFFSET: usize = 0x50;
const ENGLISH_SIZE_OFFSET: usize = 0x54;
const MIXED_OFFSET_OFFSET: usize = 0x60;
const MIXED_SIZE_OFFSET: usize = 0x64;
const REGULAR_COUNT_OFFSET: usize = 0x70;
const ENGLISH_COUNT_OFFSET: usize = 0x74;
const MIXED_COUNT_OFFSET: usize = 0x78;
const NAME_RANGE: Range<usize> = 0x90..0xd0;
const AUTHOR_RANGE: Range<usize> = 0xd0..0x110;
const EXAMPLE_RANGE: Range<usize> = 0x110..0x150;
const DESCRIPTION_RANGE: Range<usize> = 0x150..0x350;
const REGULAR_MIN_RECORD_LEN: usize = 4;
const ENGLISH_MIN_RECORD_LEN: usize = 4;
const MIXED_MIN_RECORD_LEN: usize = 8;

const INITIALS: [&str; 24] = [
    "c", "d", "b", "f", "g", "h", "ch", "j", "k", "l", "m", "n", "", "p", "q", "r", "s", "t", "sh",
    "zh", "w", "x", "y", "z",
];

const FINALS: [&str; 33] = [
    "uang", "iang", "iong", "ang", "eng", "ian", "iao", "ing", "ong", "uai", "uan", "ai", "an",
    "ao", "ei", "en", "er", "ua", "ie", "in", "iu", "ou", "ia", "ue", "ui", "un", "uo", "a", "e",
    "i", "o", "u", "v",
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BaiduVariant {
    Bdict,
    Bcd,
}

impl BaiduVariant {
    fn format(self) -> Format {
        match self {
            Self::Bdict => Format::Bdict,
            Self::Bcd => Format::Bcd,
        }
    }
}

#[derive(Clone, Copy)]
struct Section {
    offset: usize,
    size: usize,
    count: usize,
}

struct Header {
    regular: Section,
    english: Section,
    mixed: Section,
}

pub(crate) fn parse(data: &[u8], variant: BaiduVariant) -> Result<Dictionary> {
    let format = variant.format();
    let header = parse_header(data, variant)?;
    let metadata = parse_metadata(data, format)?;
    let mut entries = Vec::new();

    parse_regular_entries(data, header.regular, format, &mut entries)?;
    parse_english_entries(data, header.english, format, &mut entries)?;
    parse_mixed_entries(data, header.mixed, format, &mut entries)?;

    Ok(Dictionary { metadata, entries })
}

pub(crate) fn parse_file(path: impl AsRef<Path>, variant: BaiduVariant) -> Result<Dictionary> {
    let data = read_file(path, variant.format())?;
    parse(&data, variant)
}

fn parse_header(data: &[u8], variant: BaiduVariant) -> Result<Header> {
    let format = variant.format();
    let header = slice_at(data, 0, HEADER_LEN, format)?;

    if &header[..MAGIC.len()] != MAGIC {
        return Err(Error::InvalidMagic {
            format,
            found: header[..MAGIC.len()].to_vec(),
        });
    }

    let version = read_u32_at(header, VERSION_OFFSET, format)?;
    if version != SUPPORTED_VERSION {
        return Err(Error::UnsupportedVersion {
            format,
            found: version.to_le_bytes().to_vec(),
        });
    }

    let regular_count = read_u32_at(header, REGULAR_COUNT_OFFSET, format)? as usize;
    let english_count = read_u32_at(header, ENGLISH_COUNT_OFFSET, format)? as usize;
    let mixed_count = read_u32_at(header, MIXED_COUNT_OFFSET, format)? as usize;

    let mut parsed = Header {
        regular: Section {
            offset: read_u32_at(header, REGULAR_OFFSET_OFFSET, format)? as usize,
            size: read_u32_at(header, REGULAR_SIZE_OFFSET, format)? as usize,
            count: regular_count,
        },
        english: Section {
            offset: read_u32_at(header, ENGLISH_OFFSET_OFFSET, format)? as usize,
            size: read_u32_at(header, ENGLISH_SIZE_OFFSET, format)? as usize,
            count: english_count,
        },
        mixed: Section {
            offset: read_u32_at(header, MIXED_OFFSET_OFFSET, format)? as usize,
            size: read_u32_at(header, MIXED_SIZE_OFFSET, format)? as usize,
            count: mixed_count,
        },
    };

    let has_declared_sections = parsed.regular.offset != 0
        || parsed.regular.size != 0
        || parsed.english.offset != 0
        || parsed.english.size != 0
        || parsed.mixed.offset != 0
        || parsed.mixed.size != 0;

    if variant == BaiduVariant::Bcd && !has_declared_sections {
        parsed.regular = Section {
            offset: HEADER_LEN,
            size: data.len() - HEADER_LEN,
            count: regular_count,
        };
    }

    Ok(parsed)
}

fn parse_metadata(data: &[u8], format: Format) -> Result<Metadata> {
    let name = decode_fixed_utf16(data, NAME_RANGE, "dictionary name", format)?;
    let author = decode_fixed_utf16(data, AUTHOR_RANGE, "dictionary author", format)?;
    let example = decode_fixed_utf16(data, EXAMPLE_RANGE, "dictionary example", format)?;
    let description =
        decode_fixed_utf16(data, DESCRIPTION_RANGE, "dictionary description", format)?;

    let mut extra = BTreeMap::new();
    if let Some(author) = author {
        extra.insert("author".to_owned(), author);
    }
    if let Some(example) = example {
        extra.insert("example".to_owned(), example);
    }

    Ok(Metadata {
        name,
        category: None,
        description,
        extra,
    })
}

fn section_reader<'data>(
    data: &'data [u8],
    section: Section,
    field: &'static str,
    minimum_record_len: usize,
    format: Format,
) -> Result<Reader<'data>> {
    validate_count(
        field,
        section.count as u64,
        section.size / minimum_record_len,
        format,
    )?;

    if section.size == 0 {
        return Ok(Reader::section(&[], section.offset, format));
    }

    if section.offset < HEADER_LEN || section.offset > data.len() {
        return Err(Error::InvalidOffset {
            format,
            field,
            offset: section.offset,
            minimum: HEADER_LEN,
            maximum: data.len(),
        });
    }

    let bytes = slice_at(data, section.offset, section.size, format)?;
    Ok(Reader::section(bytes, section.offset, format))
}

fn parse_regular_entries(
    data: &[u8],
    section: Section,
    format: Format,
    entries: &mut Vec<Entry>,
) -> Result<()> {
    let mut reader = section_reader(
        data,
        section,
        "regular entries",
        REGULAR_MIN_RECORD_LEN,
        format,
    )?;

    for _ in 0..section.count {
        let code_len = usize::from(reader.read_u16()?);
        let weight = reader.read_u16()?;
        validate_count(
            "regular entry code units",
            code_len as u64,
            reader.remaining() / 4,
            format,
        )?;

        let code_offset = reader.position();
        let encoded_code = reader.read_bytes(code_len * 2)?;
        let mut code = Vec::new();
        code.try_reserve_exact(code_len)
            .map_err(|source| Error::AllocationFailed {
                format,
                field: "entry code components",
                requested: code_len,
                source,
            })?;

        for (index, pair) in encoded_code.chunks_exact(2).enumerate() {
            let initial_index = pair[0];
            let final_index = pair[1];
            let pair_offset = code_offset + index * 2;

            if initial_index == 0xff {
                if !final_index.is_ascii() {
                    return Err(Error::InvalidAscii {
                        format,
                        field: "embedded Latin code",
                        offset: pair_offset + 1,
                        byte: final_index,
                    });
                }
                code.push(char::from(final_index).to_string());
                continue;
            }

            let initial = INITIALS.get(usize::from(initial_index)).copied().ok_or(
                Error::InvalidCodeComponent {
                    format,
                    field: "initial",
                    index: u64::from(initial_index),
                    offset: pair_offset,
                },
            )?;
            let final_part = FINALS.get(usize::from(final_index)).copied().ok_or(
                Error::InvalidCodeComponent {
                    format,
                    field: "final",
                    index: u64::from(final_index),
                    offset: pair_offset + 1,
                },
            )?;

            let mut syllable = String::with_capacity(initial.len() + final_part.len());
            syllable.push_str(initial);
            syllable.push_str(final_part);
            code.push(syllable);
        }

        let word = reader.read_utf16(code_len * 2, "dictionary word")?;
        entries.push(Entry {
            word,
            code,
            weight: Some(u32::from(weight)),
        });
    }

    reader.finish("regular entries")
}

fn parse_english_entries(
    data: &[u8],
    section: Section,
    format: Format,
    entries: &mut Vec<Entry>,
) -> Result<()> {
    let mut reader = section_reader(
        data,
        section,
        "English entries",
        ENGLISH_MIN_RECORD_LEN,
        format,
    )?;

    for _ in 0..section.count {
        let length = usize::from(reader.read_u16()?);
        let weight = reader.read_u16()?;
        let word = reader.read_ascii(length, "English dictionary word")?;

        entries.push(Entry {
            code: vec![word.clone()],
            word,
            weight: Some(u32::from(weight)),
        });
    }

    reader.finish("English entries")
}

fn parse_mixed_entries(
    data: &[u8],
    section: Section,
    format: Format,
    entries: &mut Vec<Entry>,
) -> Result<()> {
    let mut reader = section_reader(data, section, "mixed entries", MIXED_MIN_RECORD_LEN, format)?;

    for _ in 0..section.count {
        let header_offset = reader.position();
        let header = reader.read_bytes(MIXED_MIN_RECORD_LEN)?;
        let first = u16::from_le_bytes([header[0], header[1]]);
        let second = u16::from_le_bytes([header[2], header[3]]);
        let third = u16::from_le_bytes([header[4], header[5]]);
        let word_len = usize::from(u16::from_le_bytes([header[6], header[7]]));

        let (code_len, weight) = if first != 0 && third == 0 {
            (usize::from(first), Some(u32::from(second)))
        } else if first == 0 && second == 0 && third != 0 {
            (usize::from(third), None)
        } else {
            return Err(Error::InvalidRecordHeader {
                format,
                field: "mixed entry",
                offset: header_offset,
                found: header.to_vec(),
            });
        };

        let units = code_len + word_len;
        validate_count(
            "mixed entry UTF-16 units",
            units as u64,
            reader.remaining() / 2,
            format,
        )?;

        let code = reader.read_utf16(code_len * 2, "mixed entry code")?;
        let word = reader.read_utf16(word_len * 2, "dictionary word")?;
        entries.push(Entry {
            word,
            code: vec![code],
            weight,
        });
    }

    reader.finish("mixed entries")
}
