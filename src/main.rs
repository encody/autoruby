use std::{fs, path::PathBuf};

use clap::{Parser, ValueEnum};
use lindera::{
    mode::Mode,
    tokenizer::{Tokenizer, TokenizerConfig},
};
use rusqlite::Connection;
use wana_kana::IsJapaneseChar;

pub const DB_PATH: &'static str = "./data/furi.db3";

#[derive(Parser, Debug)]
#[command(author, version)]
/// Command-line utility for adding ruby text to documents
struct Args {
    pub input_path: PathBuf,

    #[arg(value_enum, long, short)]
    pub mode: OutputMode,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum OutputMode {
    Markdown,
    Html,
    Latex,
}

impl OutputMode {
    pub fn format_string(&self) -> &'static str {
        match self {
            OutputMode::Markdown => "[%b]{%t}",
            OutputMode::Html => "<ruby>%b<rp>(</rp><rt>%t</rt><rp>)</rp></ruby>",
            OutputMode::Latex => "\\ruby{%b}{%t}",
        }
    }
}

fn main() {
    let args = Args::parse();

    let input_text = fs::read_to_string(args.input_path).unwrap();

    let generated = generate_rubies(&input_text, args.mode.format_string());

    println!("{}", generated);
}

#[derive(Debug)]
struct Ruby {
    pub start_index: usize,
    pub end_index: usize,
    pub ruby_text: String,
}

fn lookup_rubies(db: &Connection, text: &str) -> Vec<Ruby> {
    let mut stmt = db
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
        Ok(Ruby {
            start_index: r.get(0)?,
            end_index: r.get(1)?,
            ruby_text: r.get(2)?,
        })
    })
    .unwrap()
    .map(|r| r.unwrap())
    .collect::<Vec<_>>()
}

fn format_ruby_text(format_string: &str, base: &str, ruby_text: &str) -> String {
    format_string.replace("%b", base).replace("%t", ruby_text)
}

fn apply_rubies(format_string: &str, text: &str, rubies: &[Ruby]) -> String {
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
                    let bottom = &text[ruby.start_index..=ruby.end_index]
                        .iter()
                        .collect::<String>();
                    let top = &ruby.ruby_text;

                    s.push_str(&format_ruby_text(format_string, bottom, top));
                    (ruby.end_index + 1, s)
                } else {
                    (valid_next_index, s)
                }
            });

    s.push_str(&text[last_index..].iter().collect::<String>());
    s
}

pub fn generate_rubies(format_string: &str, input: &str) -> String {
    let t = Tokenizer::with_config(TokenizerConfig {
        dictionary: lindera::tokenizer::DictionaryConfig {
            kind: Some(lindera::DictionaryKind::IPADIC),
            path: None,
        },
        user_dictionary: None,
        mode: Mode::Normal,
    })
    .unwrap();

    let db = Connection::open(crate::DB_PATH).unwrap();

    let res = t.tokenize_with_details(input).unwrap();

    let mut output = String::new();

    for token in res {
        if token.text.as_ref().chars().any(IsJapaneseChar::is_kanji) {
            let details = token.details.unwrap();
            if let [_, _, _, _, _, _, dictionary_form, _reading, _pronunciation] = &details[..] {
                let rubies = lookup_rubies(&db, dictionary_form);
                let with_rubies = apply_rubies(format_string, &token.text, &rubies);
                output.push_str(&with_rubies);
            } else {
                output.push_str(&token.text);
                // println!("Could not get dictionary form for: {}", token.text);
            }
        } else {
            output.push_str(&token.text);
            // println!("Not ruby candidate: {}", token.text);
        }
    }

    output
}

#[cfg(test)]
mod tests {
    use crate::OutputMode;

    #[test]
    fn test() {
        println!("{}", super::generate_rubies(OutputMode::Markdown.format_string(), "<p>あの年代、人類が思い出した。「カタカナ言葉」って言っちゃったが、信じられなかった。淡水湖の世界最大を思い描いてほしい。どういう感じだろう？賑やかな魚か、[カヤック](https://kayak.com/)している家族たちか？または、もしかしたら汚くて藻でいっぱいの湖なのか？</p>"));
    }
}
