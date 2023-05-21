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

#[cfg(all(test, feature = "integrated"))]
mod tests {
    use pretty_assertions::assert_eq;

    use crate::{annotate, format};

    #[test]
    fn test_complex_short() {
        let processor = annotate::Annotator::new_with_integrated_dictionary(true);
        let result = processor.annotate_with_first(format::markdown, "全単射。");
        assert_eq!(result, "[全]{ぜん}[単]{たん}[射]{しゃ}。",);
    }

    #[test]
    fn test_simple() {
        let processor = annotate::Annotator::new_with_integrated_dictionary(false);
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
        let processor = annotate::Annotator::new_with_integrated_dictionary(true);
        let result = processor.annotate_with_first(
            format::markdown,
            "計算機科学における継続とは、プログラムを実行中のある時点において、評価されていない残りのプログラムを表現するものであり、手続きあるいは関数として表現されるものである。",
        );
        assert_eq!(
            result,
            "[計]{けい}[算]{さん}[機]{き}[科]{か}[学]{がく}における継続とは、プログラムを[実]{じっ}[行]{こう}[中]{ちゅう}のある時点において、評価されていない残りのプログラムを表現するものであり、手続きあるいは[関]{かん}[数]{すう}として表現されるものである。"
        );
    }
}
