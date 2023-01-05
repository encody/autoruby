#[cfg(feature = "dict-autodownload")]
#[path = "./src/dictionary.rs"]
mod dictionary;

#[cfg(feature = "dict-autodownload")]
#[path = "./src/parse.rs"]
mod parse;

#[cfg(feature = "dict-autodownload")]
#[tokio::main]
async fn main() {
    const DB_PATH: &str = "./data/annotations.db3";
    const DICT_PATH: &str = "./data/dictionary.txt";

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={DICT_PATH}");

    let dictionary_file = {
        if let Ok(file) = std::fs::File::open(DICT_PATH) {
            file
        } else {
            let dictionary_file = dictionary::download().await.unwrap();
            std::fs::write(DICT_PATH, dictionary_file).unwrap();
            std::fs::File::open(DICT_PATH).unwrap()
        }
    };

    let dictionary_reader = std::io::BufReader::new(dictionary_file);

    let db = rusqlite::Connection::open(DB_PATH).unwrap();

    dictionary::build(dictionary_reader, &db);
}

#[cfg(not(feature = "dict-autodownload"))]
fn main() {}
