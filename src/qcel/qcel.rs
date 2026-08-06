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
const PINYIN_TABLE_OFFSET: usize = 0x1544;
const ENGLISH_CODE_COUNT: u32 = 26;
const HASH_CODE_INDEX: u32 = 482;
const QQ_VARIANT: [u8; 3] = [0xd2, 0x6d, 0x53];
const FORMAT: Format = Format::Qcel;

enum CodeTable {
    Default,
    Explicit {
        codes: Vec<Option<String>>,
        declared_count: u32,
    },
}

// QCEL files may omit their pinyin table. This is the default table used by
// QQ Pinyin and by rose for that layout.
const DEFAULT_CODES: [&str; 449] = [
    "a", "ai", "an", "ang", "ao", "ba", "bai", "ban", "bang", "bao", "bei", "ben", "beng", "bi",
    "bian", "biao", "bie", "bin", "bing", "bo", "bu", "ca", "cai", "can", "cang", "cao", "ce",
    "cen", "ceng", "cha", "chai", "chan", "chang", "chao", "che", "chen", "cheng", "chi", "chong",
    "chou", "chu", "chua", "chuai", "chuan", "chuang", "chui", "chun", "chuo", "ci", "cong", "cou",
    "cu", "cuan", "cui", "cun", "cuo", "da", "dai", "dan", "dang", "dao", "de", "dei", "den",
    "deng", "di", "dia", "dian", "diao", "die", "ding", "diu", "dong", "dou", "du", "duan", "dui",
    "dun", "duo", "e", "ei", "en", "eng", "er", "fa", "fan", "fang", "fei", "fen", "feng", "fiao",
    "fo", "fou", "fu", "ga", "gai", "gan", "gang", "gao", "ge", "gei", "gen", "geng", "gong",
    "gou", "gu", "gua", "guai", "guan", "guang", "gui", "gun", "guo", "ha", "hai", "han", "hang",
    "hao", "he", "hei", "hen", "heng", "hong", "hou", "hu", "hua", "huai", "huan", "huang", "hui",
    "hun", "huo", "ji", "jia", "jian", "jiang", "jiao", "jie", "jin", "jing", "jiong", "jiu", "ju",
    "juan", "jue", "jun", "ka", "kai", "kan", "kang", "kao", "ke", "kei", "ken", "keng", "kong",
    "kou", "ku", "kua", "kuai", "kuan", "kuang", "kui", "kun", "kuo", "la", "lai", "lan", "lang",
    "lao", "le", "lei", "leng", "li", "lia", "lian", "liang", "liao", "lie", "lin", "ling", "liu",
    "lo", "long", "lou", "lu", "luan", "lue", "lun", "luo", "lv", "ma", "mai", "man", "mang",
    "mao", "me", "mei", "men", "meng", "mi", "mian", "miao", "mie", "min", "ming", "miu", "mo",
    "mou", "mu", "na", "nai", "nan", "nang", "nao", "ne", "nei", "nen", "neng", "ni", "nian",
    "niang", "niao", "nie", "nin", "ning", "niu", "nong", "nou", "nu", "nuan", "nue", "nun", "nuo",
    "nv", "o", "ou", "pa", "pai", "pan", "pang", "pao", "pei", "pen", "peng", "pi", "pian", "piao",
    "pie", "pin", "ping", "po", "pou", "pu", "qi", "qia", "qian", "qiang", "qiao", "qie", "qin",
    "qing", "qiong", "qiu", "qu", "quan", "que", "qun", "ran", "rang", "rao", "re", "ren", "reng",
    "ri", "rong", "rou", "ru", "rua", "ruan", "rui", "run", "ruo", "sa", "sai", "san", "sang",
    "sao", "se", "sen", "seng", "sha", "shai", "shan", "shang", "shao", "she", "shei", "shen",
    "sheng", "shi", "shou", "shu", "shua", "shuai", "shuan", "shuang", "shui", "shun", "shuo",
    "si", "song", "sou", "su", "suan", "sui", "sun", "suo", "ta", "tai", "tan", "tang", "tao",
    "te", "tei", "teng", "ti", "tian", "tiao", "tie", "ting", "tong", "tou", "tu", "tuan", "tui",
    "tun", "tuo", "wa", "wai", "wan", "wang", "wei", "wen", "weng", "wo", "wu", "xi", "xia",
    "xian", "xiang", "xiao", "xie", "xin", "xing", "xiong", "xiu", "xu", "xuan", "xue", "xun",
    "ya", "yan", "yang", "yao", "ye", "yi", "yin", "ying", "yo", "yong", "you", "yu", "yuan",
    "yue", "yun", "za", "zai", "zan", "zang", "zao", "ze", "zei", "zen", "zeng", "zha", "zhai",
    "zhan", "zhang", "zhao", "zhe", "zhei", "zhen", "zheng", "zhi", "zhong", "zhou", "zhu", "zhua",
    "zhuai", "zhuan", "zhuang", "zhui", "zhun", "zhuo", "zi", "zong", "zou", "zu", "zuan", "zui",
    "zun", "zuo", "A", "B", "C", "D", "E", "F", "G", "H", "I", "J", "K", "L", "M", "N", "O", "P",
    "Q", "R", "S", "T", "U", "V", "W", "X", "Y", "Z", "0", "1", "2", "3", "4", "5", "6", "7", "8",
    "9",
];

/// Parses a QCEL dictionary from memory.
///
/// Entries are returned in source order and are not normalized, sorted, or
/// deduplicated.
///
/// # Examples
///
/// ```no_run
/// let bytes = std::fs::read("dictionary.qcel")?;
/// let dictionary = cidian::qcel::parse(&bytes)?;
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
    let pinyin_count = read_u32_at(data, PINYIN_COUNT_OFFSET, FORMAT)?;

    let mut reader = Reader::at(data, PINYIN_TABLE_OFFSET, FORMAT);
    let code_table = parse_code_table(&mut reader, pinyin_count)?;
    let entries = parse_word_table(&mut reader, record_count, total_words, &code_table)?;

    // QCEL files may append sections such as DELTBL after the declared main
    // word groups. These bytes are not active dictionary entries.
    Ok(Dictionary { metadata, entries })
}

/// Parses a QCEL dictionary file.
///
/// Entries are returned in source order and are not normalized, sorted, or
/// deduplicated.
///
/// # Examples
///
/// ```no_run
/// let dictionary = cidian::qcel::parse_file("dictionary.qcel")?;
/// println!("{} entries", dictionary.entries.len());
/// # Ok::<(), cidian::Error>(())
/// ```
pub fn parse_file(path: impl AsRef<Path>) -> Result<Dictionary> {
    let data = read_file(path, FORMAT)?;
    parse(&data)
}

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
    if variant != *b"DCS" && variant != QQ_VARIANT {
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

fn parse_code_table(reader: &mut Reader<'_>, pinyin_count: u32) -> Result<CodeTable> {
    if pinyin_count == 0 {
        return Ok(CodeTable::Default);
    }

    validate_count(
        "pinyin table",
        u64::from(pinyin_count),
        reader.remaining() / 4,
        FORMAT,
    )?;

    let mut code_table = Vec::new();

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

    Ok(CodeTable::Explicit {
        codes: code_table,
        declared_count: pinyin_count,
    })
}

fn parse_word_table(
    reader: &mut Reader<'_>,
    record_count: u32,
    total_words: u32,
    code_table: &CodeTable,
) -> Result<Vec<Entry>> {
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
                code_table.resolve(index, indices_offset + index_position * 2)
            })
            .collect::<Result<Vec<_>>>()?;

        for word_index in 0..word_count {
            let word_byte_len = reader.read_u16()? as usize;
            let word = reader.read_utf16(word_byte_len, "dictionary word")?;

            let extension_offset = reader.position();
            let extension_len = reader.read_u16()? as usize;
            if extension_len < 4 {
                return Err(Error::InvalidExtensionLength {
                    format: FORMAT,
                    field: "word extension",
                    offset: extension_offset,
                    length: extension_len,
                    minimum: 4,
                });
            }

            let extension = reader.read_bytes(extension_len)?;
            let weight = Some(u32::from_le_bytes([
                extension[0],
                extension[1],
                extension[2],
                extension[3],
            ]));
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

impl CodeTable {
    fn resolve(&self, raw_index: u16, offset: usize) -> Result<String> {
        let index = u32::from(raw_index);

        match self {
            Self::Default => {
                if let Some(code) = DEFAULT_CODES.get(usize::from(raw_index)) {
                    return Ok((*code).to_owned());
                }
            }
            Self::Explicit {
                codes,
                declared_count,
            } => {
                if let Some(code) = codes.get(usize::from(raw_index)).and_then(Option::as_ref) {
                    return Ok(code.clone());
                }

                if index >= *declared_count {
                    let english_offset = index - *declared_count;
                    if english_offset < ENGLISH_CODE_COUNT {
                        return Ok(char::from(b'a' + english_offset as u8).to_string());
                    }
                }
            }
        }

        if index == HASH_CODE_INDEX {
            return Ok("#".to_owned());
        }

        Err(Error::InvalidCodeIndex {
            format: FORMAT,
            index: u64::from(index),
            offset,
        })
    }
}
