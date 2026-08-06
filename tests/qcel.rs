//! Integration tests against real QQ Pinyin Cell Dictionary files.

mod support;

use cidian::{Entry, qcel};
use support::{assert_entry, parse_for_test, read_fixture};

// These properties were independently checked by following the byte counts,
// pinyin indices, UTF-16LE fields, and u32 extensions with Python's standard
// library. Do not regenerate them from `cidian` alone without review.
macro_rules! qcel_golden_test {
    (
        $test_name:ident,
        $file_name:literal,
        entries = $entries:expr,
        name = $name:literal,
        category = $category:literal,
        description = $description:literal,
        first = ($first_word:literal, $first_code:expr, $first_weight:expr),
        middle = ($middle_index:expr, $middle_word:literal, $middle_code:expr, $middle_weight:expr),
        last = ($last_word:literal, $last_code:expr, $last_weight:expr),
    ) => {
        #[test]
        fn $test_name() {
            let file_name = $file_name;
            let data = read_fixture("qcel", file_name);
            let dictionary = parse_for_test(file_name, &data, qcel::parse);

            assert_eq!(dictionary.entries.len(), $entries);
            assert_eq!(dictionary.metadata.name.as_deref(), Some($name));
            assert_eq!(dictionary.metadata.category.as_deref(), Some($category));
            assert_eq!(
                dictionary.metadata.description.as_deref(),
                Some($description)
            );

            for (index, entry) in dictionary.entries.iter().enumerate() {
                assert!(
                    !entry.word.is_empty(),
                    "{file_name}: entry {index} has an empty word"
                );
                assert!(
                    !entry.code.is_empty(),
                    "{file_name}: entry {index} has an empty code"
                );
                assert!(
                    entry.code.iter().all(|component| !component.is_empty()),
                    "{file_name}: entry {index} has an empty code component"
                );
                assert!(
                    entry.weight.is_some(),
                    "{file_name}: entry {index} has no weight"
                );
            }

            assert_qcel_entry(
                file_name,
                0,
                &dictionary.entries[0],
                $first_word,
                $first_code,
                $first_weight,
            );
            assert_qcel_entry(
                file_name,
                $middle_index,
                &dictionary.entries[$middle_index],
                $middle_word,
                $middle_code,
                $middle_weight,
            );
            let last = dictionary.entries.len() - 1;
            assert_qcel_entry(
                file_name,
                last,
                &dictionary.entries[last],
                $last_word,
                $last_code,
                $last_weight,
            );
        }
    };
}

#[track_caller]
fn assert_qcel_entry(
    file_name: &str,
    index: usize,
    entry: &Entry,
    word: &str,
    code: &[&str],
    weight: u32,
) {
    assert_entry(file_name, index, entry, word, code);
    assert_eq!(
        entry.weight,
        Some(weight),
        "{file_name}: weight at entry {index}"
    );
}

qcel_golden_test!(
    parses_idiom_dictionary,
    "成语.qcel",
    entries = 1732,
    name = "成语",
    category = "人文",
    description = "成语",
    first = ("阿保之功", &["a", "bao", "zhi", "gong"], 1732),
    middle = (866, "深入细致", &["shen", "ru", "xi", "zhi"], 1732),
    last = ("葄枕图史", &["zuo", "zhen", "tu", "shi"], 1732),
);

qcel_golden_test!(
    parses_idioms_and_sayings_dictionary,
    "成语俗语大全.qcel",
    entries = 66418,
    name = "成语俗语大全",
    category = "成语",
    description = "对于之前发布的成语和俗语的一次整合",
    first = ("啊啊啊", &["a", "a", "a"], 35306),
    middle = (33209, "秘银罩帽", &["mi", "yin", "zhao", "mao"], 9074),
    last = ("作作有芒", &["zuo", "zuo", "you", "mang"], 48819),
);

qcel_golden_test!(
    parses_internet_language_dictionary,
    "网络流行新词.qcel",
    entries = 37494,
    name = "网络流行新词【官方推荐】",
    category = "北京",
    description = "搜狗搜索自动生成的流行新词，每周更新。",
    first = ("阿敖", &["a", "ao"], 20258),
    middle = (18747, "米麻薯", &["mi", "ma", "shu"], 16565),
    last = ("做桌", &["zuo", "zhuo"], 28226),
);

qcel_golden_test!(
    parses_medicine_dictionary,
    "药品名称大全.qcel",
    entries = 36180,
    name = "药品名称大全",
    category = "西药学",
    description = "药品名称欢迎使用",
    first = ("阿巴卡韦", &["a", "ba", "ka", "wei"], 36109),
    middle = (
        18090,
        "吗拉胺中间体",
        &["ma", "la", "an", "zhong", "jian", "ti"],
        10538
    ),
    last = ("坐珠达西丸", &["zuo", "zhu", "da", "xi", "wan"], 4098),
);

qcel_golden_test!(
    parses_computer_dictionary,
    "计算机科技.qcel",
    entries = 9646,
    name = "计算机名词",
    category = "计算机科技",
    description = "计算机专业词库",
    first = ("阿里通", &["a", "li", "tong"], 8),
    middle = (4823, "模数转换", &["mo", "shu", "zhuan", "huan"], 5064),
    last = ("作用域", &["zuo", "yong", "yu"], 387),
);

#[test]
fn preserves_weights_larger_than_u16() {
    let file_name = "成语俗语大全.qcel";
    let data = read_fixture("qcel", file_name);
    let dictionary = parse_for_test(file_name, &data, qcel::parse);

    assert_qcel_entry(
        file_name,
        4,
        &dictionary.entries[4],
        "阿鼻地狱",
        &["a", "bi", "di", "yu"],
        66346,
    );
    assert_qcel_entry(
        file_name,
        58078,
        &dictionary.entries[58078],
        "一年被蛇咬三年怕草索",
        &[
            "yi", "nian", "bei", "she", "yao", "san", "nian", "pa", "cao", "suo",
        ],
        u32::MAX,
    );
}
