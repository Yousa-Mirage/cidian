//! Integration tests against real QQ Pinyin category dictionary files.

mod support;

use cidian::qpyd;
use support::{assert_entry, parse_for_test, read_fixture};

// These properties were independently cross-checked by decoding the zlib
// stream and index records with Python's standard library. Do not regenerate
// them from `cidian` alone without reviewing the result.
macro_rules! qpyd_golden_test {
    (
        $test_name:ident,
        $file_name:literal,
        entries = $entries:expr,
        name = $name:literal,
        category = $category:literal,
        first_type = $first_type:literal,
        version = $version:literal,
        first = ($first_word:literal, $first_code:expr),
        middle = ($middle_index:expr, $middle_word:literal, $middle_code:expr),
        last = ($last_word:literal, $last_code:expr),
    ) => {
        #[test]
        fn $test_name() {
            let file_name = $file_name;
            let data = read_fixture("qpyd", file_name);
            let dictionary = parse_for_test(file_name, &data, qpyd::parse);

            assert_eq!(dictionary.entries.len(), $entries);
            assert_eq!(dictionary.metadata.name.as_deref(), Some($name));
            assert_eq!(dictionary.metadata.category.as_deref(), Some($category));
            assert_eq!(
                dictionary
                    .metadata
                    .extra
                    .get("first_type")
                    .map(String::as_str),
                Some($first_type)
            );
            assert_eq!(
                dictionary.metadata.extra.get("version").map(String::as_str),
                Some($version)
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
                assert_eq!(
                    entry.weight, None,
                    "{file_name}: entry {index} unexpectedly has a weight"
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

qpyd_golden_test!(
    parses_lol_dictionary,
    "LOL.qpyd",
    entries = 2357,
    name = "LOL",
    category = "网络游戏",
    first_type = "电子游戏",
    version = "62",
    first = ("阿尔法突袭", &["a", "er", "fa", "tu", "xi"]),
    middle = (1178, "狂暴撕咬", &["kuang", "bao", "si", "yao"]),
    last = ("魅惑妖术", &["mei", "huo", "yao", "shu"]),
);

qpyd_golden_test!(
    parses_classical_poetry_dictionary,
    "古诗词.qpyd",
    entries = 18290,
    name = "古诗词",
    category = "人文",
    first_type = "人文社科",
    version = "7",
    first = ("刬却君山好", &["chan", "que", "jun", "shan", "hao"]),
    middle = (9145, "驱马悠悠", &["qu", "ma", "you", "you"]),
    last = ("辗转念前途", &["zhan", "zhuan", "nian", "qian", "tu"]),
);

qpyd_golden_test!(
    parses_tang_poetry_dictionary,
    "唐诗.qpyd",
    entries = 161674,
    name = "唐诗",
    category = "人文",
    first_type = "人文社科",
    version = "4",
    first = (
        "丱岁便将为肘腋",
        &["guan", "sui", "bian", "jiang", "wei", "zhou", "ye"]
    ),
    middle = (80837, "岂若归吾庐", &["qi", "ruo", "gui", "wu", "lu"]),
    last = ("麹氏雄西北", &["qu", "shi", "xiong", "xi", "bei"]),
);

qpyd_golden_test!(
    parses_dungeon_fighter_dictionary,
    "地下城与勇士.qpyd",
    entries = 5543,
    name = "地下城与勇士",
    category = "网络游戏",
    first_type = "电子游戏",
    version = "76",
    first = ("阿克雄", &["a", "ke", "xiong"]),
    middle = (
        2771,
        "拉格纳罗斯之拳",
        &["la", "ge", "na", "luo", "si", "zhi", "quan"]
    ),
    last = (
        "最高生命值榜",
        &["zui", "gao", "sheng", "ming", "zhi", "bang"]
    ),
);

qpyd_golden_test!(
    parses_chat_phrases_dictionary,
    "常用聊天短语.qpyd",
    entries = 4538,
    name = "常用聊天短语",
    category = "日常",
    first_type = "兴趣爱好",
    version = "60",
    first = ("我想静静", &["wo", "xiang", "jing", "jing"]),
    middle = (
        2269,
        "你依然是没变",
        &["ni", "yi", "ran", "shi", "mei", "bian"]
    ),
    last = ("怎么不接电话", &["zen", "me", "bu", "jie", "dian", "hua"]),
);

qpyd_golden_test!(
    parses_idiom_dictionary,
    "成语.qpyd",
    entries = 1732,
    name = "成语",
    category = "人文",
    first_type = "人文社科",
    version = "7",
    first = ("僾见忾闻", &["ai", "jian", "kai", "wen"]),
    middle = (866, "神算妙计", &["shen", "suan", "miao", "ji"]),
    last = ("黯然无神", &["an", "ran", "wu", "shen"]),
);

qpyd_golden_test!(
    parses_idioms_and_sayings_dictionary,
    "成语俗语.qpyd",
    entries = 13527,
    name = "成语俗语",
    category = "人文",
    first_type = "人文社科",
    version = "5",
    first = ("佹得佹失", &["gui", "de", "gui", "shi"]),
    middle = (6763, "蒙袂辑屦", &["meng", "mei", "ji", "ju"]),
    last = ("鼾声如雷", &["han", "sheng", "ru", "lei"]),
);

qpyd_golden_test!(
    parses_movie_titles_dictionary,
    "电影名称.qpyd",
    entries = 61195,
    name = "电影名称",
    category = "影视",
    first_type = "休闲娱乐",
    version = "66",
    first = ("囧蛋奇兵", &["jiong", "dan", "qi", "bing"]),
    middle = (30597, "落阳", &["luo", "yang"]),
    last = ("季春奶奶", &["ji", "chun", "nai", "nai"]),
);

qpyd_golden_test!(
    parses_internet_language_dictionary,
    "网络用语.qpyd",
    entries = 24115,
    name = "网络用语",
    category = "日常",
    first_type = "兴趣爱好",
    version = "94",
    first = ("次奥", &["ci", "ao"]),
    middle = (12057, "草莓大会", &["cao", "mei", "da", "hui"]),
    last = ("我选择狗带", &["wo", "xuan", "ze", "gou", "dai"]),
);

qpyd_golden_test!(
    parses_world_of_warcraft_dictionary,
    "魔兽世界.qpyd",
    entries = 4057,
    name = "魔兽世界",
    category = "网络游戏",
    first_type = "电子游戏",
    version = "50",
    first = ("埃斯顿", &["ai", "si", "dun"]),
    middle = (2028, "龙建股份", &["long", "jian", "gu", "fen"]),
    last = ("佐力药业", &["zuo", "li", "yao", "ye"]),
);
