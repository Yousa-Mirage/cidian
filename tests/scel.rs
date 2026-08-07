//! Integration tests against real Sogou Cell Dictionary files.

mod support;

use cidian::scel;
use support::{assert_entry, parse_for_test, read_fixture};

// These are golden properties of the exact fixture bytes checked into this
// repository. They were cross-checked with an independent reference parser;
// do not regenerate them from `cidian-rs` alone without reviewing the result.
macro_rules! scel_golden_test {
    (
        $test_name:ident,
        $file_name:literal,
        entries = $entries:expr,
        name = $name:literal,
        category = $category:literal,
        first = ($first_word:literal, $first_code:expr),
        middle = ($middle_index:expr, $middle_word:literal, $middle_code:expr),
        last = ($last_word:literal, $last_code:expr),
    ) => {
        #[test]
        fn $test_name() {
            let file_name = $file_name;
            let data = read_fixture("scel", file_name);
            let dictionary = parse_for_test(file_name, &data, scel::parse);

            assert_eq!(dictionary.entries.len(), $entries);
            assert_eq!(dictionary.metadata.name.as_deref(), Some($name));
            assert_eq!(dictionary.metadata.category.as_deref(), Some($category));

            for (index, entry) in dictionary.entries.iter().enumerate() {
                assert!(
                    !entry.word.is_empty(),
                    "{file_name}: entry {index} has an empty word"
                );
                assert!(
                    !entry.code.is_empty(),
                    "{file_name}: entry {index} has an empty code"
                );
            }

            assert_entry(
                file_name,
                0,
                &dictionary.entries[0],
                $first_word,
                $first_code,
            );
            assert_entry(
                file_name,
                $middle_index,
                &dictionary.entries[$middle_index],
                $middle_word,
                $middle_code,
            );
            let last = dictionary.entries.len() - 1;
            assert_entry(
                file_name,
                last,
                &dictionary.entries[last],
                $last_word,
                $last_code,
            );
        }
    };
}

scel_golden_test!(
    parses_agriculture_dictionary,
    "农业词汇大全.scel",
    entries = 8874,
    name = "农业词汇大全【官方推荐】",
    category = "农业",
    first = ("阿尔泰狗哇花", &["a", "er", "tai", "gou", "wa", "hua"]),
    middle = (4437, "面粉处理", &["mian", "fen", "chu", "li"]),
    last = ("作种", &["zuo", "zhong"]),
);
scel_golden_test!(
    parses_animals_dictionary,
    "动物词汇大全.scel",
    entries = 37092,
    name = "动物词汇大全【官方推荐】",
    category = "动物",
    first = ("阿比西尼亚猫", &["a", "bi", "xi", "ni", "ya", "mao"]),
    middle = (18546, "眶管鰕虎鱼", &["kuang", "guan", "xia", "hu", "yu"]),
    last = ("钻菱鲷", &["zuan", "ling", "diao"]),
);
scel_golden_test!(
    parses_medicine_dictionary,
    "医学词汇大全.scel",
    entries = 90047,
    name = "医学词汇大全【官方推荐】",
    category = "基础医学",
    first = ("阿埃二氏病变", &["a", "ai", "er", "shi", "bing", "bian"]),
    middle = (
        45023,
        "龙虾钳状手",
        &["long", "xia", "qian", "zhuang", "shou"]
    ),
    last = ("左主支气管", &["zuo", "zhu", "zhi", "qi", "guan"]),
);
scel_golden_test!(
    parses_idioms_dictionary,
    "成语俗语.scel",
    entries = 46785,
    name = "成语俗语【官方推荐】",
    category = "成语",
    first = ("阿保之功", &["a", "bao", "zhi", "gong"]),
    middle = (23392, "流风遗俗", &["liu", "feng", "yi", "su"]),
    last = ("作作有芒", &["zuo", "zuo", "you", "mang"]),
);
scel_golden_test!(
    parses_government_dictionary,
    "政府机关团体机构大全.scel",
    entries = 29764,
    name = "政府机关团体机构大全【官方推荐】",
    category = "单位机构名",
    first = ("阿巴嘎旗政府", &["a", "ba", "ga", "qi", "zheng", "fu"]),
    middle = (14882, "蒙阴县委", &["meng", "yin", "xian", "wei"]),
    last = ("左云县公安局", &["zuo", "yun", "xian", "gong", "an", "ju"]),
);
scel_golden_test!(
    parses_fantasy_game_dictionary,
    "梦幻西游.scel",
    entries = 6586,
    name = "梦幻西游【官方推荐】",
    category = "网页游戏",
    first = ("阿德里奥", &["a", "de", "li", "ao"]),
    middle = (3293, "马猴", &["ma", "hou"]),
    last = ("左眼", &["zuo", "yan"]),
);
scel_golden_test!(
    parses_automobile_dictionary,
    "汽车词汇大全.scel",
    entries = 2401,
    name = "汽车词汇大全【官方推荐】",
    category = "汽车",
    first = ("阿蒂玛", &["a", "di", "ma"]),
    middle = (1200, "摩托车配件", &["mo", "tuo", "che", "pei", "jian"]),
    last = ("左转向开关", &["zuo", "zhuan", "xiang", "kai", "guan"]),
);
scel_golden_test!(
    parses_law_dictionary,
    "法律词汇大全.scel",
    entries = 4560,
    name = "法律词汇大全【官方推荐】",
    category = "法律",
    first = ("阿奎那", &["a", "kui", "na"]),
    middle = (2280, "罗马法系", &["luo", "ma", "fa", "xi"]),
    last = (
        "作为证据使用",
        &["zuo", "wei", "zheng", "ju", "shi", "yong"]
    ),
);
scel_golden_test!(
    parses_painting_dictionary,
    "绘画美术词汇大全.scel",
    entries = 6317,
    name = "绘画美术词汇大全【官方推荐】",
    category = "绘画",
    first = ("阿嘉娜", &["a", "jia", "na"]),
    middle = (3158, "启朝荣", &["qi", "chao", "rong"]),
    last = ("左右均齐", &["zuo", "you", "jun", "qi"]),
);
scel_golden_test!(
    parses_computer_dictionary,
    "计算机词汇大全.scel",
    entries = 10300,
    name = "计算机词汇大全【官方推荐】",
    category = "计算机科技",
    first = ("阿姆达尔定律", &["a", "mu", "da", "er", "ding", "lv"]),
    middle = (5150, "逆变器", &["ni", "bian", "qi"]),
    last = ("作用域解析", &["zuo", "yong", "yu", "jie", "xi"]),
);
scel_golden_test!(
    parses_chess_dictionary,
    "象棋.scel",
    entries = 1772,
    name = "象棋【官方推荐】",
    category = "象棋",
    first = ("暗根子", &["an", "gen", "zi"]),
    middle = (886, "摸子走子", &["mo", "zi", "zou", "zi"]),
    last = ("左中炮", &["zuo", "zhong", "pao"]),
);
scel_golden_test!(
    parses_food_dictionary,
    "饮食大全.scel",
    entries = 6918,
    name = "饮食大全【官方推荐】",
    category = "饮食",
    first = ("阿拉伯胶", &["a", "la", "bo", "jiao"]),
    middle = (
        3459,
        "吗啉脂肪酸盐果蜡",
        &["ma", "lin", "zhi", "fang", "suan", "yan", "guo", "la"]
    ),
    last = ("遵义毛峰", &["zun", "yi", "mao", "feng"]),
);
