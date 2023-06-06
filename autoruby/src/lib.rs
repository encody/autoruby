#![doc = include_str!("../README.md")]
#![warn(clippy::pedantic)]
#![warn(missing_docs)]

use dictionary::Dictionary;

#[cfg(feature = "integrated")]
static DICTIONARY: once_cell::sync::Lazy<Dictionary> = once_cell::sync::Lazy::new(|| {
    let dict_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/dict.bin"));
    let dictionary: Dictionary = bincode::deserialize(dict_bytes).unwrap();

    dictionary
});

pub mod annotate;
pub mod dictionary;
pub mod format;
mod parse;
pub mod select;

#[cfg(all(test, feature = "integrated"))]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::{annotate, format, select};

    struct Test<'a> {
        input: &'a str,
        expected_uncommon_only: &'a str,
        expected_all: &'a str,
    }

    #[test]
    fn conversions() {
        let tests = [
            Test {
                input: "神は「光あれ」と言われた。すると光があった。",
                expected_uncommon_only:
                    "神は「光あれ」と言われた。すると光があった。",
                expected_all:
                    "[神]{かみ}は「[光]{ひかり}あれ」と[言]{い}われた。すると[光]{ひかり}があった。",
            },
            Test {
                input: "全単射。",
                expected_uncommon_only: "[全]{ぜん}[単]{たん}[射]{しゃ}。",
                expected_all: "[全]{ぜん}[単]{たん}[射]{しゃ}。",
            },
            Test {
                input: "計算機科学における継続とは、プログラムを実行中のある時点において、評価されていない残りのプログラムを表現するものであり、手続きあるいは関数として表現されるものである。",
                expected_uncommon_only: "[計]{けい}[算]{さん}[機]{き}[科]{か}[学]{がく}における継続とは、プログラムを[実]{じっ}[行]{こう}[中]{ちゅう}のある時点において、評価されていない残りのプログラムを表現するものであり、手続きあるいは[関]{かん}[数]{すう}として表現されるものである。",
                expected_all: "[計]{けい}[算]{さん}[機]{き}[科]{か}[学]{がく}における[継]{けい}[続]{ぞく}とは、プログラムを[実]{じっ}[行]{こう}[中]{ちゅう}のある[時]{じ}[点]{てん}において、[評]{ひょう}[価]{か}されていない[残]{のこ}りのプログラムを[表]{ひょう}[現]{げん}するものであり、[手]{て}[続]{つづ}きあるいは[関]{かん}[数]{すう}として[表]{ひょう}[現]{げん}されるものである。",
            },
        ];

        let annotator = annotate::Annotator::new_with_integrated_dictionary();

        for test in tests {
            let annotated = annotator.annotate(test.input);
            let result = annotated.render(&select::heuristic::uncommon_only, &format::markdown);
            assert_eq!(result, test.expected_uncommon_only);

            let result = annotated.render(&select::heuristic::all, &format::markdown);
            assert_eq!(result, test.expected_all);
        }
    }

    #[test]
    #[ignore = "lack of dictionary support"]
    fn place_names() {
        let tests = vec![
            Test {
                input: "東京都",
                expected_uncommon_only: "[東]{とう}[京]{きょう}[都]{と}",
                expected_all: "[東]{とう}[京]{きょう}[都]{と}",
            },
            Test {
                input: "新宿",
                expected_uncommon_only: "[新]{しん}[宿]{じゅく}",
                expected_all: "[新]{しん}[宿]{じゅく}",
            },
            Test {
                input: "渋谷",
                expected_uncommon_only: "[渋]{しぶ}[谷]{や}",
                expected_all: "[渋]{しぶ}[谷]{や}",
            },
            Test {
                input: "福岡市",
                expected_uncommon_only: "[福]{ふく}[岡]{おか}[市]{し}",
                expected_all: "[福]{ふく}[岡]{おか}[市]{し}",
            },
        ];

        let annotator = annotate::Annotator::new_with_integrated_dictionary();

        for test in tests {
            let annotated = annotator.annotate(test.input);
            let result = annotated.render(&select::heuristic::uncommon_only, &format::markdown);
            assert_eq!(result, test.expected_uncommon_only);

            let result = annotated.render(&select::heuristic::all, &format::markdown);
            assert_eq!(result, test.expected_all);
        }
    }
}
