use std::io::BufRead;

use rusqlite::Connection;

use crate::parse::dictionary_line;

pub const DOWNLOAD_URL: &str =
    "https://github.com/Doublevil/JmdictFurigana/releases/latest/download/JmdictFurigana.txt";

/// Transactions of how many dictionary entries worth of insertions at a time?
const BATCH_SIZE: usize = 1000;

pub async fn download() -> Result<String, reqwest::Error> {
    reqwest::get(DOWNLOAD_URL).await.unwrap().text().await
}

pub struct FrequencyEntry<'a> {
    kanji_element: &'a str,
    kanji_common: bool,
    reading_element: &'a str,
    reading_common: bool,
}

pub fn frequency_entries() -> impl Iterator<Item = FrequencyEntry<'static>> {
    jmdict::entries().flat_map(|e| {
        e.kanji_elements().flat_map(move |k| {
            e.reading_elements().map(move |r| FrequencyEntry {
                kanji_element: k.text,
                kanji_common: k.priority.is_common(),
                reading_element: r.text,
                reading_common: r.priority.is_common(),
            })
        })
    })
}

pub fn build(input_reader: impl BufRead, db: &Connection) {
    db.execute_batch(
        r#"--sql
            create table if not exists text_entry (
                id              integer primary key,
                text            text not null,
                text_common     boolean,
                reading         text not null,
                reading_common  boolean,
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

    let mut insert_text_entry = db
        .prepare(
            r#"--sql
                insert or ignore into text_entry (text, reading) values (?1, ?2);
            "#,
        )
        .unwrap();

    let mut insert_ruby_entry = db
        .prepare(
            r#"--sql
                insert into ruby_entry (text_entry_id, start_index, end_index, rt) values (?1, ?2, ?3, ?4);
            "#,
        )
        .unwrap();

    db.execute_batch("begin").unwrap();

    for (i, line) in input_reader.lines().enumerate() {
        if i % BATCH_SIZE == 0 && i != 0 {
            db.execute_batch("commit").unwrap();
            db.execute_batch("begin").unwrap();
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
                insert_ruby_entry
                    .execute((&id, &ruby.start_index, &ruby.end_index, &ruby.rt))
                    .unwrap();
            }
        }
    }

    db.execute_batch("commit").unwrap();

    // frequency data

    let mut insert_frequency_entry = db
        .prepare(
            r#"--sql
                update text_entry
                set text_common = ?1, reading_common = ?2
                where text = ?3 and reading = ?4
            "#,
        )
        .unwrap();

    db.execute_batch("begin").unwrap();

    for (i, frequency_entry) in frequency_entries().enumerate() {
        if i != 0 && i % BATCH_SIZE == 0 {
            db.execute_batch("commit").unwrap();
            db.execute_batch("begin").unwrap();
        }

        insert_frequency_entry
            .execute((
                &frequency_entry.kanji_common,
                &frequency_entry.reading_common,
                &frequency_entry.kanji_element,
                &frequency_entry.reading_element,
            ))
            .unwrap();
    }

    db.execute_batch("commit").unwrap();
}
