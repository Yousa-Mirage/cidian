//! Integration tests against real Sogou Cell Dictionary files.

use cidian::{Dictionary, Entry, scel};

// These are golden properties of the exact fixture bytes checked into this
// repository. They were cross-checked with an independent reference parser;
// do not regenerate them from `cidian` alone without reviewing the result.
macro_rules! parses_scel_fixture {
    (
        $test_name:ident,
        $file_name:literal,
        entries = $entries:expr,
        name = $name:literal,
        category = $category:literal,
        first = ($first_word:literal, $first_code:expr),
        last = ($last_word:literal, $last_code:expr),
    ) => {
        #[test]
        fn $test_name() {
            let file_name = $file_name;
            let data = include_bytes!(concat!("fixtures/scel/", $file_name));
            let dictionary = parse_for_test($file_name, data);

            assert_eq!(dictionary.entries.len(), $entries);
            assert_eq!(dictionary.metadata.name.as_deref(), Some($name));
            assert_eq!(dictionary.metadata.category.as_deref(), Some($category));

            assert!(
                dictionary
                    .entries
                    .iter()
                    .all(|entry| !entry.word.is_empty()),
                "{file_name} contains an empty word"
            );
            assert!(
                dictionary
                    .entries
                    .iter()
                    .all(|entry| !entry.code.is_empty()),
                "{file_name} contains an entry without a code"
            );

            assert_entry(&dictionary.entries[0], $first_word, $first_code);
            let last = dictionary.entries.len() - 1;
            assert_entry(&dictionary.entries[last], $last_word, $last_code);
        }
    };
}

fn parse_for_test(file_name: &str, data: &[u8]) -> Dictionary {
    match scel::parse(data) {
        Ok(dictionary) => dictionary,
        Err(error) => panic!("failed to parse {file_name}: {error}"),
    }
}

fn assert_entry(entry: &Entry, word: &str, code: &[&str]) {
    assert_eq!(entry.word, word);
    assert_eq!(entry.code.as_slice(), code);
}

parses_scel_fixture!(
    parses_agriculture_dictionary,
    "农业词汇大全.scel",
    entries = 8874,
    name = "农业词汇大全【官方推荐】",
    category = "农业",
    first = ("阿尔泰狗哇花", &["a", "er", "tai", "gou", "wa", "hua"]),
    last = ("作种", &["zuo", "zhong"]),
);
parses_scel_fixture!(
    parses_animals_dictionary,
    "动物词汇大全.scel",
    entries = 37092,
    name = "动物词汇大全【官方推荐】",
    category = "动物",
    first = ("阿比西尼亚猫", &["a", "bi", "xi", "ni", "ya", "mao"]),
    last = ("钻菱鲷", &["zuan", "ling", "diao"]),
);
parses_scel_fixture!(
    parses_medicine_dictionary,
    "医学词汇大全.scel",
    entries = 90047,
    name = "医学词汇大全【官方推荐】",
    category = "基础医学",
    first = ("阿埃二氏病变", &["a", "ai", "er", "shi", "bing", "bian"]),
    last = ("左主支气管", &["zuo", "zhu", "zhi", "qi", "guan"]),
);
parses_scel_fixture!(
    parses_idioms_dictionary,
    "成语俗语.scel",
    entries = 46785,
    name = "成语俗语【官方推荐】",
    category = "成语",
    first = ("阿保之功", &["a", "bao", "zhi", "gong"]),
    last = ("作作有芒", &["zuo", "zuo", "you", "mang"]),
);
parses_scel_fixture!(
    parses_government_dictionary,
    "政府机关团体机构大全.scel",
    entries = 29764,
    name = "政府机关团体机构大全【官方推荐】",
    category = "单位机构名",
    first = ("阿巴嘎旗政府", &["a", "ba", "ga", "qi", "zheng", "fu"]),
    last = ("左云县公安局", &["zuo", "yun", "xian", "gong", "an", "ju"]),
);
parses_scel_fixture!(
    parses_fantasy_game_dictionary,
    "梦幻西游.scel",
    entries = 6586,
    name = "梦幻西游【官方推荐】",
    category = "网页游戏",
    first = ("阿德里奥", &["a", "de", "li", "ao"]),
    last = ("左眼", &["zuo", "yan"]),
);
parses_scel_fixture!(
    parses_automobile_dictionary,
    "汽车词汇大全.scel",
    entries = 2401,
    name = "汽车词汇大全【官方推荐】",
    category = "汽车",
    first = ("阿蒂玛", &["a", "di", "ma"]),
    last = ("左转向开关", &["zuo", "zhuan", "xiang", "kai", "guan"]),
);
parses_scel_fixture!(
    parses_law_dictionary,
    "法律词汇大全.scel",
    entries = 4560,
    name = "法律词汇大全【官方推荐】",
    category = "法律",
    first = ("阿奎那", &["a", "kui", "na"]),
    last = (
        "作为证据使用",
        &["zuo", "wei", "zheng", "ju", "shi", "yong"]
    ),
);
parses_scel_fixture!(
    parses_painting_dictionary,
    "绘画美术词汇大全.scel",
    entries = 6317,
    name = "绘画美术词汇大全【官方推荐】",
    category = "绘画",
    first = ("阿嘉娜", &["a", "jia", "na"]),
    last = ("左右均齐", &["zuo", "you", "jun", "qi"]),
);
parses_scel_fixture!(
    parses_computer_dictionary,
    "计算机词汇大全.scel",
    entries = 10300,
    name = "计算机词汇大全【官方推荐】",
    category = "计算机科技",
    first = ("阿姆达尔定律", &["a", "mu", "da", "er", "ding", "lv"]),
    last = ("作用域解析", &["zuo", "yong", "yu", "jie", "xi"]),
);
parses_scel_fixture!(
    parses_chess_dictionary,
    "象棋.scel",
    entries = 1772,
    name = "象棋【官方推荐】",
    category = "象棋",
    first = ("暗根子", &["an", "gen", "zi"]),
    last = ("左中炮", &["zuo", "zhong", "pao"]),
);
parses_scel_fixture!(
    parses_food_dictionary,
    "饮食大全.scel",
    entries = 6918,
    name = "饮食大全【官方推荐】",
    category = "饮食",
    first = ("阿拉伯胶", &["a", "la", "bo", "jiao"]),
    last = ("遵义毛峰", &["zun", "yi", "mao", "feng"]),
);
