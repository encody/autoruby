use dictionary::Dictionary;
use once_cell::sync::Lazy;

#[cfg(feature = "dict-autodownload")]
static DICTIONARY: Lazy<Dictionary> = Lazy::new(|| {
    let dict_bytes = include_bytes!(concat!(env!("OUT_DIR"), "/dict.bin"));
    let dictionary: Dictionary = bincode::deserialize(dict_bytes).unwrap();

    dictionary
});

pub mod annotate;
pub mod dictionary;
pub mod format;
mod parse;

#[cfg(test)]
mod tests {
    use crate::{annotate, format};

    #[test]
    fn test_complex_short() {
        let processor = annotate::Annotator::new_with_default_dictionary(true);
        let result = processor.annotate_with_first(format::markdown, "全単射。");
        assert_eq!(result, "[全]{ぜん}[単]{たん}[射]{しゃ}。",);
    }

    #[test]
    fn test_simple() {
        let processor = annotate::Annotator::new_with_default_dictionary(false);
        let result = processor.annotate_with_first(
            format::markdown,
            "神は「光あれ」と言われた。すると光があった。",
        );
        assert_eq!(
            result,
            "[神]{かみ}は「[光]{ひかり}あれ」と[言]{い}われた。すると[光]{ひかり}があった。",
        );
    }

    #[test]
    fn test_complex_long() {
        let processor = annotate::Annotator::new_with_default_dictionary(true);
        let result = processor.annotate_with_first(
            format::markdown,
            "数学において、全単射あるいは双射とは、写像であって、その写像の終域となる集合の任意の元に対し、その元を写像の像とする元が、写像の定義域となる集合に常にただ一つだけ存在するようなもの、すなわち単射かつ全射であるような写像のことを言う。",
        );
        assert_eq!(
            result,
            "数学において、[全]{ぜん}[単]{たん}[射]{しゃ}あるいは[双]{そう}[射]{しゃ}とは、[写]{しゃ}[像]{ぞう}であって、その[写]{しゃ}[像]{ぞう}の[終]{おわり}[域]{いき}となる集合の任意の[元]{もと}に[対]{たい}し、その[元]{もと}を[写]{しゃ}[像]{ぞう}の像とする[元]{もと}が、[写]{しゃ}[像]{ぞう}の[定]{てい}[義]{ぎ}[域]{いき}となる集合に常にただ一つだけ存在するようなもの、すなわち[単]{たん}[射]{しゃ}かつ[全]{ぜん}[射]{しゃ}であるような[写]{しゃ}[像]{ぞう}のことを言う。"
        );
    }
}
