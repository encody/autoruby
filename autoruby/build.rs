#![allow(unused)]

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

    let furigana_path: PathBuf = [&out_dir, "./furigana.txt"].iter().collect();
    let bin_path: PathBuf = [&out_dir, "./dict.bin"].iter().collect();

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=.env");

    let dictionary_file = {
        if let Ok(file) = std::fs::File::open(&furigana_path) {
            file
        } else {
            let dictionary_file = dictionary::download().await.unwrap();
            std::fs::write(&furigana_path, dictionary_file).unwrap();
            std::fs::File::open(&furigana_path).unwrap()
        }
    };

    let dictionary_reader = std::io::BufReader::new(&dictionary_file);
    let dict = dictionary::build(dictionary_reader);
    std::fs::write(bin_path, bincode::serialize(&dict).unwrap()).unwrap();
}

#[cfg(not(feature = "dict-autodownload"))]
fn main() {}
