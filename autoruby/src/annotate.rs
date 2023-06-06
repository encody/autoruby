//! Work with and generate annotations.

use std::{borrow::Cow, cmp::Ordering, vec};

use lindera_tokenizer::tokenizer::{Tokenizer, TokenizerConfig};
use wana_kana::ConvertJapanese;

use crate::{
    dictionary::{Dictionary, TextEntry},
    format::Format,
    select::Select,
};

fn apply(text_entry: &TextEntry, text: &str, format: &(impl Format + ?Sized)) -> String {
    // assuming the rubies are already sorted
    let text = text.chars().collect::<Vec<_>>();
    let (last_index, mut s) = text_entry.reading_spans.iter().fold(
        (0, String::new()),
        |(valid_next_index, mut s), span| {
            let start_index = span.start_index as usize;
            let end_index = span.end_index as usize;
            if start_index >= valid_next_index {
                s.push_str(
                    &text[valid_next_index..start_index]
                        .iter()
                        .collect::<String>(),
                );
                let base = &text[start_index..=end_index].iter().collect::<String>();
                let text = &span.text;

                s.push_str(&format.format(base, text));
                (end_index + 1, s)
            } else {
                (valid_next_index, s)
            }
        },
    );

    s.push_str(&text[last_index..].iter().collect::<String>());
    s
}

/// A text fragment with annotations. Usually a word or well-known phrase.
#[derive(Clone, Debug)]
pub struct AnnotatedTextFragment<'a> {
    /// The original text of the fragment.
    pub text: Cow<'a, str>,
    /// The annotations associated with the fragment.
    pub annotations: Vec<&'a TextEntry>,
}

impl<'a> AnnotatedTextFragment<'a> {
    /// Create a new fragment of a word with no annotations.
    #[must_use]
    pub fn plain(text: Cow<'a, str>) -> Self {
        Self {
            text,
            annotations: vec![],
        }
    }
}

/// A complete text with annotations.
#[derive(Clone, Debug, Default)]
pub struct AnnotatedText<'a> {
    /// The fragments of the text with annotations.
    pub fragments: Vec<AnnotatedTextFragment<'a>>,
}

impl<'a> AnnotatedText<'a> {
    /// Render the annotated text into a string.
    pub fn render(
        &self,
        selector: &(impl Select + ?Sized),
        format: &(impl Format + ?Sized),
    ) -> String {
        self.fragments
            .iter()
            .map(|frag| {
                let annotation = selector.select(&frag.annotations);
                match annotation {
                    Some(annotation) => apply(annotation, &frag.text, format).into(),
                    None => frag.text.clone(),
                }
            })
            .collect()
    }
}

#[derive(Clone, Debug)]
struct InternalToken<'a> {
    pub original_text: Cow<'a, str>,
    pub lookup_text: String,
    pub reading_hint: Option<String>,
}

impl<'a> From<&'a str> for InternalToken<'a> {
    fn from(value: &'a str) -> Self {
        let lookup_text = value.to_string();
        Self {
            original_text: value.into(),
            lookup_text,
            reading_hint: None,
        }
    }
}

impl<'a> From<Cow<'a, str>> for InternalToken<'a> {
    fn from(text: Cow<'a, str>) -> Self {
        let lookup_text = text.to_string();
        Self {
            original_text: text,
            lookup_text,
            reading_hint: None,
        }
    }
}

impl<'a> From<String> for InternalToken<'a> {
    fn from(text: String) -> Self {
        let lookup_text = text.clone();
        Self {
            original_text: text.into(),
            lookup_text,
            reading_hint: None,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Details<'a>(&'a [String]);

impl<'a> TryFrom<&'a [String]> for Details<'a> {
    type Error = ();

    fn try_from(value: &'a [String]) -> Result<Self, Self::Error> {
        if value.len() < 11 {
            return Err(());
        }
        Ok(Self(value))
    }
}

impl<'a> Details<'a> {
    fn reading_katakana(&'a self) -> &'a str {
        &self.0[9]
    }

    fn dictionary_form(&'a self) -> &'a str {
        &self.0[10]
    }
}

impl<'a> InternalToken<'a> {
    fn from_token(token_text: &'a str, details: Option<Details<'_>>) -> Self {
        Self {
            original_text: token_text.into(),
            lookup_text: details
                .map(|d| d.dictionary_form().to_string())
                .unwrap_or_default(),
            reading_hint: details.map(|d| d.reading_katakana().to_hiragana()),
        }
    }
}

/// Annotates text with readings, given a dictionary.
pub struct Annotator<'a> {
    dictionary: &'a Dictionary,
    tokenizer: Tokenizer,
}

impl<'a> Annotator<'a> {
    /// Create a new annotator with the integrated dictionary.
    #[cfg(feature = "integrated")]
    #[must_use]
    pub fn new_with_integrated_dictionary() -> Self {
        Annotator::new(&crate::DICTIONARY)
    }

    /// Create a new annotator with a dictionary.
    #[must_use]
    pub fn new(dictionary: &'a Dictionary) -> Self {
        let dictionary_kind = lindera_dictionary::DictionaryKind::UniDic;

        let tokenizer = Tokenizer::from_config(TokenizerConfig {
            dictionary: lindera_dictionary::DictionaryConfig {
                kind: Some(dictionary_kind),
                path: None,
            },
            user_dictionary: None,
            mode: lindera_core::mode::Mode::Normal,
        })
        .expect("Failed to initialize tokenizer");

        Self {
            dictionary,
            tokenizer,
        }
    }

    fn annotate_internal_token<'b>(
        &'b self,
        token: InternalToken<'b>,
    ) -> AnnotatedTextFragment<'b> {
        let reading_hint = token.reading_hint.as_ref();

        let mut entries = self
            .dictionary
            .lookup_word(&token.lookup_text)
            .collect::<Vec<_>>();

        entries.sort_by(|a, b| {
            #[allow(clippy::match_same_arms)] // order-dependent
            match (
                Some(&a.reading) == reading_hint,
                Some(&b.reading) == reading_hint,
                a.reading_is_common,
                b.reading_is_common,
            ) {
                (true, false, ..) => Ordering::Less,
                (false, true, ..) => Ordering::Greater,
                (_, _, true, false) => Ordering::Less,
                (_, _, false, true) => Ordering::Greater,
                _ => Ordering::Equal,
            }
        });

        AnnotatedTextFragment {
            text: token.original_text,
            annotations: entries,
        }
    }

    /// Annotate a text with readings.
    #[must_use]
    #[allow(clippy::missing_panics_doc)]
    pub fn annotate<'b>(&'b self, text: &'b str) -> AnnotatedText<'b> {
        if text.trim().is_empty() {
            return AnnotatedText::default();
        }

        let mut tokens = self.tokenizer.tokenize(text).unwrap(); // this function is actually infallible. Not sure why it returns a Result

        // tokens must have at least one element

        let mut internal_tokens: Vec<InternalToken<'b>> = vec![];
        let mut token_buffer_start: usize = 0;
        // Exclusive upper bound
        let mut token_buffer_end: usize = 1;
        let mut buffer_possibilities = self
            .dictionary
            .lookup_prefixed(tokens[0].text)
            .collect::<Vec<_>>();

        while token_buffer_start < tokens.len() {
            // remember: exclusive upper bound
            let next_token_exists = token_buffer_end < tokens.len();

            // closure for lazy eval
            let possibilities_remain = || {
                let current_substring = tokens[token_buffer_start..token_buffer_end]
                    .iter()
                    .map(|t| t.text)
                    .collect::<String>();

                buffer_possibilities
                    .iter()
                    .any(|p| p.text.starts_with(&current_substring))
            };

            if next_token_exists && possibilities_remain() {
                // good, continue
                token_buffer_end += 1;
            } else {
                // if not, find a possibility that does work with shorter substring
                let mut longest_possibility_end = token_buffer_end;
                while longest_possibility_end > token_buffer_start {
                    let substring = tokens[token_buffer_start..longest_possibility_end]
                        .iter()
                        .map(|t| t.text)
                        .collect::<String>();

                    if buffer_possibilities.iter().any(|p| p.text == substring) {
                        break;
                    }
                    longest_possibility_end -= 1;
                }

                // # of tokens that match possibilities is 0 or 1.
                // Obviously if no possibilities exist, there will be 0, but
                // we still have to advance, so we'll just advance by a single
                // token.
                let longest_is_single_token = longest_possibility_end <= token_buffer_start + 1;

                if longest_is_single_token {
                    // The number of tokens that match a suggestion is 0 or 1.
                    // That is, we cannot generate readings for a longer text fragment.
                    let t = &mut tokens[token_buffer_start];
                    let details = t
                        .get_details()
                        .map(|v| v.into_iter().map(ToString::to_string).collect::<Vec<_>>());
                    let internal_token = details.map_or_else(
                        || t.text.into(),
                        |details| {
                            InternalToken::from_token(
                                t.text,
                                Details::try_from(details.as_ref()).ok(),
                            )
                        },
                    );
                    internal_tokens.push(internal_token);
                    token_buffer_start += 1;
                } else {
                    // We can concatenate two or more tokens together to create
                    // a longer text fragment, for which we know readings exist.
                    let substring = tokens[token_buffer_start..longest_possibility_end]
                        .iter()
                        .map(|t| t.text)
                        .collect::<String>();
                    internal_tokens.push(substring.into());
                    token_buffer_start = longest_possibility_end;
                }

                // token_buffer_end is an exclusive bound
                token_buffer_end = token_buffer_start + 1;

                if let Some(t) = tokens.get(token_buffer_start) {
                    buffer_possibilities = self.dictionary.lookup_prefixed(t.text).collect();
                }
            }
        }

        AnnotatedText {
            fragments: internal_tokens
                .into_iter()
                .map(|internal_token| self.annotate_internal_token(internal_token))
                .collect(),
        }
    }
}
