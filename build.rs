use std::{
    fs,
    io::{BufRead, BufReader},
    num::ParseIntError,
    time::Instant,
};

const DB_PATH: &'static str = "./data/furi.db3";
const DICT_PATH: &'static str = "./data/furigana_dictionary.txt";

use nom::{
    bytes::complete::{take_till1, take_until},
    character::complete::{char, digit1},
    combinator::{map, map_res, opt},
    multi::separated_list0,
    sequence::{preceded, separated_pair, tuple},
    IResult,
};
use rusqlite::Connection;

#[derive(Debug)]
struct FuriganaEntry<'a> {
    pub text: &'a str,
    pub reading: &'a str,
    pub rubies: Vec<RubyEntry<'a>>,
}

#[derive(Debug)]
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
    println!("cargo:rerun-if-changed={DICT_PATH}");

    let dictionary_file = fs::File::open(DICT_PATH).unwrap();

    let dictionary_reader = BufReader::new(dictionary_file);

    println!("Creating database file {DB_PATH}");

    let db = Connection::open(DB_PATH).unwrap();

    db.execute_batch(
        r#"--sql
        create table if not exists text_entry (
            id              integer primary key,
            text            text not null,
            reading         text not null,
            unique(text, reading)
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

    let mut start = Instant::now();
    let mut range_start = 0;

    let mut insert_text_entry = db
        .prepare(
            r#"--sql
                insert or ignore into text_entry (text, reading) values (?1, ?2);
            "#,
        )
        .unwrap();

    db.execute_batch("begin").unwrap();

    for (i, line) in dictionary_reader.lines().enumerate() {
        if i % 1000 == 0 && i != 0 {
            // batches of 1000 insertions at a time
            db.execute_batch("commit").unwrap();
            db.execute_batch("begin").unwrap();

            println!(
                "{:.2}\trecords / second",
                ((i - range_start) as f64 / start.elapsed().as_secs_f64())
            );

            start = Instant::now();
            range_start = i;
        }

        let line = line.unwrap();
        let (_, entry) = dictionary_line(&line).unwrap();

        let rows_affected = insert_text_entry
            .execute((&entry.text, &entry.reading))
            .unwrap();

        // skipping duplicates if database already exists
        if rows_affected > 0 {
            let id = db.last_insert_rowid();

            for ruby in entry.rubies {
                db.execute(r#"--sql
                    insert into ruby_entry (text_entry_id, start_index, end_index, rt) values (?1, ?2, ?3, ?4);
                "#, (&id, &ruby.start_index, &ruby.end_index, &ruby.rt)).unwrap();
            }
        }
    }

    db.execute_batch("commit").unwrap();

    println!("Done creating dictionary database");
}
