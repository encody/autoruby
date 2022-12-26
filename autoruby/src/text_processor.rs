use std::path::Path;

use lindera::tokenizer::{Tokenizer, TokenizerConfig};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use wana_kana::IsJapaneseChar;

use crate::format::FormatRuby;

pub struct TextProcessor {
    db: Connection,
    tokenizer: Tokenizer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RubySpan {
    pub start_index: usize,
    pub end_index: usize,
    pub ruby_text: String,
}

impl TextProcessor {
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

    pub fn suggest_rubies(&self, text: &str) -> Vec<RubySpan> {
        let mut stmt = self
            .db
            .prepare(
                r#"--sql
            select start_index, end_index, rt
            from text_entry
            join ruby_entry on ruby_entry.text_entry_id = text_entry.id
            where text = ?1
            order by start_index asc, end_index desc -- longest options first
        "#,
            )
            .unwrap();
        stmt.query_map([text], |r| {
            Ok(RubySpan {
                start_index: r.get(0)?,
                end_index: r.get(1)?,
                ruby_text: r.get(2)?,
            })
        })
        .unwrap()
        .map(|r| r.unwrap())
        .collect::<Vec<_>>()
    }

    pub fn generate_rubies(&self, f: FormatRuby, input: &str) -> String {
        let res = self.tokenizer.tokenize_with_details(input).unwrap();

        let mut output = String::new();

        for token in res {
            if token.text.as_ref().chars().any(IsJapaneseChar::is_kanji) {
                let details = token.details.unwrap();
                if let [_, _, _, _, _, _, dictionary_form, _reading, _pronunciation] = &details[..]
                {
                    let suggestions = self.suggest_rubies(dictionary_form);
                    let with_rubies = apply_suggested_rubies(f, &token.text, &suggestions);
                    output.push_str(&with_rubies);
                } else {
                    output.push_str(&token.text);
                }
            } else {
                output.push_str(&token.text);
            }
        }

        output
    }
}

pub fn format_ruby_text(format_string: &str, base: &str, ruby_text: &str) -> String {
    format_string.replace("%b", base).replace("%t", ruby_text)
}

fn apply_suggested_rubies(format: FormatRuby, text: &str, rubies: &[RubySpan]) -> String {
    // assuming the rubies are already sorted
    let text = text.chars().collect::<Vec<_>>();
    let (last_index, mut s) =
        rubies
            .iter()
            .fold((0, String::new()), |(valid_next_index, mut s), ruby| {
                if ruby.start_index >= valid_next_index {
                    s.push_str(
                        &text[valid_next_index..ruby.start_index]
                            .iter()
                            .collect::<String>(),
                    );
                    let base = &text[ruby.start_index..=ruby.end_index]
                        .iter()
                        .collect::<String>();
                    let text = &ruby.ruby_text;

                    s.push_str(&format(base, text));
                    (ruby.end_index + 1, s)
                } else {
                    (valid_next_index, s)
                }
            });

    s.push_str(&text[last_index..].iter().collect::<String>());
    s
}
