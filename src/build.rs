use std::{
    fs,
    io::{BufRead, BufReader},
    num::ParseIntError,
};

const DB_PATH: &'static str = "./data/furi.db3";
const DICT_PATH: &'static str = "./data/furigana_dictionary.txt";

use nom::{
    branch::alt,
    bytes::complete::{take_till1, take_until, take_while, take_while1},
    character::{
        complete::{char, digit1},
        is_digit,
    },
    combinator::{map, map_res, opt, rest},
    multi::{separated_list0, separated_list1},
    number,
    sequence::{preceded, separated_pair, tuple},
    IResult,
};
use rusqlite::Connection;
use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(transparent)]
struct FuriganaDictionary<'a>(#[serde(borrow)] pub Vec<FuriganaEntry<'a>>);

#[derive(Deserialize, Debug)]
struct FuriganaEntry<'a> {
    pub text: &'a str,
    pub reading: &'a str,
    pub rubies: Vec<RubyEntry<'a>>,
}

#[derive(Deserialize, Debug)]
struct RubyEntry<'a> {
    pub start_index: u8,
    pub end_index: u8,
    pub rt: &'a str,
}

fn take_range(input: &str) -> IResult<&str, (u8, u8)> {
    map_res(
        tuple((digit1, opt(preceded(char('-'), digit1)))),
        |(start, end): (&str, Option<&str>)| {
            let start: u8 = start.parse()?;
            let end = if let Some(end) = end {
                end.parse()?
            } else {
                start
            };
            Ok::<_, ParseIntError>((start, end))
        },
    )(input)
}

fn take_ruby(input: &str) -> IResult<&str, RubyEntry> {
    map(
        separated_pair(take_range, char(':'), take_till1(|c| c == '\n' || c == ';')),
        |((start_index, end_index), rt)| RubyEntry {
            start_index,
            end_index,
            rt,
        },
    )(input)
}

fn take_rubies(input: &str) -> IResult<&str, Vec<RubyEntry>> {
    separated_list0(char(';'), take_ruby)(input)
}

fn dictionary_line(input: &str) -> IResult<&str, FuriganaEntry> {
    map(
        tuple((
            take_until("|"),
            char('|'),
            take_until("|"),
            char('|'),
            take_rubies,
        )),
        |(text, _, reading, _, rubies)| FuriganaEntry {
            text,
            reading,
            rubies,
        },
    )(input)
}

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=./data/furigana_dictionary.txt");

    let dictionary_file = fs::File::open(DICT_PATH).unwrap();
    let dictionary_reader = BufReader::new(dictionary_file);

    println!("Creating database file {DB_PATH}");

    let db = Connection::open(DB_PATH).unwrap();
    db.execute_batch(
        r#"--sql
create table if not exists text_entry (
    id              integer primary key,
    text            text not null,
    reading         text not null
);

create index if not exists idx_text_entry_text on text_entry(text);

create table if not exists ruby_entry (
    id              integer primary key,
    text_entry_id   integer not null,
    start_index     byte not null,
    end_index       byte not null,
    rt              text not null,
    foreign key(text_entry_id) references text_entry(id)
);
    "#,
    )
    .unwrap();

    println!("Writing records to database");

    for line in dictionary_reader.lines() {
        let line = line.unwrap();
        let (_, entry) = dictionary_line(&line).unwrap();

        db.execute(
            r#"--sql
        insert into text_entry (text, reading) values (?1, ?2);
        "#,
            (&entry.text, &entry.reading),
        )
        .unwrap();

        let id = db
            .query_row(
                r#"--sql
        select last_insert_rowid() as id
        "#,
                (),
                |r| r.get::<_, u64>(0),
            )
            .unwrap();

        for ruby in entry.rubies {
            db.execute(r#"--sql
            insert into ruby_entry (text_entry_id, start_index, end_index, rt) values (?1, ?2, ?3, ?4);
            "#, (&id, &ruby.start_index, &ruby.end_index, &ruby.rt)).unwrap();
        }
    }

    println!("Done creating dictionary database");
}
