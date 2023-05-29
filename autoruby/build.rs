#![allow(unused)]

#[cfg(feature = "integrated")]
#[path = "./src/dictionary.rs"]
mod dictionary;

#[cfg(feature = "integrated")]
#[path = "./src/parse.rs"]
mod parse;

#[cfg(feature = "integrated")]
#[tokio::main]
async fn main() {
    use std::{io::BufRead, path::PathBuf};

    dotenvy::dotenv().ok();

    let out_dir = std::env::var("OUT_DIR").unwrap();

    let cache_dir = std::env::var("AUTORUBY_CACHE_DIR").unwrap_or_else(|_| out_dir.clone());

    let furigana_path: PathBuf = [&cache_dir, "./furigana.txt"].iter().collect();
    let bin_path: PathBuf = [&out_dir, "./dict.bin"].iter().collect();

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=.env");

    let dictionary_reader = {
        #[cfg(feature = "dummy")]
        {
            let dummy_dictionary = "有る|ある|0:あ\n";
            Box::new(dummy_dictionary.as_bytes()) as Box<dyn BufRead>
        }

        #[cfg(not(feature = "dummy"))]
        if let Ok(file) = std::fs::File::open(&furigana_path) {
            Box::new(std::io::BufReader::new(file)) as Box<dyn BufRead>
        } else {
            let dictionary_file = reqwest::get(dictionary::DOWNLOAD_URL)
                .await
                .unwrap()
                .text()
                .await
                .unwrap();
            std::fs::write(&furigana_path, dictionary_file).unwrap();
            let file = std::fs::File::open(&furigana_path).unwrap();
            Box::new(std::io::BufReader::new(file)) as Box<dyn BufRead>
        }
    };

    let dict = dictionary::build(dictionary_reader);
    std::fs::write(bin_path, bincode::serialize(&dict).unwrap()).unwrap();
}

#[cfg(not(feature = "integrated"))]
fn main() {}
