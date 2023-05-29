use std::{collections::BTreeMap, io::BufRead};

use crate::parse::{self, dictionary_line};

pub const DOWNLOAD_URL: &str =
    "https://github.com/Doublevil/JmdictFurigana/releases/latest/download/JmdictFurigana.txt";

pub struct FrequencyEntry<'a> {
    kanji_element: &'a str,
    kanji_common: bool,
    reading_element: &'a str,
    reading_common: bool,
}

pub fn frequency_entries() -> impl Iterator<Item = FrequencyEntry<'static>> {
    #[cfg(feature = "dummy")]
    {
        [].into_iter()
    }
    #[cfg(not(feature = "dummy"))]
    jmdict::entries().flat_map(|e| {
        e.kanji_elements().flat_map(move |k| {
            e.reading_elements().map(move |r| FrequencyEntry {
                kanji_element: k.text,
                kanji_common: k.priority.is_common(),
                reading_element: r.text,
                reading_common: r.priority.is_common(),
            })
        })
    })
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct ReadingSpan {
    pub start_index: u8,
    pub end_index: u8,
    pub text: String,
}

impl From<parse::ReadingSpan<'_>> for ReadingSpan {
    fn from(value: parse::ReadingSpan<'_>) -> Self {
        Self {
            start_index: value.start_index,
            end_index: value.end_index,
            text: value.text.to_string(),
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct TextEntry {
    pub text: String,
    pub text_is_common: bool,
    pub reading: String,
    pub reading_is_common: bool,
    pub reading_spans: Vec<ReadingSpan>,
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone)]
pub struct DictionaryIndex {
    text: String,
    reading: String,
}

impl<T: AsRef<str>> From<T> for DictionaryIndex {
    fn from(s: T) -> Self {
        Self {
            text: s.as_ref().to_string(),
            reading: Default::default(),
        }
    }
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct Dictionary(BTreeMap<DictionaryIndex, TextEntry>);

impl Dictionary {
    pub fn lookup_word<'s>(&'s self, word: &str) -> Vec<&'s TextEntry> {
        self.0
            .range(DictionaryIndex::from(word)..)
            .map_while(|(DictionaryIndex { text, .. }, entry)| (text == word).then_some(entry))
            .collect()
    }

    pub fn lookup_prefixed<'s>(&'s self, prefix: &'s str) -> Vec<&'s TextEntry> {
        self.0
            .range(DictionaryIndex::from(prefix)..)
            .map_while(|(DictionaryIndex { text, .. }, entry)| {
                text.starts_with(prefix).then_some(entry)
            })
            .collect()
    }
}

pub fn build(input_reader: impl BufRead) -> Dictionary {
    let mut tree = input_reader
        .lines()
        .map(|line| {
            let line = line.unwrap();
            let (_, entry) = dictionary_line(&line).unwrap();
            (
                DictionaryIndex {
                    text: entry.text.to_string(),
                    reading: entry.reading.to_string(),
                },
                TextEntry {
                    text: entry.text.to_string(),
                    text_is_common: false,
                    reading: entry.reading.to_string(),
                    reading_is_common: false,
                    reading_spans: entry.reading_spans.into_iter().map(Into::into).collect(),
                },
            )
        })
        .collect::<BTreeMap<_, _>>();

    frequency_entries().for_each(|freq| {
        if let Some(e) = tree.get_mut(&DictionaryIndex {
            text: freq.kanji_element.into(),
            reading: freq.reading_element.into(),
        }) {
            e.reading_is_common = freq.reading_common;
            e.text_is_common = freq.kanji_common;
        }
    });

    Dictionary(tree)
}
