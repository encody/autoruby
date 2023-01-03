use std::io::BufRead;

use rusqlite::Connection;

use crate::parse::dictionary_line;

/// Transactions of how many dictionary entries worth of insertions at a time?
const BATCH_SIZE: usize = 1000;

pub fn build(input_reader: impl BufRead, db: &Connection) {
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
}
