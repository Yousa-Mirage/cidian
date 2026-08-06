//! Integration tests against real Baidu category dictionaries (.bcd and .bdict).

mod support;

use cidian::{Entry, bcd, bdict};
use support::{assert_entry, parse_for_test, read_fixture};

macro_rules! baidu_golden_test {
    (
        $test_name:ident,
        parser = $parser:path,
        file_name = $file_name:literal,
        entries = $entries:expr,
        empty_words = $empty_words:expr,
        missing_weights = $missing_weights:expr,
        name = $name:literal,
        author = $author:literal,
        example = $example:literal,
        description = $description:literal,
        first = ($first_word:literal, $first_code:expr, $first_weight:expr),
        middle = ($middle_index:expr, $middle_word:literal, $middle_code:expr, $middle_weight:expr),
        last = ($last_word:literal, $last_code:expr, $last_weight:expr),
    ) => {
        #[test]
        fn $test_name() {
            let file_name = $file_name;
            let data = read_fixture("baidu", file_name);
            let dictionary = parse_for_test(file_name, &data, $parser);

            assert_eq!(dictionary.entries.len(), $entries);
            assert_eq!(dictionary.metadata.name.as_deref(), Some($name));
            assert_eq!(dictionary.metadata.category, None);
            assert_eq!(
                dictionary.metadata.description.as_deref(),
                Some($description)
            );
            assert_eq!(
                dictionary.metadata.extra.get("author").map(String::as_str),
                Some($author)
            );
            assert_eq!(
                dictionary.metadata.extra.get("example").map(String::as_str),
                Some($example)
            );

            assert_eq!(
                dictionary
                    .entries
                    .iter()
                    .filter(|entry| entry.word.is_empty())
                    .count(),
                $empty_words,
                "{file_name}: unexpected number of empty source words"
            );

            assert_eq!(
                dictionary
                    .entries
                    .iter()
                    .filter(|entry| entry.weight.is_none())
                    .count(),
                $missing_weights,
                "{file_name}: unexpected number of entries without weight"
            );

            for (index, entry) in dictionary.entries.iter().enumerate() {
                assert!(
                    !entry.code.is_empty(),
                    "{file_name}: entry {index} has an empty code"
                );
                assert!(
                    entry.code.iter().all(|component| !component.is_empty()),
                    "{file_name}: entry {index} has an empty code component"
                );
            }

            assert_baidu_entry(
                file_name,
                0,
                &dictionary.entries[0],
                $first_word,
                $first_code,
                $first_weight,
            );
            assert_baidu_entry(
                file_name,
                $middle_index,
                &dictionary.entries[$middle_index],
                $middle_word,
                $middle_code,
                $middle_weight,
            );
            let last = dictionary.entries.len() - 1;
            assert_baidu_entry(
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
fn assert_baidu_entry(
    file_name: &str,
    index: usize,
    entry: &Entry,
    word: &str,
    code: &[&str],
    weight: Option<u32>,
) {
    assert_entry(file_name, index, entry, word, code);
    assert_eq!(entry.weight, weight, "{file_name}: weight at entry {index}");
}

baidu_golden_test!(
    parses_bcd_idioms_and_sayings_dictionary,
    parser = bcd::parse,
    file_name = "俗语成语.bcd",
    entries = 22443,
    empty_words = 0,
    missing_weights = 0,
    name = "俗语成语",
    author = "百度",
    example = "七足八手,单鹄寡凫,绿衣黄里",
    description = "2016年8月18日更新。",
    first = ("七足八手", &["qi", "zu", "ba", "shou"], Some(1000)),
    middle = (
        11221,
        "德言容功",
        &["de", "yan", "rong", "gong"],
        Some(1000)
    ),
    last = ("一推六二五", &["yi", "tui", "liu", "er", "wu"], Some(1000)),
);

baidu_golden_test!(
    parses_bcd_historical_figures_dictionary,
    parser = bcd::parse,
    file_name = "历史人物.bcd",
    entries = 16207,
    empty_words = 0,
    missing_weights = 0,
    name = "历史人物",
    author = "百度",
    example = "文家市战斗,长汀战斗,骨酥鱼历史",
    description = "2016年8月18日更新。",
    first = ("文家市战斗", &["wen", "jia", "shi", "zhan", "dou"], Some(0)),
    middle = (8103, "萧讹都斡", &["xiao", "e", "dou", "wo"], Some(1000)),
    last = ("秦孝文王", &["qin", "xiao", "wen", "wang"], Some(1000)),
);

baidu_golden_test!(
    parses_bcd_prefecture_regions_dictionary,
    parser = bcd::parse,
    file_name = "地级行政区域.bcd",
    entries = 75,
    empty_words = 0,
    missing_weights = 0,
    name = "地级行政区域",
    author = "百度",
    example = "东莞市，铜陵市，衢州市",
    description = "2013年3月20日更新。",
    first = ("阿勒泰地区", &["a", "le", "tai", "di", "qu"], Some(0)),
    middle = (37, "铜仁地区", &["tong", "ren", "di", "qu"], Some(0)),
    last = ("六盘水市", &["liu", "pan", "shui", "shi"], Some(0)),
);

baidu_golden_test!(
    parses_bcd_chat_phrases_dictionary,
    parser = bcd::parse,
    file_name = "常用聊天短语.bcd",
    entries = 4907,
    empty_words = 0,
    missing_weights = 0,
    name = "常用聊天短语",
    author = "指尖先锋",
    example = "保持联络，真的好开心啊，你我心照不宣了",
    description = "2013年3月21日更新。",
    first = ("安静点", &["an", "jing", "dian"], Some(10000)),
    middle = (2453, "你也知道", &["ni", "ye", "zhi", "dao"], Some(10000)),
    last = (
        "昨天去玩了吗",
        &["zuo", "tian", "qu", "wan", "le", "ma"],
        Some(10000)
    ),
);

baidu_golden_test!(
    parses_bcd_daily_language_dictionary,
    parser = bcd::parse,
    file_name = "日常用语大词库.bcd",
    entries = 38500,
    empty_words = 0,
    missing_weights = 0,
    name = "日常用语大词库",
    author = "百度",
    example = "还不睡觉啊，真高兴，那很好",
    description = "2013年3月20日更新。",
    first = (
        "还不睡觉啊",
        &["hai", "bu", "shui", "jiao", "a"],
        Some(3869)
    ),
    middle = (19250, "论是", &["lun", "shi"], Some(72)),
    last = ("日耳曼", &["ri", "er", "man"], Some(35)),
);

baidu_golden_test!(
    parses_bdict_animation_dictionary,
    parser = bdict::parse,
    file_name = "动漫作品词库.bdict",
    entries = 86064,
    empty_words = 42,
    missing_weights = 2597,
    name = "动漫作品词库",
    author = "admin",
    example = "娱乐休闲",
    description = "经典动漫作品词汇",
    first = ("上条刀夜", &["shang", "tiao", "dao", "ye"], Some(0)),
    middle = (43032, "泽北荣治", &["ze", "bei", "rong", "zhi"], Some(0)),
    last = ("俘虏Darling", &["fu'lu'Darling"], None),
);

baidu_golden_test!(
    parses_bdict_idiom_dictionary,
    parser = bdict::parse,
    file_name = "成语大全.bdict",
    entries = 74392,
    empty_words = 17,
    missing_weights = 100,
    name = "成语大全",
    author = "admin",
    example = "人文社会",
    description = "常用成语大全",
    first = ("迫于眉睫", &["po", "yu", "mei", "jie"], Some(0)),
    middle = (37196, "恃才放旷", &["shi", "cai", "fang", "kuang"], Some(0)),
    last = ("风饕雪虐", &["feng'tao'xue'nve"], None),
);

baidu_golden_test!(
    parses_bdict_computer_games_dictionary,
    parser = bdict::parse,
    file_name = "电脑游戏词汇.bdict",
    entries = 6246,
    empty_words = 0,
    missing_weights = 0,
    name = "电脑游戏词汇",
    author = "百度输入法",
    example = "游戏,电脑游戏词汇",
    description = "经典电脑游戏名称及人物、技能、场景名名",
    first = ("九彩云龙珠", &["jiu", "cai", "yun", "long", "zhu"], Some(0)),
    middle = (3123, "嗜血蜂后", &["shi", "xue", "feng", "hou"], Some(0)),
    last = ("QQ好友买卖", &["qqhaoyoumaimai"], Some(0)),
);

baidu_golden_test!(
    parses_bdict_internet_language_dictionary,
    parser = bdict::parse,
    file_name = "网络用语.bdict",
    entries = 5977,
    empty_words = 0,
    missing_weights = 9,
    name = "网络用语",
    author = "admin",
    example = "生活百科\r\n",
    description = "网络流行语",
    first = ("成龙", &["cheng", "long"], Some(0)),
    middle = (2988, "你傻了啊", &["ni", "sha", "le", "a"], Some(0)),
    last = (
        "zheyangzihaonankanV这样子漂亮吗",
        &["zhe'yang'zi'piao'liang'ma"],
        None
    ),
);

baidu_golden_test!(
    parses_bdict_selected_poetry_dictionary,
    parser = bdict::parse,
    file_name = "诗词精选.bdict",
    entries = 100264,
    empty_words = 29,
    missing_weights = 924,
    name = "诗词精选",
    author = "admin",
    example = "学术教育\r\n",
    description = "古诗词经典名句精选",
    first = (
        "吾日三省吾身",
        &["wu", "ri", "san", "xing", "wu", "shen"],
        Some(0)
    ),
    middle = (50132, "何情不诉", &["he", "qing", "bu", "su"], Some(0)),
    last = (
        "huzhoubowenyingkezhuo虎落平川被犬欺",
        &["hu'luo'ping'chuan'bei'quan'qi"],
        None
    ),
);

#[test]
fn parses_bdict_english_and_both_mixed_record_layouts() {
    let file_name = "电脑游戏词汇.bdict";
    let data = read_fixture("baidu", file_name);
    let dictionary = parse_for_test(file_name, &data, bdict::parse);

    assert_baidu_entry(
        file_name,
        6229,
        &dictionary.entries[6229],
        "BioShock",
        &["BioShock"],
        Some(0),
    );
    assert_baidu_entry(
        file_name,
        6237,
        &dictionary.entries[6237],
        "剑网2",
        &["jianwang"],
        Some(0),
    );

    let animation_file = "动漫作品词库.bdict";
    let animation_data = read_fixture("baidu", animation_file);
    let animation = parse_for_test(animation_file, &animation_data, bdict::parse);
    assert_baidu_entry(
        animation_file,
        82094,
        &animation.entries[82094],
        "Trash黑街杀手",
        &["Trash'hei'jie'sha'shou"],
        None,
    );
}
