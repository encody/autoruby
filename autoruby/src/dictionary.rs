//! Dictionary data structures and parsing.

use std::{collections::BTreeMap, io::BufRead};

use crate::parse::{self, dictionary_line};

/// The URL to download the dictionary from.
pub const DOWNLOAD_URL: &str =
    "https://github.com/Doublevil/JmdictFurigana/releases/latest/download/JmdictFurigana.txt";

/// Frequency metadata for a dictionary entry.
pub struct FrequencyEntry<'a> {
    kanji_element: &'a str,
    kanji_common: bool,
    reading_element: &'a str,
    reading_common: bool,
}

/// Returns an iterator over all entries in the dictionary, including frequency metadata.
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

/// Represents the reading text associated with a substring of a word.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct ReadingSpan {
    /// The index of the first character of the substring.
    pub start_index: u8,
    /// The end index (exclusive) of the substring.
    pub end_index: u8,
    /// The reading text.
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

/// A dictionary entry, including reading and frequency data.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct TextEntry {
    /// The actual word the entry represents.
    pub text: String,
    /// Whether the text is common.
    pub text_is_common: bool,
    /// The reading of the word.
    pub reading: String,
    /// Whether the reading is common.
    pub reading_is_common: bool,
    /// The readings associated with each substring of the word.
    pub reading_spans: Vec<ReadingSpan>,
}

/// Dictionary index.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Clone)]
pub struct Index {
    text: String,
    reading: String,
}

impl<T: AsRef<str>> From<T> for Index {
    fn from(s: T) -> Self {
        Self {
            text: s.as_ref().to_string(),
            reading: String::new(),
        }
    }
}

/// A dictionary of words and their readings.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub struct Dictionary(BTreeMap<Index, TextEntry>);

impl Dictionary {
    /// Returns an iterator over all entries exactly matching a given word in the dictionary.
    pub fn lookup_word<'s: 'w, 'w>(
        &'s self,
        word: &'w str,
    ) -> impl 'w + Iterator<Item = &'s TextEntry> {
        self.0
            .range(Index::from(word)..)
            .map_while(move |(Index { text, .. }, entry)| (text == word).then_some(entry))
    }

    /// Returns an iterator over all dictionary entries matching a given prefix.
    pub fn lookup_prefixed<'s>(&'s self, prefix: &'s str) -> impl Iterator<Item = &'s TextEntry> {
        self.0
            .range(Index::from(prefix)..)
            .map_while(move |(Index { text, .. }, entry)| text.starts_with(prefix).then_some(entry))
    }
}

/// Error type for dictionary building.
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    /// Error reading a line.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    /// Error parsing a line.
    #[error("Failed to parse line: {0}")]
    Parse(String),
}

/// Builds a dictionary from a reader.
///
/// # Errors
///
/// Returns an error if the input reader fails to read or parse.
pub fn build(input_reader: impl BufRead) -> Result<Dictionary, BuildError> {
    let mut tree = input_reader
        .lines()
        .try_fold(BTreeMap::default(), |mut map, line| {
            let line = line?;
            let (_, entry) =
                dictionary_line(&line).map_err(|_| BuildError::Parse(line.to_string()))?;

            let index = Index {
                text: entry.text.to_string(),
                reading: entry.reading.to_string(),
            };
            map.insert(
                index,
                TextEntry {
                    text: entry.text.to_string(),
                    text_is_common: false,
                    reading: entry.reading.to_string(),
                    reading_is_common: false,
                    reading_spans: entry.reading_spans.into_iter().map(Into::into).collect(),
                },
            );
            Ok::<_, BuildError>(map)
        })?;

    frequency_entries().for_each(|freq| {
        if let Some(e) = tree.get_mut(&Index {
            text: freq.kanji_element.into(),
            reading: freq.reading_element.into(),
        }) {
            e.reading_is_common = freq.reading_common;
            e.text_is_common = freq.kanji_common;
        }
    });

    Ok(Dictionary(tree))
}
