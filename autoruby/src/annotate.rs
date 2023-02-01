use std::{borrow::Cow, path::Path, vec};

use lindera::tokenizer::{Tokenizer, TokenizerConfig};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use wana_kana::ConvertJapanese;

use crate::format::Format;

fn escape_sql_like(input: &str) -> String {
    input
        .replace('\\', "\\\\")
        .replace('%', "\\%")
        .replace('_', "\\_")
}

pub struct Annotator {
    db: Connection,
    tokenizer: Tokenizer,
    avoid_common: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotationSpan {
    pub start_index: usize,
    pub end_index: usize,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotation {
    pub reading: String,
    pub spans: Vec<AnnotationSpan>,
}

impl Annotation {
    fn apply(&self, text: &str, format: Format) -> String {
        // assuming the rubies are already sorted
        let text = text.chars().collect::<Vec<_>>();
        let (last_index, mut s) =
            self.spans
                .iter()
                .fold((0, String::new()), |(valid_next_index, mut s), span| {
                    if span.start_index >= valid_next_index {
                        s.push_str(
                            &text[valid_next_index..span.start_index]
                                .iter()
                                .collect::<String>(),
                        );
                        let base = &text[span.start_index..=span.end_index]
                            .iter()
                            .collect::<String>();
                        let text = &span.text;

                        s.push_str(&format(base, text));
                        (span.end_index + 1, s)
                    } else {
                        (valid_next_index, s)
                    }
                });

        s.push_str(&text[last_index..].iter().collect::<String>());
        s
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnnotatedTextFragment<'a> {
    pub text: Cow<'a, str>,
    pub annotations: Vec<Annotation>,
}

impl<'a> AnnotatedTextFragment<'a> {
    pub fn plain(text: Cow<'a, str>) -> Self {
        Self {
            text,
            annotations: vec![],
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, Default)]
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

impl Annotator {
    pub fn new(db_path: impl AsRef<Path>, avoid_common: bool) -> Self {
        let tokenizer = Tokenizer::with_config(TokenizerConfig {
            dictionary: lindera::tokenizer::DictionaryConfig {
                kind: Some(lindera::DictionaryKind::IPADIC),
                path: None,
            },
            user_dictionary: None,
            mode: lindera::mode::Mode::Normal,
        })
        .expect("Failed to initialize tokenizer");

        let db = Connection::open(&db_path).unwrap_or_else(|e| {
            panic!(
                "Failed to connect to database at {}: {e}",
                &db_path.as_ref().display(),
            )
        });

        Self {
            db,
            tokenizer,
            avoid_common,
        }
    }

    pub fn find_annotations_for_word(&self, word: &str) -> Vec<Annotation> {
        // This function is a little janky. Although calculating relations like
        // this would be unnecessary if we used an ORM, I don't think this
        // project justifies the additional weight/complexity just for this
        // single function. ORMs are probably slower, too.
        let mut query = self
            .db
            .prepare(
                r#"--sql
                    select text_entry_id, reading, start_index, end_index, rt
                    from text_entry
                    join ruby_entry on text_entry.id = ruby_entry.text_entry_id
                    where text_entry.text = ?1
                    order by text_entry_id asc, end_index asc
                "#,
            )
            .unwrap();

        let spans = query
            .query_map([word], |r| {
                let text_entry_id: usize = r.get(0)?;
                let reading: String = r.get(1)?;
                Ok((
                    text_entry_id,
                    reading,
                    AnnotationSpan {
                        start_index: r.get(2)?,
                        end_index: r.get(3)?,
                        text: r.get(4)?,
                    },
                ))
            })
            .unwrap()
            .filter_map(|r| r.ok())
            .collect::<Vec<_>>();

        let mut annotations = vec![];
        let mut last_group_id = 0; // sqlite indices start at 1
        for (group_id, reading, span) in spans {
            if group_id != last_group_id {
                annotations.push(Annotation {
                    reading,
                    spans: vec![span],
                });
                last_group_id = group_id;
            } else {
                annotations.last_mut().unwrap().spans.push(span);
            }
        }

        annotations
    }

    fn annotate_internal_token<'a>(&self, token: InternalToken<'a>) -> AnnotatedTextFragment<'a> {
        if self.avoid_common && self.is_kanji_common(&token.lookup_text) {
            return AnnotatedTextFragment::plain(token.lookup_text.into());
        }

        let reading_hint = token.reading_hint.as_ref();

        let annotations = reading_hint.map_or_else(
            || self.find_annotations_for_word(&token.lookup_text),
            |reading_hint| {
                let (mut reading_matches, others) = self
                    .find_annotations_for_word(&token.lookup_text)
                    .into_iter()
                    .partition::<Vec<_>, _>(|v| &v.reading == reading_hint);

                reading_matches.extend(others);
                reading_matches
            },
        );

        AnnotatedTextFragment {
            text: token.original_text,
            annotations,
        }
    }

    fn is_kanji_common(&self, input: &str) -> bool {
        let mut query = self
            .db
            .prepare(
                r#"--sql
                    select text_common from text_entry where text = ?1
                "#,
            )
            .unwrap();

        query
            .query_row([input], |r| r.get::<_, bool>(0))
            .unwrap_or_default()
    }

    fn query_text_entries_starting_with(&self, prefix: &str) -> Vec<String> {
        let prefix_escaped = escape_sql_like(prefix);
        let mut query = self
            .db
            .prepare(
                r#"--sql
                    select text from text_entry where text like ?1 || '%' escape '\'
                "#,
            )
            .unwrap();

        query
            .query_map([prefix_escaped], |r| r.get::<_, String>(0))
            .unwrap()
            .filter_map(|a| a.ok())
            .collect()
    }

    pub fn annotate<'a>(&self, text: &'a str) -> AnnotatedText<'a> {
        if text.trim().is_empty() {
            return Default::default();
        }

        let tokens = self.tokenizer.tokenize_with_details(text).unwrap();

        // tokens must have at least one element

        let mut internal_tokens: Vec<InternalToken<'a>> = vec![];
        let mut token_buffer_start: usize = 0;
        // Exclusive upper bound
        let mut token_buffer_end: usize = 1;
        let mut buffer_possibilities: Vec<String> =
            self.query_text_entries_starting_with(&tokens[0].text);

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
                    .any(|p| p.starts_with(&current_substring))
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

                    if buffer_possibilities.contains(&substring) {
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
                    buffer_possibilities = self.query_text_entries_starting_with(&t.text);
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
                Some(a) => a.apply(&frag.text, f).into(),
                None => frag.text,
            })
            .collect()
    }
}
