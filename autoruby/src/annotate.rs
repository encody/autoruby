use std::{borrow::Cow, path::Path, vec};

use lindera::tokenizer::{Tokenizer, TokenizerConfig};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use wana_kana::IsJapaneseChar;

use crate::format::Format;

pub struct Annotator {
    db: Connection,
    tokenizer: Tokenizer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnnotationSpan {
    pub start_index: usize,
    pub end_index: usize,
    pub text: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Annotation {
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
    // TODO: Technically, it is possible for annotations to be empty. Is it better devex to do away with the enum and just use an empty annotations vector for text that we didn't even try to annotate?
    pub annotations: Vec<Annotation>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TextFragment<'a> {
    Plain(Cow<'a, str>),
    Annotated(AnnotatedTextFragment<'a>),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AnnotatedText<'a> {
    pub fragments: Vec<TextFragment<'a>>,
}

impl Annotator {
    pub fn new(db_path: impl AsRef<Path>) -> Self {
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

        Self { db, tokenizer }
    }

    pub fn find_annotations_for_text(&self, text: &str) -> Vec<Annotation> {
        // This function is a little janky. Although calculating relations like
        // this would be unnecessary if we used an ORM, I don't think this
        // project justifies the additional weight/complexity just for this
        // single function. ORMs are probably slower, too.
        let mut query = self
            .db
            .prepare(
                r#"--sql
                    select text_entry_id, start_index, end_index, rt
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
                Ok((
                    text_entry_id,
                    AnnotationSpan {
                        start_index: r.get(1)?,
                        end_index: r.get(2)?,
                        text: r.get(3)?,
                    },
                ))
            })
            .unwrap()
            .filter_map(|r| r.ok())
            .collect::<Vec<_>>();

        let mut annotations = vec![];
        let mut last_group_id = 0; // sqlite indices start at 1
        for (group_id, span) in spans {
            if group_id != last_group_id {
                annotations.push(Annotation { spans: vec![span] });
                last_group_id = group_id;
            } else {
                annotations.last_mut().unwrap().spans.push(span);
            }
        }

        annotations
    }

    fn annotate_token<'a>(&self, token: lindera::Token<'a>) -> AnnotatedTextFragment<'a> {
        let dictionary_form = token.details.as_ref().and_then(|d| {
            if let [_, _, _, _, _, _, dictionary_form, _reading, _pronunciation] = &d[..] {
                Some(dictionary_form)
            } else {
                None
            }
        });

        let annotations = dictionary_form
            .map(|dictionary_form| self.find_annotations_for_text(dictionary_form))
            .unwrap_or_default();

        AnnotatedTextFragment {
            text: token.text,
            annotations,
        }
    }

    pub fn generate_annotations<'a>(&self, input: &'a str) -> AnnotatedText<'a> {
        let fragments = self
            .tokenizer
            .tokenize_with_details(input)
            .unwrap()
            .into_iter()
            .map(|t| {
                let should_annotate = t.text.as_ref().chars().any(IsJapaneseChar::is_kanji);

                if should_annotate {
                    TextFragment::Annotated(self.annotate_token(t))
                } else {
                    TextFragment::Plain(t.text)
                }
            })
            .collect();

        AnnotatedText { fragments }
    }

    pub fn annotate(&self, f: Format, input: &str) -> String {
        let t = self.generate_annotations(input);
        t.fragments
            .into_iter()
            .map(|frag| match frag {
                TextFragment::Plain(s) => s,
                TextFragment::Annotated(AnnotatedTextFragment { text, annotations }) => {
                    annotations[0].apply(&text, f).into()
                }
            })
            .collect()
    }
}
