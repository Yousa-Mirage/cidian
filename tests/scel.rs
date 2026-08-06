//! Integration tests against real Sogou Cell Dictionary files.

use cidian::scel;

fn assert_parses_dictionary(file_name: &str, data: &[u8]) {
    let dictionary = match scel::parse(data) {
        Ok(dictionary) => dictionary,
        Err(error) => panic!("failed to parse {file_name}: {error}"),
    };

    assert!(
        !dictionary.entries.is_empty(),
        "{file_name} contains no dictionary entries"
    );

    for entry in &dictionary.entries {
        assert!(!entry.word.is_empty(), "{file_name} contains an empty word");
        assert!(
            !entry.code.is_empty(),
            "{file_name} contains an entry without a code: {}",
            entry.word
        );
    }
}

macro_rules! parses_scel_fixture {
    ($test_name:ident, $file_name:literal) => {
        #[test]
        fn $test_name() {
            let data = include_bytes!(concat!("fixtures/scel/", $file_name));
            assert_parses_dictionary($file_name, data);
        }
    };
}

parses_scel_fixture!(parses_agriculture_dictionary, "农业词汇大全.scel");
parses_scel_fixture!(parses_animals_dictionary, "动物词汇大全.scel");
parses_scel_fixture!(parses_medicine_dictionary, "医学词汇大全.scel");
parses_scel_fixture!(parses_idioms_dictionary, "成语俗语.scel");
parses_scel_fixture!(parses_government_dictionary, "政府机关团体机构大全.scel");
parses_scel_fixture!(parses_fantasy_game_dictionary, "梦幻西游.scel");
parses_scel_fixture!(parses_automobile_dictionary, "汽车词汇大全.scel");
parses_scel_fixture!(parses_law_dictionary, "法律词汇大全.scel");
parses_scel_fixture!(parses_painting_dictionary, "绘画美术词汇大全.scel");
parses_scel_fixture!(parses_computer_dictionary, "计算机词汇大全.scel");
parses_scel_fixture!(parses_chess_dictionary, "象棋.scel");
parses_scel_fixture!(parses_food_dictionary, "饮食大全.scel");
