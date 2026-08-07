//! Parser for QQ Pinyin Cell Dictionary (`.qcel`) files.
//!
//! QCEL shares its main word-group layout with SCEL but is maintained as an
//! independent parser because its header variants, default code table, and
//! word-extension semantics belong to QQ Pinyin.

mod qcel;

pub use qcel::{parse, parse_file};

#[cfg(test)]
mod tests {
    use std::fs;

    use super::*;
    use crate::{Dictionary, Error, Format};

    const RECORD_COUNT_OFFSET: usize = 0x120;
    const TOTAL_WORDS_OFFSET: usize = 0x124;
    const NAME_RANGE: std::ops::Range<usize> = 0x130..0x338;
    const CATEGORY_RANGE: std::ops::Range<usize> = 0x338..0x540;
    const DESCRIPTION_RANGE: std::ops::Range<usize> = 0x540..0x0d40;
    const EXAMPLE_RANGE: std::ops::Range<usize> = 0x0d40..0x1540;
    const PINYIN_COUNT_OFFSET: usize = 0x1540;
    const PINYIN_TABLE_OFFSET: usize = 0x1544;
    const VALID_EXTENSION: [u8; 4] = [0, 0, 0, 0];
    type TestResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

    #[test]
    fn parses_metadata_and_u32_weight() {
        let data = fixture(&[0x78, 0x56, 0x34, 0x12]);
        let dictionary = must_parse(&data);

        assert_eq!(dictionary.metadata.name.as_deref(), Some("测试词库"));
        assert_eq!(dictionary.metadata.category.as_deref(), Some("测试分类"));
        assert_eq!(dictionary.metadata.description.as_deref(), Some("测试描述"));
        assert_eq!(
            dictionary.metadata.extra.get("example").map(String::as_str),
            Some("测试示例")
        );
        assert_eq!(dictionary.entries.len(), 1);
        assert_eq!(dictionary.entries[0].word, "词");
        assert_eq!(dictionary.entries[0].code, ["ci"]);
        assert_eq!(dictionary.entries[0].weight, Some(0x1234_5678));
    }

    #[test]
    fn accepts_dcs_variant() {
        let mut data = fixture(&VALID_EXTENSION);
        data[4..7].copy_from_slice(b"DCS");

        assert_eq!(must_parse(&data).entries.len(), 1);
    }

    #[test]
    fn reads_full_u32_pinyin_count() {
        let mut data = fixture(&VALID_EXTENSION);
        set_u16(&mut data, PINYIN_COUNT_OFFSET + 2, 1);

        assert!(matches!(
            must_fail(&data),
            Error::InvalidCount {
                format: Format::Qcel,
                field: "pinyin table",
                count: 65_537,
                ..
            }
        ));
    }

    #[test]
    fn rejects_invalid_magic() {
        let mut data = fixture(&VALID_EXTENSION);
        data[0..4].fill(0);

        assert!(matches!(
            must_fail(&data),
            Error::InvalidMagic {
                format: Format::Qcel,
                found,
            } if found == vec![0; 4]
        ));
    }

    #[test]
    fn rejects_unknown_variant() {
        let mut data = fixture(&VALID_EXTENSION);
        data[4..7].copy_from_slice(b"XYZ");

        assert!(matches!(
            must_fail(&data),
            Error::UnsupportedVariant {
                format: Format::Qcel,
                found,
            } if found == b"XYZ"
        ));
    }

    #[test]
    fn rejects_unsupported_version() {
        let mut data = fixture(&VALID_EXTENSION);
        data[7..12].copy_from_slice(&[2, 1, 0, 0, 0]);

        assert!(matches!(
            must_fail(&data),
            Error::UnsupportedVersion {
                format: Format::Qcel,
                found,
            } if found == vec![2, 1, 0, 0, 0]
        ));
    }

    #[test]
    fn uses_default_code_table_when_source_table_is_empty() {
        let data = fixture_with_pinyin(&[], &[48, 413, 448, 482], &VALID_EXTENSION);
        let dictionary = must_parse(&data);

        assert_eq!(dictionary.entries[0].code, ["ci", "A", "9", "#"]);
    }

    #[test]
    fn resolves_implicit_english_code_indices() {
        let data = fixture_with_pinyin(&[(0, "ci")], &[1, 2, 26], &VALID_EXTENSION);
        let dictionary = must_parse(&data);

        assert_eq!(dictionary.entries[0].code, ["a", "b", "z"]);
    }

    #[test]
    fn rejects_unknown_code_index() {
        let data = fixture_with_pinyin(&[(0, "ci")], &[27], &VALID_EXTENSION);

        assert!(matches!(
            must_fail(&data),
            Error::InvalidCodeIndex {
                format: Format::Qcel,
                index: 27,
                ..
            }
        ));
    }

    #[test]
    fn rejects_duplicate_pinyin_index() {
        let data = fixture_with_pinyin(&[(42, "ci"), (42, "qi")], &[42], &VALID_EXTENSION);

        assert!(matches!(
            must_fail(&data),
            Error::DuplicateCodeIndex {
                format: Format::Qcel,
                index: 42,
            }
        ));
    }

    #[test]
    fn rejects_odd_pinyin_entry_length() {
        let mut data = fixture(&VALID_EXTENSION);
        set_u16(&mut data, PINYIN_TABLE_OFFSET + 2, 3);

        assert!(matches!(
            must_fail(&data),
            Error::OddUtf16ByteLength {
                format: Format::Qcel,
                field: "pinyin table entry",
                length: 3,
                ..
            }
        ));
    }

    #[test]
    fn rejects_odd_pinyin_index_length() {
        let mut data = fixture(&VALID_EXTENSION);
        let word_table = pinyin_table_end(&data);
        set_u16(&mut data, word_table + 2, 1);

        assert!(matches!(
            must_fail(&data),
            Error::OddUtf16ByteLength {
                format: Format::Qcel,
                field: "pinyin indices",
                length: 1,
                ..
            }
        ));
    }

    #[test]
    fn rejects_invalid_utf16_pinyin() {
        let mut data = fixture(&VALID_EXTENSION);
        set_u16(&mut data, PINYIN_TABLE_OFFSET + 4, 0xd800);

        assert!(matches!(
            must_fail(&data),
            Error::InvalidUtf16 {
                format: Format::Qcel,
                field: "pinyin table entry",
                ..
            }
        ));
    }

    #[test]
    fn rejects_invalid_utf16_word() {
        let mut data = fixture(&VALID_EXTENSION);
        let word_offset = pinyin_table_end(&data) + 8;
        set_u16(&mut data, word_offset, 0xd800);

        assert!(matches!(
            must_fail(&data),
            Error::InvalidUtf16 {
                format: Format::Qcel,
                field: "dictionary word",
                ..
            }
        ));
    }

    #[test]
    fn rejects_impossible_pinyin_count() {
        let mut data = fixture(&VALID_EXTENSION);
        set_u32(&mut data, PINYIN_COUNT_OFFSET, u32::MAX);

        assert!(matches!(
            must_fail(&data),
            Error::InvalidCount {
                format: Format::Qcel,
                field: "pinyin table",
                ..
            }
        ));
    }

    #[test]
    fn rejects_impossible_word_group_count() {
        let mut data = fixture(&VALID_EXTENSION);
        set_u32(&mut data, RECORD_COUNT_OFFSET, u32::MAX);

        assert!(matches!(
            must_fail(&data),
            Error::InvalidCount {
                format: Format::Qcel,
                field: "word group",
                ..
            }
        ));
    }

    #[test]
    fn rejects_word_count_mismatch() {
        let mut data = fixture(&VALID_EXTENSION);
        set_u32(&mut data, TOTAL_WORDS_OFFSET, 2);

        assert!(matches!(
            must_fail(&data),
            Error::CountMismatch {
                format: Format::Qcel,
                field: "word",
                expected: 2,
                actual: 1,
            }
        ));
    }

    #[test]
    fn rejects_extension_shorter_than_u32() {
        for extension in [&[][..], &[1][..], &[1, 2][..], &[1, 2, 3][..]] {
            let data = fixture(extension);
            let error = must_fail(&data);

            assert!(matches!(
                error,
                Error::InvalidExtensionLength {
                    format: Format::Qcel,
                    field: "word extension",
                    length,
                    minimum: 4,
                    ..
                } if length == extension.len()
            ));
        }
    }

    #[test]
    fn longer_extension_is_consumed_and_only_first_u32_sets_weight() {
        let mut data = fixture(&[0x78, 0x56, 0x34, 0x12, 9, 8]);
        set_u32(&mut data, TOTAL_WORDS_OFFSET, 2);

        let word_table = pinyin_table_end(&data);
        set_u16(&mut data, word_table, 2);
        push_utf16(&mut data, "第二词");
        push_u16(&mut data, VALID_EXTENSION.len() as u16);
        data.extend_from_slice(&VALID_EXTENSION);

        let dictionary = must_parse(&data);
        assert_eq!(dictionary.entries.len(), 2);
        assert_eq!(dictionary.entries[0].weight, Some(0x1234_5678));
        assert_eq!(dictionary.entries[1].word, "第二词");
        assert_eq!(dictionary.entries[1].weight, Some(0));
    }

    #[test]
    fn ignores_trailing_data_after_declared_word_groups() {
        let mut data = fixture(&VALID_EXTENSION);
        data.extend_from_slice(b"D\0E\0L\0T\0B\0L\0");

        let dictionary = must_parse(&data);
        assert_eq!(dictionary.entries.len(), 1);
        assert_eq!(dictionary.entries[0].word, "词");
    }

    #[test]
    fn reports_truncated_word_extension() {
        let mut data = fixture(&[0x78, 0x56, 0x34, 0x12]);
        let extension_offset = data.len() - 4;
        data.truncate(data.len() - 1);

        assert!(matches!(
            must_fail(&data),
            Error::UnexpectedEof {
                format: Format::Qcel,
                offset,
                needed: 4,
                available: 3,
            } if offset == extension_offset
        ));
    }

    #[test]
    fn parse_file_matches_memory_parse() -> TestResult {
        let data = fixture(&[0x78, 0x56, 0x34, 0x12]);
        let expected = must_parse(&data);
        let parsed = parse_temp_file("cidian-rs-qcel", &data)?;

        assert_eq!(parsed, expected);
        Ok(())
    }

    #[test]
    fn parse_file_exposes_io_error_context() {
        let path = std::env::temp_dir()
            .join(format!("cidian-rs-missing-qcel-{}", std::process::id()))
            .join("dictionary.qcel");

        let error = match parse_file(&path) {
            Ok(_) => panic!("missing dictionary unexpectedly parsed"),
            Err(error) => error,
        };

        assert!(matches!(
            error,
            Error::Io {
                format: Format::Qcel,
                path: error_path,
                source,
            } if error_path == path && source.kind() == std::io::ErrorKind::NotFound
        ));
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
        let path = std::env::temp_dir().join(format!("{label}-{}.qcel", std::process::id()));
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
        data[4..7].copy_from_slice(&[0xd2, 0x6d, 0x53]);
        data[7..12].copy_from_slice(&[1, 1, 0, 0, 0]);
        set_u32(&mut data, RECORD_COUNT_OFFSET, 1);
        set_u32(&mut data, TOTAL_WORDS_OFFSET, 1);
        set_u32(&mut data, PINYIN_COUNT_OFFSET, pinyin_entries.len() as u32);
        write_fixed_utf16(&mut data, NAME_RANGE.start, "测试词库");
        write_fixed_utf16(&mut data, CATEGORY_RANGE.start, "测试分类");
        write_fixed_utf16(&mut data, DESCRIPTION_RANGE.start, "测试描述");
        write_fixed_utf16(&mut data, EXAMPLE_RANGE.start, "测试示例");

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
