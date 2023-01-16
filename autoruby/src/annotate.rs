use std::{borrow::Cow, path::Path, vec};

use lindera::tokenizer::{Tokenizer, TokenizerConfig};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use wana_kana::ConvertJapanese;

use crate::format::Format;

fn escape_like(input: &str) -> String {
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
pub struct TextFragment<'a> {
    pub text: Cow<'a, str>,
    pub annotations: Vec<Annotation>,
}

impl<'a> TextFragment<'a> {
    pub fn plain(text: Cow<'a, str>) -> Self {
        Self {
            text,
            annotations: vec![],
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnnotatedText<'a> {
    pub fragments: Vec<TextFragment<'a>>,
}

struct InternalToken<'a> {
    pub original_text: Cow<'a, str>,
    pub lookup_text: String,
    pub reading_hint: Option<String>,
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

impl<'a> TryFrom<&'_ lindera::Token<'a>> for InternalToken<'a> {
    type Error = ();

    fn try_from(token: &'_ lindera::Token<'a>) -> Result<Self, Self::Error> {
        let details = &token.details.as_ref().ok_or(())?[..];
        if let [_, _, _, _, _, _, dictionary_form, reading_katakana, _pronunciation] = details {
            Ok(Self {
                original_text: token.text.clone(),
                lookup_text: dictionary_form.clone(),
                reading_hint: Some(reading_katakana.to_hiragana()),
            })
        } else {
            Err(())
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
        .unwrap();

        let db = Connection::open(db_path).unwrap();

        Self {
            db,
            tokenizer,
            avoid_common,
        }
    }

    pub fn find_annotations_for(&self, text: &str) -> Vec<Annotation> {
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
            .query_map([text], |r| {
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

    fn annotate_token<'a>(&self, token: InternalToken<'a>) -> TextFragment<'a> {
        if self.avoid_common {
            if self.is_kanji_common(&token.lookup_text) {
                return TextFragment::plain(token.lookup_text.into());
            }
        }

        let reading_hint = token.reading_hint.as_ref();

        let annotations = reading_hint.map_or_else(
            || self.find_annotations_for(&token.lookup_text),
            |reading_hint| {
                let (mut reading_matches, others) = self
                    .find_annotations_for(&token.lookup_text)
                    .into_iter()
                    .partition::<Vec<_>, _>(|v| &v.reading == reading_hint);

                reading_matches.extend(others);
                reading_matches
            },
        );

        TextFragment {
            text: token.original_text.into(),
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

    fn db_prefixed(&self, input: &str) -> Vec<String> {
        let input_escaped = escape_like(input);
        let mut query = self
            .db
            .prepare(
                r#"--sql
                    select text from text_entry where text like ?1 || '%' escape '\'
                "#,
            )
            .unwrap();

        query
            .query_map([input_escaped], |r| r.get::<_, String>(0))
            .unwrap()
            .map(|a| a.unwrap())
            .collect()
    }

    pub fn generate_annotations<'a>(&self, input: &'a str) -> AnnotatedText<'a> {
        let tokens = self.tokenizer.tokenize_with_details(input).unwrap();

        // tokens must have at least one element

        let mut pieces: Vec<InternalToken<'a>> = vec![];
        let mut token_buffer_start: usize = 0;
        let mut token_buffer_end: usize = 1;
        let mut buffer_possibilities: Vec<String> = self.db_prefixed(&tokens[0].text);

        while token_buffer_end <= tokens.len() {
            let current_substring = tokens[token_buffer_start..token_buffer_end]
                .iter()
                .map(|t| t.text.as_ref())
                .collect::<String>();

            if buffer_possibilities
                .iter()
                .any(|p| p.starts_with(&current_substring)) {
                // good, continue
                token_buffer_end += 1;
            } else {
                // if not, find a possibility that does work with shorter substring
                let mut end = token_buffer_end - 1;
                while end > token_buffer_start {
                    let substring = tokens[token_buffer_start..end]
                        .iter()
                        .map(|t| t.text.as_ref())
                        .collect::<String>();

                    if buffer_possibilities.contains(&substring) {
                        break;
                    }
                    end -= 1;
                }

                if end <= token_buffer_start + 1 {
                    pieces.push((&tokens[token_buffer_start]).try_into().unwrap());
                    token_buffer_start += 1;
                } else {
                    let substring = tokens[token_buffer_start..end]
                        .iter()
                        .map(|t| t.text.as_ref())
                        .collect::<String>();
                    pieces.push(substring.into());
                    token_buffer_start = end;
                }

                token_buffer_end = token_buffer_start + 1;

                if let Some(t) = tokens.get(token_buffer_start) {
                    buffer_possibilities = self.db_prefixed(&t.text);
                }
            }
        }

        // probably need to do something with the leftovers

        AnnotatedText {
            fragments: pieces
                .into_iter()
                .map(|token| self.annotate_token(token))
                .collect(),
        }
    }

    pub fn annotate(&self, f: Format, input: &str) -> String {
        let t = self.generate_annotations(input);
        t.fragments
            .into_iter()
            .map(|frag| match frag.annotations.first() {
                Some(a) => a.apply(&frag.text, f).into(),
                None => frag.text,
            })
            .collect()
    }
}
