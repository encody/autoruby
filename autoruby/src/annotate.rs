use std::{borrow::Cow, path::Path, vec};

use lindera::tokenizer::{Tokenizer, TokenizerConfig};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use wana_kana::{to_katakana::to_katakana, IsJapaneseChar};

use crate::format::Format;

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

    fn annotate_token<'a>(&self, token: lindera::Token<'a>) -> TextFragment<'a> {
        println!("{}", token.text);

        let mut details = token.details.as_ref().and_then(|d| {
            if let [_, _, _, _, _, _, dictionary_form, reading_katakana, _pronunciation] = &d[..] {
                Some((dictionary_form, reading_katakana))
            } else {
                None
            }
        });

        if self.avoid_common {
            details = details.filter(|(dictionary_form, _)| !self.is_kanji_common(dictionary_form));
        }

        let details = details;

        let annotations = details
            .map(|(dictionary_form, reading_katakana)| {
                let (mut reading_matches, others) = self
                    .find_annotations_for(dictionary_form)
                    .into_iter()
                    .partition::<Vec<_>, _>(|v| &to_katakana(&v.reading) == reading_katakana);

                // reorder so the ones where the reading matches the analyzer's suggestion wins
                reading_matches.extend(others);
                reading_matches
            })
            .unwrap_or_default();

        TextFragment {
            text: token.text,
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

        query.query_row([input], |r| r.get::<_, bool>(0)).unwrap()
    }

    pub fn generate_annotations<'a>(&self, input: &'a str) -> AnnotatedText<'a> {
        let fragments = self
            .tokenizer
            .tokenize_with_details(input)
            .unwrap()
            .into_iter()
            .map(|t| {
                let contains_kanji = t.text.as_ref().chars().any(IsJapaneseChar::is_kanji);

                if contains_kanji {
                    self.annotate_token(t)
                } else {
                    TextFragment::plain(t.text)
                }
            })
            .collect();

        AnnotatedText { fragments }
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
