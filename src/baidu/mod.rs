mod parser;

pub mod bcd;
pub mod bdict;

pub(crate) use parser::{BaiduVariant, parse, parse_file};

#[cfg(test)]
mod tests {
    use super::{BaiduVariant, parse};
    use crate::{Dictionary, Error, Format};

    const MAGIC: &[u8; 8] = b"biptbdsw";
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

    #[test]
    fn parses_bdict_sections_and_mixed_record_variants() {
        let data = bdict_fixture();
        let dictionary = must_parse(&data, BaiduVariant::Bdict);

        assert_eq!(dictionary.metadata.name.as_deref(), Some("测试百度词库"));
        assert_eq!(dictionary.metadata.category, None);
        assert_eq!(dictionary.metadata.description.as_deref(), Some("测试描述"));
        assert_eq!(
            dictionary.metadata.extra.get("author").map(String::as_str),
            Some("测试作者")
        );
        assert_eq!(
            dictionary.metadata.extra.get("example").map(String::as_str),
            Some("你好，Rust")
        );

        assert_eq!(dictionary.entries.len(), 6);
        assert_eq!(dictionary.entries[0].word, "你好");
        assert_eq!(dictionary.entries[0].code, ["ni", "hao"]);
        assert_eq!(dictionary.entries[0].weight, Some(42));
        assert_eq!(dictionary.entries[1].word, "A");
        assert_eq!(dictionary.entries[1].code, ["A"]);
        assert_eq!(dictionary.entries[1].weight, Some(3));
        assert_eq!(dictionary.entries[2].word, "女娲");
        assert_eq!(dictionary.entries[2].code, ["nv", "wa"]);
        assert_eq!(dictionary.entries[2].weight, Some(9));
        assert_eq!(dictionary.entries[3].word, "Rust");
        assert_eq!(dictionary.entries[3].code, ["Rust"]);
        assert_eq!(dictionary.entries[3].weight, Some(7));
        assert_eq!(dictionary.entries[4].word, "剑网3");
        assert_eq!(dictionary.entries[4].code, ["jianwang"]);
        assert_eq!(dictionary.entries[4].weight, Some(0));
        assert_eq!(dictionary.entries[5].word, "俘虏Darling");
        assert_eq!(dictionary.entries[5].code, ["fu'lu'Darling"]);
        assert_eq!(dictionary.entries[5].weight, None);
    }

    #[test]
    fn parses_bcd_fixed_regular_section() {
        let data = bcd_fixture();
        let dictionary = must_parse(&data, BaiduVariant::Bcd);

        assert_eq!(dictionary.metadata.name.as_deref(), Some("手机百度词库"));
        assert_eq!(dictionary.entries.len(), 1);
        assert_eq!(dictionary.entries[0].word, "你好");
        assert_eq!(dictionary.entries[0].code, ["ni", "hao"]);
        assert_eq!(dictionary.entries[0].weight, Some(1000));
    }

    #[test]
    fn bcd_parse_file_exposes_io_context() {
        let path =
            std::env::temp_dir().join(format!("cidian-rs-missing-bcd-{}.bcd", std::process::id()));
        let error = match super::bcd::parse_file(&path) {
            Ok(_) => panic!("missing BCD file unexpectedly parsed"),
            Err(error) => error,
        };

        assert!(matches!(
            error,
            Error::Io {
                format: Format::Bcd,
                path: error_path,
                ..
            } if error_path == path
        ));
    }

    #[test]
    fn bdict_parse_file_exposes_io_context() {
        let path = std::env::temp_dir().join(format!(
            "cidian-rs-missing-bdict-{}.bdict",
            std::process::id()
        ));
        let error = match super::bdict::parse_file(&path) {
            Ok(_) => panic!("missing BDICT file unexpectedly parsed"),
            Err(error) => error,
        };

        assert!(matches!(
            error,
            Error::Io {
                format: Format::Bdict,
                path: error_path,
                ..
            } if error_path == path
        ));
    }

    #[test]
    fn rejects_invalid_magic() {
        let mut data = bdict_fixture();
        data[..MAGIC.len()].fill(0);

        let error = must_fail(&data, BaiduVariant::Bdict);
        assert!(matches!(
            error,
            Error::InvalidMagic {
                format: Format::Bdict,
                found,
            } if found == vec![0; MAGIC.len()]
        ));
    }

    #[test]
    fn rejects_unsupported_version() {
        let mut data = bcd_fixture();
        set_u32(&mut data, VERSION_OFFSET, 2);

        let error = must_fail(&data, BaiduVariant::Bcd);
        assert!(matches!(
            error,
            Error::UnsupportedVersion {
                format: Format::Bcd,
                found,
            } if found == 2_u32.to_le_bytes()
        ));
    }

    #[test]
    fn rejects_invalid_utf16_metadata() {
        let mut data = bdict_fixture();
        data[0x90..0x92].copy_from_slice(&0xd800_u16.to_le_bytes());

        let error = must_fail(&data, BaiduVariant::Bdict);
        assert!(matches!(
            error,
            Error::InvalidUtf16 {
                format: Format::Bdict,
                field: "dictionary name",
                offset: 0x90,
                ..
            }
        ));
    }

    #[test]
    fn rejects_invalid_initial_index() {
        let mut data = bdict_fixture();
        data[HEADER_LEN + 4] = 24;

        let error = must_fail(&data, BaiduVariant::Bdict);
        assert!(matches!(
            error,
            Error::InvalidCodeComponent {
                format: Format::Bdict,
                field: "initial",
                index: 24,
                offset,
            } if offset == HEADER_LEN + 4
        ));
    }

    #[test]
    fn rejects_invalid_final_index() {
        let mut data = bdict_fixture();
        data[HEADER_LEN + 5] = 33;

        let error = must_fail(&data, BaiduVariant::Bdict);
        assert!(matches!(
            error,
            Error::InvalidCodeComponent {
                format: Format::Bdict,
                field: "final",
                index: 33,
                offset,
            } if offset == HEADER_LEN + 5
        ));
    }

    #[test]
    fn rejects_non_ascii_embedded_code() {
        let mut data = bdict_fixture();
        let second_entry = HEADER_LEN + regular_entry_len(2) + 4;
        data[second_entry] = 0xff;
        data[second_entry + 1] = 0x80;

        let error = must_fail(&data, BaiduVariant::Bdict);
        assert!(matches!(
            error,
            Error::InvalidAscii {
                format: Format::Bdict,
                field: "embedded Latin code",
                byte: 0x80,
                ..
            }
        ));
    }

    #[test]
    fn rejects_non_ascii_english_word() {
        let mut data = bdict_fixture();
        let english_offset = u32_at(&data, ENGLISH_OFFSET_OFFSET) as usize;
        data[english_offset + 4] = 0x80;

        let error = must_fail(&data, BaiduVariant::Bdict);
        assert!(matches!(
            error,
            Error::InvalidAscii {
                format: Format::Bdict,
                field: "English dictionary word",
                byte: 0x80,
                ..
            }
        ));
    }

    #[test]
    fn rejects_invalid_mixed_record_header() {
        let mut data = bdict_fixture();
        let mixed_offset = u32_at(&data, MIXED_OFFSET_OFFSET) as usize;
        data[mixed_offset..mixed_offset + 8].fill(0);

        let error = must_fail(&data, BaiduVariant::Bdict);
        assert!(matches!(
            error,
            Error::InvalidRecordHeader {
                format: Format::Bdict,
                field: "mixed entry",
                found,
                ..
            } if found == vec![0; 8]
        ));
    }

    #[test]
    fn rejects_impossible_section_count() {
        let mut data = bdict_fixture();
        set_u32(&mut data, REGULAR_COUNT_OFFSET, u32::MAX);

        let error = must_fail(&data, BaiduVariant::Bdict);
        assert!(matches!(
            error,
            Error::InvalidCount {
                format: Format::Bdict,
                field: "regular entries",
                ..
            }
        ));
    }

    #[test]
    fn rejects_section_size_mismatch() {
        let mut data = bdict_fixture();
        let regular_size = u32_at(&data, REGULAR_SIZE_OFFSET);
        set_u32(&mut data, REGULAR_SIZE_OFFSET, regular_size + 1);

        let error = must_fail(&data, BaiduVariant::Bdict);
        assert!(matches!(
            error,
            Error::SizeMismatch {
                format: Format::Bdict,
                field: "regular entries",
                expected,
                actual,
            } if expected == u64::from(regular_size + 1) && actual == u64::from(regular_size)
        ));
    }

    #[test]
    fn reports_truncated_header() {
        let mut data = bdict_fixture();
        data.truncate(HEADER_LEN - 1);

        let error = must_fail(&data, BaiduVariant::Bdict);
        assert!(matches!(
            error,
            Error::UnexpectedEof {
                format: Format::Bdict,
                offset: 0,
                needed: HEADER_LEN,
                available,
            } if available == HEADER_LEN - 1
        ));
    }

    fn must_parse(data: &[u8], variant: BaiduVariant) -> Dictionary {
        match parse(data, variant) {
            Ok(dictionary) => dictionary,
            Err(error) => panic!("test fixture failed to parse: {error}"),
        }
    }

    fn must_fail(data: &[u8], variant: BaiduVariant) -> Error {
        match parse(data, variant) {
            Ok(_) => panic!("invalid fixture unexpectedly parsed"),
            Err(error) => error,
        }
    }

    fn bdict_fixture() -> Vec<u8> {
        let mut regular = Vec::new();
        push_regular_entry(&mut regular, &[(11, 29), (5, 13)], 42, "你好");
        push_regular_entry(&mut regular, &[(0xff, b'A')], 3, "A");
        push_regular_entry(&mut regular, &[(11, 32), (20, 27)], 9, "女娲");

        let mut english = Vec::new();
        push_english_entry(&mut english, "Rust", 7);

        let mut mixed = Vec::new();
        push_mixed_entry_with_weight(&mut mixed, "jianwang", "剑网3", 0);
        push_mixed_entry_without_weight(&mut mixed, "fu'lu'Darling", "俘虏Darling");

        container(
            BaiduVariant::Bdict,
            (&regular, 3),
            (&english, 1),
            (&mixed, 2),
        )
    }

    fn bcd_fixture() -> Vec<u8> {
        let mut regular = Vec::new();
        push_regular_entry(&mut regular, &[(11, 29), (5, 13)], 1000, "你好");
        container(BaiduVariant::Bcd, (&regular, 1), (&[], 0), (&[], 0))
    }

    fn container(
        variant: BaiduVariant,
        regular: (&[u8], u32),
        english: (&[u8], u32),
        mixed: (&[u8], u32),
    ) -> Vec<u8> {
        let mut data = vec![0; HEADER_LEN];
        data[..MAGIC.len()].copy_from_slice(MAGIC);
        set_u32(&mut data, VERSION_OFFSET, 1);
        set_u32(&mut data, REGULAR_COUNT_OFFSET, regular.1);
        set_u32(&mut data, ENGLISH_COUNT_OFFSET, english.1);
        set_u32(&mut data, MIXED_COUNT_OFFSET, mixed.1);
        write_fixed_utf16(&mut data, 0x90, "测试百度词库");
        write_fixed_utf16(&mut data, 0xd0, "测试作者");
        write_fixed_utf16(&mut data, 0x110, "你好，Rust");
        write_fixed_utf16(&mut data, 0x150, "测试描述");

        if variant == BaiduVariant::Bcd {
            write_fixed_utf16(&mut data, 0x90, "手机百度词库");
            data.extend_from_slice(regular.0);
            return data;
        }

        append_section(
            &mut data,
            regular.0,
            REGULAR_OFFSET_OFFSET,
            REGULAR_SIZE_OFFSET,
        );
        append_section(
            &mut data,
            english.0,
            ENGLISH_OFFSET_OFFSET,
            ENGLISH_SIZE_OFFSET,
        );
        append_section(&mut data, mixed.0, MIXED_OFFSET_OFFSET, MIXED_SIZE_OFFSET);
        data
    }

    fn append_section(data: &mut Vec<u8>, section: &[u8], offset_field: usize, size_field: usize) {
        if section.is_empty() {
            return;
        }

        let offset = data.len() as u32;
        set_u32(data, offset_field, offset);
        set_u32(data, size_field, section.len() as u32);
        data.extend_from_slice(section);
    }

    fn push_regular_entry(data: &mut Vec<u8>, code: &[(u8, u8)], weight: u16, word: &str) {
        assert_eq!(code.len(), word.encode_utf16().count());
        push_u16(data, code.len() as u16);
        push_u16(data, weight);
        for &(initial, final_part) in code {
            data.push(initial);
            data.push(final_part);
        }
        push_utf16(data, word);
    }

    fn push_english_entry(data: &mut Vec<u8>, word: &str, weight: u16) {
        push_u16(data, word.len() as u16);
        push_u16(data, weight);
        data.extend_from_slice(word.as_bytes());
    }

    fn push_mixed_entry_with_weight(data: &mut Vec<u8>, code: &str, word: &str, weight: u16) {
        push_u16(data, code.encode_utf16().count() as u16);
        push_u16(data, weight);
        push_u16(data, 0);
        push_u16(data, word.encode_utf16().count() as u16);
        push_utf16(data, code);
        push_utf16(data, word);
    }

    fn push_mixed_entry_without_weight(data: &mut Vec<u8>, code: &str, word: &str) {
        push_u16(data, 0);
        push_u16(data, 0);
        push_u16(data, code.encode_utf16().count() as u16);
        push_u16(data, word.encode_utf16().count() as u16);
        push_utf16(data, code);
        push_utf16(data, word);
    }

    fn regular_entry_len(code_len: usize) -> usize {
        4 + code_len * 4
    }

    fn write_fixed_utf16(data: &mut [u8], offset: usize, value: &str) {
        for (index, unit) in value.encode_utf16().enumerate() {
            let start = offset + index * 2;
            data[start..start + 2].copy_from_slice(&unit.to_le_bytes());
        }
    }

    fn push_utf16(data: &mut Vec<u8>, value: &str) {
        data.extend(value.encode_utf16().flat_map(u16::to_le_bytes));
    }

    fn push_u16(data: &mut Vec<u8>, value: u16) {
        data.extend_from_slice(&value.to_le_bytes());
    }

    fn set_u32(data: &mut [u8], offset: usize, value: u32) {
        data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn u32_at(data: &[u8], offset: usize) -> u32 {
        u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ])
    }
}
