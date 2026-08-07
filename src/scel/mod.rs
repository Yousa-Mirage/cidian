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
    type TestResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

    #[test]
    fn rejects_invalid_magic() {
        let mut data = fixture(&[]);
        data[0..4].copy_from_slice(&[0, 0, 0, 0]);

        let error = must_fail(&data);

        match error {
            Error::InvalidMagic {
                format: Format::Scel,
                found,
            } => assert_eq!(found, vec![0, 0, 0, 0]),
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn rejects_unknown_variant() {
        let mut data = fixture(&[]);
        data[4..7].copy_from_slice(b"XYZ");

        let error = must_fail(&data);

        match error {
            Error::UnsupportedVariant {
                format: Format::Scel,
                found,
            } => assert_eq!(found, b"XYZ"),
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn rejects_unsupported_version() {
        let mut data = fixture(&[]);
        data[7..12].copy_from_slice(&[2, 1, 0, 0, 0]);

        let error = must_fail(&data);

        match error {
            Error::UnsupportedVersion {
                format: Format::Scel,
                found,
            } => assert_eq!(found, vec![2, 1, 0, 0, 0]),
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn accepts_ecs_variant() {
        let mut data = fixture(&[]);
        data[4..7].copy_from_slice(b"ECS");
        let dictionary = must_parse(&data);

        assert_eq!(dictionary.entries.len(), 1);
    }

    #[test]
    fn ignores_unknown_pinyin_count_field() {
        let mut data = fixture(&[]);
        set_u16(&mut data, PINYIN_COUNT_OFFSET + 2, 1);

        let dictionary = must_parse(&data);

        assert_eq!(dictionary.entries.len(), 1);
        assert_eq!(dictionary.entries[0].code, ["ci"]);
    }

    #[test]
    fn uses_stored_pinyin_index() {
        let data = fixture(&[]);
        let dictionary = must_parse(&data);

        assert_eq!(dictionary.entries[0].word, "词");
        assert_eq!(dictionary.entries[0].code, ["ci"]);
    }

    #[test]
    fn resolves_implicit_english_code_indices() {
        let data = fixture_with_pinyin(&[(0, "ci")], &[1, 2, 26], &[]);
        let dictionary = must_parse(&data);

        assert_eq!(dictionary.entries[0].code, ["a", "b", "z"]);
    }

    #[test]
    fn prefers_stored_code_over_implicit_english_index() {
        let data = fixture_with_pinyin(&[(0, "ci"), (2, "A")], &[2], &[]);
        let dictionary = must_parse(&data);

        assert_eq!(dictionary.entries[0].code, ["A"]);
    }

    #[test]
    fn rejects_code_index_after_implicit_english_alphabet() {
        let data = fixture_with_pinyin(&[(0, "ci")], &[27], &[]);
        let error = must_fail(&data);

        assert!(matches!(
            error,
            Error::InvalidCodeIndex {
                format: Format::Scel,
                index: 27,
                ..
            }
        ));
    }

    #[test]
    fn ignores_trailing_data_after_declared_word_groups() {
        let mut data = fixture(&[]);
        data.extend_from_slice(b"tail");
        let dictionary = must_parse(&data);

        assert_eq!(dictionary.entries.len(), 1);
        assert_eq!(dictionary.entries[0].word, "词");
    }

    #[test]
    fn extension_without_bytes_has_no_weight() {
        let data = fixture(&[]);
        let dictionary = must_parse(&data);

        assert_eq!(dictionary.entries[0].weight, None);
    }

    #[test]
    fn one_byte_extension_has_no_weight() {
        let data = fixture(&[0x34]);
        let dictionary = must_parse(&data);

        assert_eq!(dictionary.entries[0].weight, None);
    }

    #[test]
    fn two_byte_extension_sets_weight() {
        let data = fixture(&[0x34, 0x12]);
        let dictionary = must_parse(&data);

        assert_eq!(dictionary.entries[0].weight, Some(0x1234));
    }

    #[test]
    fn longer_extension_is_consumed_and_only_first_two_bytes_set_weight() {
        let mut data = fixture(&[0x34, 0x12, 9, 8]);
        set_u32(&mut data, TOTAL_WORDS_OFFSET, 2);

        let word_table = pinyin_table_end(&data);
        set_u16(&mut data, word_table, 2);
        push_utf16(&mut data, "第二词");
        push_u16(&mut data, 0);

        let dictionary = must_parse(&data);

        assert_eq!(dictionary.entries.len(), 2);
        assert_eq!(dictionary.entries[0].weight, Some(0x1234));
        assert_eq!(dictionary.entries[1].word, "第二词");
        assert_eq!(dictionary.entries[1].weight, None);
    }

    #[test]
    fn rejects_unknown_pinyin_index() {
        let mut data = fixture(&[1, 0]);
        let referenced_index_offset = pinyin_table_end(&data) + 4;
        set_u16(&mut data, referenced_index_offset, 99);
        let error = must_fail(&data);

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
    fn rejects_duplicate_pinyin_index() {
        let data = fixture_with_pinyin(&[(42, "ci"), (42, "qi")], &[42], &[]);
        let error = must_fail(&data);

        assert!(matches!(
            error,
            Error::DuplicateCodeIndex {
                format: Format::Scel,
                index: 42,
            }
        ));
    }

    #[test]
    fn rejects_odd_pinyin_entry_length() {
        let mut data = fixture(&[]);
        let length_offset = PINYIN_TABLE_OFFSET + 2;
        set_u16(&mut data, length_offset, 3);

        let error = must_fail(&data);

        assert!(matches!(
            error,
            Error::OddUtf16ByteLength {
                format: Format::Scel,
                field: "pinyin table entry",
                length: 3,
                ..
            }
        ));
    }

    #[test]
    fn rejects_invalid_utf16_pinyin() {
        let mut data = fixture(&[]);
        let pinyin_offset = PINYIN_TABLE_OFFSET + 4;
        set_u16(&mut data, pinyin_offset, 0xd800);

        let error = must_fail(&data);

        match error {
            Error::InvalidUtf16 {
                format: Format::Scel,
                field: "pinyin table entry",
                source,
                ..
            } => assert_eq!(source.unpaired_surrogate(), 0xd800),
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn rejects_impossible_pinyin_count() {
        let mut data = fixture(&[]);
        set_u32(&mut data, PINYIN_COUNT_OFFSET, u32::MAX);

        let error = must_fail(&data);

        match error {
            Error::InvalidCount {
                format: Format::Scel,
                field: "pinyin table",
                count,
                maximum,
            } => {
                assert_eq!(count, u64::from(u16::MAX));
                assert!(maximum < count);
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn rejects_impossible_word_group_count() {
        let mut data = fixture(&[]);
        set_u32(&mut data, RECORD_COUNT_OFFSET, u32::MAX);

        let error = must_fail(&data);

        match error {
            Error::InvalidCount {
                format: Format::Scel,
                field: "word group",
                count,
                maximum,
            } => {
                assert_eq!(count, u64::from(u32::MAX));
                assert!(maximum < count);
            }
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn rejects_odd_word_length() {
        let mut data = fixture(&[1, 0]);
        let word_table = pinyin_table_end(&data);
        let word_length_offset = word_table + 4 + 2;
        set_u16(&mut data, word_length_offset, 1);

        let error = must_fail(&data);
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
    fn rejects_odd_pinyin_index_length() {
        let mut data = fixture(&[]);
        let word_table = pinyin_table_end(&data);
        let pinyin_length_offset = word_table + 2;
        set_u16(&mut data, pinyin_length_offset, 1);

        let error = must_fail(&data);

        assert!(matches!(
            error,
            Error::OddUtf16ByteLength {
                format: Format::Scel,
                field: "pinyin indices",
                length: 1,
                ..
            }
        ));
    }

    #[test]
    fn rejects_invalid_utf16_word() {
        let mut data = fixture(&[1, 0]);
        let word_table = pinyin_table_end(&data);
        let word_offset = word_table + 4 + 2 + 2;
        set_u16(&mut data, word_offset, 0xd800);

        let error = must_fail(&data);
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
        let mut data = fixture(&[1, 0]);
        set_u32(&mut data, TOTAL_WORDS_OFFSET, 2);

        let error = must_fail(&data);
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
    fn reports_truncated_metadata() {
        let mut data = fixture(&[]);
        data.truncate(100);
        let error = must_fail(&data);
        assert!(matches!(
            error,
            Error::UnexpectedEof {
                format: Format::Scel,
                offset: 0x130,
                needed: 520,
                available: 0,
            }
        ));
    }

    #[test]
    fn reports_pinyin_header_missing_unknown_field() {
        let mut data = fixture(&[]);
        data.truncate(PINYIN_COUNT_OFFSET + 2);

        let error = must_fail(&data);
        assert!(matches!(
            error,
            Error::UnexpectedEof {
                format: Format::Scel,
                offset: PINYIN_COUNT_OFFSET,
                needed: 4,
                available: 2,
            }
        ));
    }

    #[test]
    fn reports_truncated_pinyin_header_unknown_field() {
        let mut data = fixture(&[]);
        data.truncate(PINYIN_COUNT_OFFSET + 3);

        let error = must_fail(&data);
        assert!(matches!(
            error,
            Error::UnexpectedEof {
                format: Format::Scel,
                offset: PINYIN_COUNT_OFFSET,
                needed: 4,
                available: 3,
            }
        ));
    }

    #[test]
    fn reports_truncated_pinyin_entry() {
        let mut data = fixture(&[]);
        data.truncate(PINYIN_TABLE_OFFSET + 5);

        let error = must_fail(&data);
        assert!(matches!(
            error,
            Error::UnexpectedEof {
                format: Format::Scel,
                offset,
                needed: 4,
                available: 1,
            } if offset == PINYIN_TABLE_OFFSET + 4
        ));
    }

    #[test]
    fn reports_truncated_word_extension() {
        let mut data = fixture(&[0x34, 0x12, 9, 8]);
        let extension_offset = data.len() - 4;
        data.truncate(data.len() - 1);

        let error = must_fail(&data);
        assert!(matches!(
            error,
            Error::UnexpectedEof {
                format: Format::Scel,
                offset,
                needed: 4,
                available: 3,
            } if offset == extension_offset
        ));
    }

    #[test]
    fn parse_file_does_not_infer_missing_name_from_path() -> TestResult {
        let mut data = fixture(&[1, 0]);
        data[NAME_RANGE].fill(0);
        let expected = must_parse(&data);

        let dictionary = parse_temp_file("cidian-rs-no-name", &data)?;
        assert_eq!(dictionary, expected);
        assert_eq!(dictionary.metadata.name, None);

        Ok(())
    }

    #[test]
    fn parse_file_preserves_embedded_name() -> TestResult {
        let data = fixture(&[1, 0]);
        let expected = must_parse(&data);

        let dictionary = parse_temp_file("cidian-rs-different-name", &data)?;
        assert_eq!(dictionary, expected);
        assert_eq!(dictionary.metadata.name.as_deref(), Some("测试词库"));

        Ok(())
    }

    #[test]
    fn parse_file_exposes_io_error_context() {
        let path = std::env::temp_dir()
            .join(format!(
                "cidian-rs-missing-directory-{}",
                std::process::id()
            ))
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

    #[track_caller]
    fn must_parse(data: &[u8]) -> Dictionary {
        match parse(data) {
            Ok(dictionary) => dictionary,
            Err(error) => panic!("test fixture failed to parse: {error}"),
        }
    }

    #[track_caller]
    fn must_fail(data: &[u8]) -> Error {
        match parse(data) {
            Ok(_) => panic!("invalid fixture unexpectedly parsed"),
            Err(error) => error,
        }
    }

    #[track_caller]
    fn parse_temp_file(label: &str, data: &[u8]) -> TestResult<Dictionary> {
        let path = std::env::temp_dir().join(format!("{label}-{}.scel", std::process::id()));
        fs::write(&path, data)?;
        let parsed = parse_file(&path);
        let removed = fs::remove_file(&path);

        let dictionary = parsed?;
        removed?;
        Ok(dictionary)
    }

    fn fixture(extension: &[u8]) -> Vec<u8> {
        fixture_with_pinyin(&[(42, "ci")], &[42], extension)
    }

    fn fixture_with_pinyin(
        pinyin_entries: &[(u16, &str)],
        referenced_indices: &[u16],
        extension: &[u8],
    ) -> Vec<u8> {
        let mut data = vec![0; PINYIN_TABLE_OFFSET];
        data[0..4].copy_from_slice(&[0x40, 0x15, 0, 0]);
        data[4..7].copy_from_slice(b"DCS");
        data[7..12].copy_from_slice(&[1, 1, 0, 0, 0]);
        set_u32(&mut data, RECORD_COUNT_OFFSET, 1);
        set_u32(&mut data, TOTAL_WORDS_OFFSET, 1);
        set_u16(&mut data, PINYIN_COUNT_OFFSET, pinyin_entries.len() as u16);
        write_fixed_utf16(&mut data, NAME_RANGE.start, "测试词库");

        for &(index, text) in pinyin_entries {
            push_u16(&mut data, index);
            push_utf16(&mut data, text);
        }

        push_u16(&mut data, 1);
        push_u16(&mut data, (referenced_indices.len() * 2) as u16);
        for &index in referenced_indices {
            push_u16(&mut data, index);
        }
        push_utf16(&mut data, "词");
        push_u16(&mut data, extension.len() as u16);
        data.extend_from_slice(extension);
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

    fn set_u16(data: &mut [u8], offset: usize, value: u16) {
        data[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn set_u32(data: &mut [u8], offset: usize, value: u32) {
        data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }
}
