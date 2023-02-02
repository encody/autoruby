use std::{borrow::Cow, cmp::Ordering, rc::Rc, vec};

use lindera::tokenizer::{Tokenizer, TokenizerConfig};
use thiserror::Error;
use wana_kana::ConvertJapanese;

use crate::{
    dictionary::{Dictionary, TextEntry},
    format::Format,
};

pub struct Annotator {
    dictionary: Rc<Dictionary>,
    tokenizer: Tokenizer,
    avoid_common: bool,
}

fn apply(text_entry: &TextEntry, text: &str, format: Format) -> String {
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

                s.push_str(&format(base, text));
                (end_index + 1, s)
            } else {
                (valid_next_index, s)
            }
        },
    );

    s.push_str(&text[last_index..].iter().collect::<String>());
    s
}

#[derive(Clone, Debug)]
pub struct AnnotatedTextFragment<'a> {
    pub text: Cow<'a, str>,
    pub annotations: Vec<&'a TextEntry>,
}

impl<'a> AnnotatedTextFragment<'a> {
    pub fn plain(text: Cow<'a, str>) -> Self {
        Self {
            text,
            annotations: vec![],
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct AnnotatedText<'a> {
    pub fragments: Vec<AnnotatedTextFragment<'a>>,
}

#[derive(Clone, Debug)]
struct InternalToken<'a> {
    pub original_text: Cow<'a, str>,
    pub lookup_text: String,
    pub reading_hint: Option<String>,
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

#[derive(Debug, Clone, Error)]
enum TokenError {
    #[error("Missing details on token \"{token_text}\"")]
    MissingDetails { token_text: String },
    #[error("Unknown details format on token \"{token_text}\": {details:?}")]
    UnknownDetails {
        token_text: String,
        details: Vec<String>,
    },
}

impl<'a> TryFrom<&'_ lindera::Token<'a>> for InternalToken<'a> {
    type Error = TokenError;

    fn try_from(token: &'_ lindera::Token<'a>) -> Result<Self, Self::Error> {
        let details = &token
            .details
            .as_ref()
            .ok_or_else(|| TokenError::MissingDetails {
                token_text: token.text.to_string(),
            })?[..];

        if let [_, _, _, _, _, _, dictionary_form, reading_katakana, _pronunciation] = details {
            Ok(Self {
                original_text: token.text.clone(),
                lookup_text: dictionary_form.clone(),
                reading_hint: Some(reading_katakana.to_hiragana()),
            })
        } else {
            Err(TokenError::UnknownDetails {
                token_text: token.text.to_string(),
                details: details.to_vec(),
            })
        }
    }
}

impl Default for Annotator {
    fn default() -> Self {
        Annotator::new(Rc::clone(&*crate::DICTIONARY), true)
    }
}

impl Annotator {
    pub fn new_with_default_dictionary(avoid_common: bool) -> Self {
        Annotator::new(Rc::clone(&*crate::DICTIONARY), avoid_common)
    }

    pub fn new(dictionary: Rc<Dictionary>, avoid_common: bool) -> Self {
        let tokenizer = Tokenizer::from_config(TokenizerConfig {
            with_details: true,
            dictionary: lindera::tokenizer::DictionaryConfig {
                kind: Some(lindera::DictionaryKind::IPADIC),
                path: None,
            },
            user_dictionary: None,
            mode: lindera::mode::Mode::Normal,
        })
        .expect("Failed to initialize tokenizer");

        Self {
            dictionary,
            tokenizer,
            avoid_common,
        }
    }

    fn annotate_internal_token<'a>(
        &'a self,
        token: InternalToken<'a>,
    ) -> AnnotatedTextFragment<'a> {
        let reading_hint = token.reading_hint.as_ref();

        let mut entries = self.dictionary.lookup_word(&token.lookup_text);

        if self.avoid_common {
            entries.retain(|e| !e.text_is_common);
        }

        entries.sort_by(|a, b| {
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

    pub fn annotate<'a>(&'a self, text: &'a str) -> AnnotatedText<'a> {
        if text.trim().is_empty() {
            return Default::default();
        }

        let tokens = self.tokenizer.tokenize(text).unwrap();

        // tokens must have at least one element

        let mut internal_tokens: Vec<InternalToken<'a>> = vec![];
        let mut token_buffer_start: usize = 0;
        // Exclusive upper bound
        let mut token_buffer_end: usize = 1;
        let mut buffer_possibilities = self.dictionary.lookup_prefixed(&tokens[0].text);

        while token_buffer_start < tokens.len() {
            // remember: exclusive upper bound
            let next_token_exists = token_buffer_end < tokens.len();

            // closure for lazy eval
            let possibilities_remain = || {
                let current_substring = tokens[token_buffer_start..token_buffer_end]
                    .iter()
                    .map(|t| t.text.as_ref())
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
                        .map(|t| t.text.as_ref())
                        .collect::<String>();

                    if buffer_possibilities.iter().any(|p| &p.text == &substring) {
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
                    let t = &tokens[token_buffer_start];
                    internal_tokens.push(t.try_into().unwrap_or_else(|_| t.text.clone().into()));
                    token_buffer_start += 1;
                } else {
                    // We can concatenate two or more tokens together to create a longer text fragment, for which we know readings exist.
                    let substring = tokens[token_buffer_start..longest_possibility_end]
                        .iter()
                        .map(|t| t.text.as_ref())
                        .collect::<String>();
                    internal_tokens.push(substring.into());
                    token_buffer_start = longest_possibility_end;
                }

                // token_buffer_end is an exclusive bound
                token_buffer_end = token_buffer_start + 1;

                if let Some(t) = tokens.get(token_buffer_start) {
                    buffer_possibilities = self.dictionary.lookup_prefixed(&t.text);
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

    pub fn annotate_with_first(&self, f: Format, input: &str) -> String {
        let t = self.annotate(input);
        t.fragments
            .into_iter()
            .map(|frag| match frag.annotations.first() {
                Some(a) => apply(a, &frag.text, f).into(),
                None => frag.text,
            })
            .collect()
    }
}
