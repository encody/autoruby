#[cfg(feature = "dict-autodownload")]
#[tokio::main]
async fn main() {
    const DEFAULT_DB_PATH: &str = "./data/annotations.db3";
    const DEFAULT_DICT_PATH: &str = "./data/dictionary.txt";

    #[path = "./src/dictionary.rs"]
    mod dictionary;

    #[path = "./src/parse.rs"]
    mod parse;

    dotenvy::dotenv().ok();

    let db_path = &std::env::var("AUTORUBY_DB_PATH")
        .ok()
        .unwrap_or_else(|| DEFAULT_DB_PATH.to_string());
    let dict_path = &std::env::var("AUTORUBY_DICT_PATH")
        .ok()
        .unwrap_or_else(|| DEFAULT_DICT_PATH.to_string());

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=.env");
    println!("cargo:rerun-if-changed={dict_path}");

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
