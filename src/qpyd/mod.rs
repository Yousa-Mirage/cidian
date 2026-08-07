//! Parser for QQ Pinyin category dictionary (`.qpyd`) files.
//!
//! QPYD stores textual metadata in UTF-16LE and keeps its entry index and
//! payloads in a zlib-compressed section. The parser follows the offsets,
//! lengths, decompressed size, and entry count declared by the file.

mod qpyd;

pub use qpyd::{parse, parse_file};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Dictionary, Error, Format};
    use std::fs;
    use zlib_rs::{DeflateConfig, ReturnCode, compress_bound, compress_slice};

    const MAGIC: &[u8; 8] = b"\x09\xa6\x1e\x7d\x01\x00\x00\x00";
    const HEADER_LEN: usize = 0x48;
    const INFO_OFFSET: usize = 0x60;
    const INFO_OFFSET_OFFSET: usize = 0x2c;
    const INFO_SIZE_OFFSET: usize = 0x30;
    const COMPRESSED_OFFSET_OFFSET: usize = 0x38;
    const COMPRESSED_SIZE_OFFSET: usize = 0x3c;
    const DECOMPRESSED_SIZE_OFFSET: usize = 0x40;
    const ENTRY_COUNT_OFFSET: usize = 0x44;
    const INDEX_RECORD_LEN: usize = 10;
    const FILETIME: u64 = 131_120_961_430_000_000;
    const VERSION: u32 = 94;
    type TestResult<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

    struct RawEntry {
        code: Vec<u8>,
        word: Vec<u8>,
    }

    #[test]
    fn parses_metadata_and_entries() {
        let data = fixture(&[
            entry("ci'ao", "次奥"),
            entry("wo'xuan'ze'gou'dai", "我选择狗带"),
        ]);

        let dictionary = must_parse(&data);

        assert_eq!(dictionary.metadata.name.as_deref(), Some("测试词库"));
        assert_eq!(dictionary.metadata.category.as_deref(), Some("日常"));
        assert_eq!(dictionary.metadata.description.as_deref(), Some("测试介绍"));
        assert_eq!(
            dictionary
                .metadata
                .extra
                .get("first_type")
                .map(String::as_str),
            Some("兴趣爱好")
        );
        assert_eq!(
            dictionary.metadata.extra.get("example").map(String::as_str),
            Some("次奥 我选择狗带 ")
        );
        assert_eq!(
            dictionary.metadata.extra.get("custom").map(String::as_str),
            Some("保留值")
        );
        assert_eq!(
            dictionary.metadata.extra.get("version").map(String::as_str),
            Some("94")
        );
        assert_eq!(
            dictionary
                .metadata
                .extra
                .get("filetime_raw")
                .map(String::as_str),
            Some("131120961430000000")
        );
        assert_eq!(dictionary.entries.len(), 2);
        assert_eq!(dictionary.entries[0].word, "次奥");
        assert_eq!(dictionary.entries[0].code, ["ci", "ao"]);
        assert_eq!(dictionary.entries[0].weight, None);
        assert_eq!(dictionary.entries[1].word, "我选择狗带");
        assert_eq!(
            dictionary.entries[1].code,
            ["wo", "xuan", "ze", "gou", "dai"]
        );
    }

    #[test]
    fn rejects_invalid_magic() {
        let mut data = fixture(&[entry("ci'ao", "次奥")]);
        data[..MAGIC.len()].fill(0);

        let error = must_fail(&data);
        assert!(matches!(
            error,
            Error::InvalidMagic {
                format: Format::Qpyd,
                found,
            } if found == vec![0; MAGIC.len()]
        ));
    }

    #[test]
    fn reports_truncated_header() {
        let mut data = fixture(&[entry("ci'ao", "次奥")]);
        data.truncate(HEADER_LEN - 1);

        let error = must_fail(&data);
        assert!(matches!(
            error,
            Error::UnexpectedEof {
                format: Format::Qpyd,
                offset: 0,
                needed: HEADER_LEN,
                available,
            } if available == HEADER_LEN - 1
        ));
    }

    #[test]
    fn reports_information_section_out_of_bounds() {
        let mut data = fixture(&[entry("ci'ao", "次奥")]);
        set_u32(&mut data, INFO_SIZE_OFFSET, u32::MAX);

        let error = must_fail(&data);
        assert!(matches!(
            error,
            Error::UnexpectedEof {
                format: Format::Qpyd,
                offset: INFO_OFFSET,
                needed,
                ..
            } if needed == u32::MAX as usize
        ));
    }

    #[test]
    fn rejects_odd_information_utf16_length() {
        let mut data = fixture(&[entry("ci'ao", "次奥")]);
        set_u32(&mut data, INFO_SIZE_OFFSET, 1);

        let error = must_fail(&data);
        assert!(matches!(
            error,
            Error::OddUtf16ByteLength {
                format: Format::Qpyd,
                field: "dictionary information",
                offset: INFO_OFFSET,
                length: 1,
            }
        ));
    }

    #[test]
    fn rejects_invalid_zlib_data() {
        let mut data = fixture(&[entry("ci'ao", "次奥")]);
        let compressed_offset = u32_at(&data, COMPRESSED_OFFSET_OFFSET) as usize;
        data[compressed_offset] = 0;

        let error = must_fail(&data);
        assert!(matches!(
            error,
            Error::InvalidCompression {
                format: Format::Qpyd,
                offset,
                ..
            } if offset == compressed_offset
        ));
    }

    #[test]
    fn rejects_decompressed_size_mismatch() {
        let mut data = fixture(&[entry("ci'ao", "次奥")]);
        let declared = u32_at(&data, DECOMPRESSED_SIZE_OFFSET);
        set_u32(&mut data, DECOMPRESSED_SIZE_OFFSET, declared + 1);

        let error = must_fail(&data);
        assert!(matches!(
            error,
            Error::SizeMismatch {
                format: Format::Qpyd,
                field: "decompressed data",
                expected,
                actual,
            } if expected == u64::from(declared + 1) && actual == u64::from(declared)
        ));
    }

    #[test]
    fn rejects_impossible_entry_count() {
        let mut data = fixture(&[entry("ci'ao", "次奥")]);
        set_u32(&mut data, ENTRY_COUNT_OFFSET, 100);

        let error = must_fail(&data);
        assert!(matches!(
            error,
            Error::InvalidCount {
                format: Format::Qpyd,
                field: "entry index",
                count: 100,
                ..
            }
        ));
    }

    #[test]
    fn rejects_payload_offset_inside_index() {
        let entries = [entry("ci'ao", "次奥")];
        let mut uncompressed = encode_entries(&entries);
        set_u32(&mut uncompressed, 6, 0);
        let data = container(&uncompressed, entries.len());

        let error = must_fail(&data);
        assert!(matches!(
            error,
            Error::InvalidOffset {
                format: Format::Qpyd,
                field: "entry payload",
                offset: 0,
                minimum: INDEX_RECORD_LEN,
                ..
            }
        ));
    }

    #[test]
    fn reports_payload_past_decompressed_data() {
        let entries = [entry("ci'ao", "次奥")];
        let mut uncompressed = encode_entries(&entries);
        let payload_offset = uncompressed.len() - 1;
        set_u32(&mut uncompressed, 6, payload_offset as u32);
        let data = container(&uncompressed, entries.len());

        let error = must_fail(&data);
        assert!(matches!(
            error,
            Error::UnexpectedEof {
                format: Format::Qpyd,
                offset,
                needed,
                available: 1,
            } if offset == payload_offset && needed > 1
        ));
    }

    #[test]
    fn rejects_invalid_utf8_code() {
        let data = fixture(&[RawEntry {
            code: vec![0xff],
            word: utf16_bytes("词"),
        }]);

        let error = must_fail(&data);
        assert!(matches!(
            error,
            Error::InvalidUtf8 {
                format: Format::Qpyd,
                field: "dictionary code",
                ..
            }
        ));
    }

    #[test]
    fn rejects_odd_word_utf16_length() {
        let data = fixture(&[RawEntry {
            code: b"ci".to_vec(),
            word: vec![0],
        }]);

        let error = must_fail(&data);
        assert!(matches!(
            error,
            Error::OddUtf16ByteLength {
                format: Format::Qpyd,
                field: "dictionary word",
                length: 1,
                ..
            }
        ));
    }

    #[test]
    fn rejects_invalid_utf16_word() {
        let data = fixture(&[RawEntry {
            code: b"ci".to_vec(),
            word: vec![0x00, 0xd8],
        }]);

        let error = must_fail(&data);
        match error {
            Error::InvalidUtf16 {
                format: Format::Qpyd,
                field: "dictionary word",
                source,
                ..
            } => assert_eq!(source.unpaired_surrogate(), 0xd800),
            other => panic!("unexpected error: {other}"),
        }
    }

    #[test]
    fn parse_file_matches_in_memory_parse() -> TestResult {
        let data = fixture(&[entry("ci'ao", "次奥")]);
        let expected = must_parse(&data);
        let dictionary = parse_temp_file("cidian-rs-qpyd", &data)?;

        assert_eq!(dictionary, expected);
        Ok(())
    }

    #[test]
    fn parse_file_exposes_qpyd_io_context() {
        let path = std::env::temp_dir().join(format!(
            "cidian-rs-missing-qpyd-{}-{}.qpyd",
            std::process::id(),
            line!()
        ));

        let error = match parse_file(&path) {
            Ok(_) => panic!("missing QPYD file unexpectedly parsed"),
            Err(error) => error,
        };

        assert!(matches!(
            error,
            Error::Io {
                format: Format::Qpyd,
                path: error_path,
                ..
            } if error_path == path
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
        let path = std::env::temp_dir().join(format!("{label}-{}.qpyd", std::process::id()));
        fs::write(&path, data)?;
        let parsed = parse_file(&path);
        let removed = fs::remove_file(&path);

        let dictionary = parsed?;
        removed?;
        Ok(dictionary)
    }

    fn fixture(entries: &[RawEntry]) -> Vec<u8> {
        container(&encode_entries(entries), entries.len())
    }

    fn container(uncompressed: &[u8], entry_count: usize) -> Vec<u8> {
        let info = utf16_bytes(concat!(
            "Name: 测试词库\r\n",
            "Type: 日常\r\n",
            "FirstType: 兴趣爱好\r\n",
            "Intro: 测试介绍\r\n",
            "Example: 次奥 我选择狗带 \r\n",
            "custom: 保留值\r\n",
        ));
        let compressed_offset = INFO_OFFSET + info.len();
        let mut compressed_buffer = vec![0; compress_bound(uncompressed.len())];
        let (compressed, status) = compress_slice(
            &mut compressed_buffer,
            uncompressed,
            DeflateConfig::default(),
        );
        assert_eq!(status, ReturnCode::Ok);
        let compressed = compressed.to_vec();

        let mut data = vec![0; compressed_offset];
        data[..MAGIC.len()].copy_from_slice(MAGIC);
        set_u64(&mut data, 0x18, FILETIME);
        set_u64(&mut data, 0x20, FILETIME);
        set_u32(&mut data, 0x28, VERSION);
        set_u32(&mut data, INFO_OFFSET_OFFSET, INFO_OFFSET as u32);
        set_u32(&mut data, INFO_SIZE_OFFSET, info.len() as u32);
        set_u32(
            &mut data,
            COMPRESSED_OFFSET_OFFSET,
            compressed_offset as u32,
        );
        set_u32(&mut data, COMPRESSED_SIZE_OFFSET, compressed.len() as u32);
        set_u32(
            &mut data,
            DECOMPRESSED_SIZE_OFFSET,
            uncompressed.len() as u32,
        );
        set_u32(&mut data, ENTRY_COUNT_OFFSET, entry_count as u32);
        data[INFO_OFFSET..compressed_offset].copy_from_slice(&info);
        data.extend_from_slice(&compressed);
        data
    }

    fn encode_entries(entries: &[RawEntry]) -> Vec<u8> {
        let index_size = entries.len() * INDEX_RECORD_LEN;
        let mut index = vec![0; index_size];
        let mut payload = Vec::new();

        for (position, entry) in entries.iter().enumerate() {
            assert!(entry.code.len() <= usize::from(u8::MAX));
            assert!(entry.word.len() <= usize::from(u8::MAX));

            let record_offset = position * INDEX_RECORD_LEN;
            let payload_offset = index_size + payload.len();
            index[record_offset] = entry.code.len() as u8;
            index[record_offset + 1] = entry.word.len() as u8;
            index[record_offset + 2..record_offset + 6].copy_from_slice(&[0, 0, 0x80, 0x3f]);
            set_u32(&mut index, record_offset + 6, payload_offset as u32);

            payload.extend_from_slice(&entry.code);
            payload.extend_from_slice(&entry.word);
        }

        index.extend_from_slice(&payload);
        index
    }

    fn entry(code: &str, word: &str) -> RawEntry {
        RawEntry {
            code: code.as_bytes().to_vec(),
            word: utf16_bytes(word),
        }
    }

    fn utf16_bytes(value: &str) -> Vec<u8> {
        value.encode_utf16().flat_map(u16::to_le_bytes).collect()
    }

    fn u32_at(data: &[u8], offset: usize) -> u32 {
        u32::from_le_bytes([
            data[offset],
            data[offset + 1],
            data[offset + 2],
            data[offset + 3],
        ])
    }

    fn set_u32(data: &mut [u8], offset: usize, value: u32) {
        data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn set_u64(data: &mut [u8], offset: usize, value: u64) {
        data[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }
}
