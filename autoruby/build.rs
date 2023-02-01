#[cfg(feature = "dict-autodownload")]
#[path = "./src/dictionary.rs"]
mod dictionary;

#[cfg(feature = "dict-autodownload")]
#[path = "./src/parse.rs"]
mod parse;

#[cfg(feature = "dict-autodownload")]
#[tokio::main]
async fn main() {
    use std::path::PathBuf;

    dotenvy::dotenv().ok();

    let out_dir = std::env::var("OUT_DIR").unwrap();

    let default_db_path: PathBuf = [&out_dir, "./annotations.db3"].iter().collect();
    let default_dict_path: PathBuf = [&out_dir, "./dictionary.txt"].iter().collect();

    let db_path = std::env::var("AUTORUBY_DB_PATH")
        .ok()
        .map(|p| p.parse().unwrap())
        .unwrap_or(default_db_path);
    let dict_path = &std::env::var("AUTORUBY_DICT_PATH")
        .ok()
        .map(|p| p.parse().unwrap())
        .unwrap_or(default_dict_path);

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=.env");
    println!("cargo:rerun-if-changed={}", dict_path.display());

    let dictionary_file = {
        if let Ok(file) = std::fs::File::open(dict_path) {
            file
        } else {
            let dictionary_file = dictionary::download().await.unwrap();
            std::fs::write(dict_path, dictionary_file).unwrap();
            std::fs::File::open(dict_path).unwrap()
        }
    };

    let dictionary_reader = std::io::BufReader::new(dictionary_file);

    let db = rusqlite::Connection::open(db_path).unwrap();

    dictionary::build(dictionary_reader, &db);
}

#[cfg(not(feature = "dict-autodownload"))]
fn main() {}
