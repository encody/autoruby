use std::io::BufReader;

use rusqlite::Connection;

const DB_PATH: &str = "./data/furi.db3";
const DICT_PATH: &str = "./data/furigana_dictionary.txt";

#[path = "./src/dictionary.rs"]
mod dictionary;

#[path = "./src/parse.rs"]
mod parse;

#[cfg(not(feature = "dict-autobuild"))]
fn main() {}

#[cfg(feature = "dict-autobuild")]
#[tokio::main]
async fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={DICT_PATH}");

    #[cfg(feature = "dict-autobuild-download")]
    let dictionary_file = {
        if let Ok(file) = std::fs::File::open(DICT_PATH) {
            file
        } else {
            let dictionary_file = reqwest::get(dictionary::DICT_URL).await.unwrap().bytes().await.unwrap();
            std::fs::write(DICT_PATH, &dictionary_file).unwrap();
            std::fs::File::open(DICT_PATH).unwrap()
        }
    };
    #[cfg(feature = "dict-autobuild-bundled")]
    let dictionary_file = dictionary::DICT_BUNDLED;
    #[cfg(not(any(
        feature = "dict-autobuild-bundled",
        feature = "dict-autobuild-autodownload"
    )))]
    let dictionary_file = std::fs::File::open(DICT_PATH).unwrap();

    let dictionary_reader = BufReader::new(dictionary_file);

    let db = Connection::open(DB_PATH).unwrap();

    dictionary::build(dictionary_reader, &db);
}
