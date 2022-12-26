use std::{
    fs,
    io::{BufRead, BufReader},
    time::Instant,
};

use rusqlite::Connection;

const DB_PATH: &'static str = "./data/furi.db3";
const DICT_PATH: &'static str = "./data/furigana_dictionary.txt";
#[cfg(feature = "dict-autodownload")]
const DICT_URL: &'static str =
    "https://github.com/Doublevil/JmdictFurigana/releases/latest/download/JmdictFurigana.txt";

#[path = "./src/parse.rs"]
mod parse;

use parse::dictionary_line;

#[tokio::main]
async fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={DICT_PATH}");

    #[cfg(feature = "dict-autodownload")]
    let dictionary_file = {
        if let Ok(file) = fs::File::open(DICT_PATH) {
            println!("found file");
            file
        } else {
            println!("downloading");
            let dictionary_file = reqwest::get(DICT_URL).await.unwrap().bytes().await.unwrap();
            println!("writing file");
            fs::write(DICT_PATH, &dictionary_file).unwrap();
            println!("reading from file");
            fs::File::open(DICT_PATH).unwrap()
        }
    };

    #[cfg(not(any(feature = "dict-bundled", feature = "dict-autodownload")))]
    let dictionary_file = fs::File::open(DICT_PATH).unwrap();
    #[cfg(feature = "dict-bundled")]
    let dictionary_file = include_bytes!("../data/furigana_dictionary.txt") as &[u8];

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

    let mut insert_text_entry = db
        .prepare(
            r#"--sql
                insert or ignore into text_entry (text, reading) values (?1, ?2);
            "#,
        )
        .unwrap();

    let mut start = Instant::now();
    let mut range_start = 0;

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
