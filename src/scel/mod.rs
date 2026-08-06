//! Parser for Sogou Cell Dictionary (`.scel`) files.
//!
//! The parser follows counts and byte lengths stored in the file. It does not
//! rely on historical shortcuts such as fixed word-table offsets, a terminal
//! `zuo` pinyin entry, or a fixed-size word extension.

mod scel;

pub use scel::{parse, parse_file};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Dictionary, Error, Format};
    use std::fs;

    const RECORD_COUNT_OFFSET: usize = 0x120;
    const TOTAL_WORDS_OFFSET: usize = 0x124;
    const NAME_RANGE: std::ops::Range<usize> = 0x130..0x338;
    const PINYIN_COUNT_OFFSET: usize = 0x1540;
    const PINYIN_TABLE_OFFSET: usize = 0x1544;

    #[test]
    fn accepts_ecs_and_non_contiguous_pinyin_indices() {
        let data = fixture(*b"ECS", 42, 42, &[0x34, 0x12, 9, 8], b"tail");
        let dictionary = parse_for_test(&data);

        assert_eq!(dictionary.entries.len(), 1);
        assert_eq!(dictionary.entries[0].word, "词");
        assert_eq!(dictionary.entries[0].code, ["ci"]);
        assert_eq!(dictionary.entries[0].weight, Some(0x1234));
    }

    #[test]
    fn rejects_unknown_pinyin_index() {
        let data = fixture(*b"DCS", 42, 99, &[1, 0], &[]);
        let error = match parse(&data) {
            Ok(_) => panic!("unknown pinyin index unexpectedly parsed"),
            Err(error) => error,
        };

        assert!(matches!(
            error,
            Error::InvalidCodeIndex {
                format: Format::Scel,
                index: 99,
                ..
            }
        ));
    }

    #[test]
    fn rejects_odd_word_length() {
        let mut data = fixture(*b"DCS", 42, 42, &[1, 0], &[]);
        let word_table = pinyin_table_end(&data);
        let word_length_offset = word_table + 4 + 2;
        data[word_length_offset..word_length_offset + 2].copy_from_slice(&1_u16.to_le_bytes());

        let error = match parse(&data) {
            Ok(_) => panic!("odd UTF-16 word length unexpectedly parsed"),
            Err(error) => error,
        };
        assert!(matches!(
            error,
            Error::OddUtf16ByteLength {
                field: "dictionary word",
                length: 1,
                ..
            }
        ));
    }

    #[test]
    fn rejects_invalid_utf16_word() {
        let mut data = fixture(*b"DCS", 42, 42, &[1, 0], &[]);
        let word_table = pinyin_table_end(&data);
        let word_offset = word_table + 4 + 2 + 2;
        data[word_offset..word_offset + 2].copy_from_slice(&0xd800_u16.to_le_bytes());

        let error = match parse(&data) {
            Ok(_) => panic!("invalid UTF-16 word unexpectedly parsed"),
            Err(error) => error,
        };
        match error {
            Error::InvalidUtf16 {
                field: "dictionary word",
                source,
                ..
            } => assert_eq!(source.unpaired_surrogate(), 0xd800),
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn rejects_word_count_mismatch() {
        let mut data = fixture(*b"DCS", 42, 42, &[1, 0], &[]);
        data[TOTAL_WORDS_OFFSET..TOTAL_WORDS_OFFSET + 4].copy_from_slice(&2_u32.to_le_bytes());

        let error = match parse(&data) {
            Ok(_) => panic!("word count mismatch unexpectedly parsed"),
            Err(error) => error,
        };
        assert!(matches!(
            error,
            Error::CountMismatch {
                format: Format::Scel,
                field: "word",
                expected: 2,
                actual: 1,
            }
        ));
    }

    #[test]
    fn reports_truncated_input() {
        let mut data = fixture(*b"DCS", 42, 42, &[1, 0], &[]);
        data.truncate(100);
        let error = match parse(&data) {
            Ok(_) => panic!("truncated input unexpectedly parsed"),
            Err(error) => error,
        };
        assert!(matches!(
            error,
            Error::UnexpectedEof {
                format: Format::Scel,
                ..
            }
        ));
    }

    #[test]
    fn parse_file_matches_parse_without_source_name()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut data = fixture(*b"DCS", 42, 42, &[1, 0], &[]);
        data[NAME_RANGE].fill(0);
        let expected = parse(&data)?;

        let path = std::env::temp_dir().join(format!("cidian-no-name-{}.scel", std::process::id()));
        fs::write(&path, &data)?;
        let parsed = parse_file(&path);
        let removed = fs::remove_file(&path);

        let dictionary = parsed?;
        removed?;
        assert_eq!(dictionary, expected);
        assert_eq!(dictionary.metadata.name, None);

        Ok(())
    }

    #[test]
    fn parse_file_preserves_source_name() -> std::result::Result<(), Box<dyn std::error::Error>> {
        let path =
            std::env::temp_dir().join(format!("cidian-different-name-{}.scel", std::process::id()));
        let data = fixture(*b"DCS", 42, 42, &[1, 0], &[]);
        let expected = parse(&data)?;
        fs::write(&path, &data)?;
        let parsed = parse_file(&path);
        let removed = fs::remove_file(&path);

        let dictionary = parsed?;
        removed?;
        assert_eq!(dictionary, expected);
        assert_eq!(dictionary.metadata.name.as_deref(), Some("测试词库"));

        Ok(())
    }

    #[test]
    fn parse_file_exposes_io_error_context() {
        let path = std::env::temp_dir()
            .join(format!("cidian-missing-directory-{}", std::process::id()))
            .join("dictionary.scel");

        let error = match parse_file(&path) {
            Ok(_) => panic!("missing dictionary unexpectedly parsed"),
            Err(error) => error,
        };

        match error {
            Error::Io {
                format: Format::Scel,
                path: error_path,
                source,
            } => {
                assert_eq!(error_path, path);
                assert_eq!(source.kind(), std::io::ErrorKind::NotFound);
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    fn parse_for_test(data: &[u8]) -> Dictionary {
        match parse(data) {
            Ok(dictionary) => dictionary,
            Err(error) => panic!("test fixture failed to parse: {error}"),
        }
    }

    fn fixture(
        variant: [u8; 3],
        pinyin_index: u16,
        referenced_index: u16,
        extension: &[u8],
        trailing: &[u8],
    ) -> Vec<u8> {
        let mut data = vec![0; PINYIN_TABLE_OFFSET];
        data[0..4].copy_from_slice(&[0x40, 0x15, 0, 0]);
        data[4..7].copy_from_slice(&variant);
        data[7..12].copy_from_slice(&[1, 1, 0, 0, 0]);
        data[RECORD_COUNT_OFFSET..RECORD_COUNT_OFFSET + 4].copy_from_slice(&1_u32.to_le_bytes());
        data[TOTAL_WORDS_OFFSET..TOTAL_WORDS_OFFSET + 4].copy_from_slice(&1_u32.to_le_bytes());
        data[PINYIN_COUNT_OFFSET..PINYIN_COUNT_OFFSET + 4].copy_from_slice(&1_u32.to_le_bytes());
        write_fixed_utf16(&mut data, NAME_RANGE.start, "测试词库");

        push_u16(&mut data, pinyin_index);
        push_utf16(&mut data, "ci");

        push_u16(&mut data, 1);
        push_u16(&mut data, 2);
        push_u16(&mut data, referenced_index);
        push_utf16(&mut data, "词");
        push_u16(&mut data, extension.len() as u16);
        data.extend_from_slice(extension);
        data.extend_from_slice(trailing);
        data
    }

    fn pinyin_table_end(data: &[u8]) -> usize {
        let byte_len =
            u16::from_le_bytes([data[PINYIN_TABLE_OFFSET + 2], data[PINYIN_TABLE_OFFSET + 3]])
                as usize;
        PINYIN_TABLE_OFFSET + 4 + byte_len
    }

    fn write_fixed_utf16(data: &mut [u8], offset: usize, value: &str) {
        for (index, unit) in value.encode_utf16().enumerate() {
            let start = offset + index * 2;
            data[start..start + 2].copy_from_slice(&unit.to_le_bytes());
        }
    }

    fn push_utf16(data: &mut Vec<u8>, value: &str) {
        let bytes = value.encode_utf16().count() * 2;
        push_u16(data, bytes as u16);
        for unit in value.encode_utf16() {
            push_u16(data, unit);
        }
    }

    fn push_u16(data: &mut Vec<u8>, value: u16) {
        data.extend_from_slice(&value.to_le_bytes());
    }
}
