use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

use autoruby::{
    dictionary,
    format::{self, Format},
};
use clap::{Args, Parser, Subcommand, ValueEnum};
use serde::Deserialize;

const DB_FILENAME: &str = "annotations.db3";
const ENV_PREFIX: &str = "AUTORUBY_";

#[derive(Deserialize, Debug)]
struct Config {
    #[serde(default = "default_data_dir")]
    data_dir: PathBuf,
    #[serde(default = "default_db_path")]
    db_path: PathBuf,
}

fn project_dirs() -> Option<directories::ProjectDirs> {
    directories::ProjectDirs::from("io", "GeekLaunch", "autoruby-cli")
}

fn default_data_dir() -> PathBuf {
    project_dirs()
        .map(|p| p.cache_dir().to_path_buf())
        .or_else(|| directories::BaseDirs::new().map(|c| c.cache_dir().to_path_buf()))
        .unwrap()
}

fn default_db_path() -> PathBuf {
    default_data_dir().join(DB_FILENAME)
}

async fn download_dictionary(db_path: impl AsRef<Path>) {
    let dict_text = dictionary::download().await.unwrap();
    dictionary::build(
        dict_text.as_bytes(),
        &rusqlite::Connection::open(db_path).unwrap(),
    );
}

#[derive(Parser, Debug)]
#[command(author, version)]
/// Command-line utility for adding ruby text to documents
struct Arguments {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Download dictionary from the Internet
    DownloadDict,
    /// Remove data directory
    Clean {
        /// Are you sure?
        #[arg(short, long)]
        yes: bool,
    },
    /// Annotate text
    Annotate(AnnotateArgs),
}

#[derive(Args, Debug)]
struct AnnotateArgs {
    /// File to read input from, otherwise STDIN
    input_path: Option<PathBuf>,

    /// File to write output to, otherwise STDOUT
    output_path: Option<PathBuf>,

    /// Detect if the dictionary exists and download it if necessary
    #[arg(short, long)]
    auto_download: bool,

    /// Include common kanji readings.
    #[arg(short = 'c', long)]
    include_common: bool,

    /// Output mode
    #[arg(value_enum, long, short)]
    mode: OutputMode,
}

fn input(input_path: Option<impl AsRef<Path>>) -> String {
    input_path
        .map(|p| fs::read_to_string(p).expect("Could not read input file."))
        .unwrap_or_else(|| {
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .expect("Must specify input file or STDIN.");
            buf
        })
}

fn output(output_path: Option<impl AsRef<Path>>) -> Box<dyn Write> {
    output_path
        .map(|o| {
            Box::new(fs::File::create(o).expect("Could not create output file.")) as Box<dyn Write>
        })
        .unwrap_or_else(|| Box::new(std::io::stdout()) as Box<dyn Write>)
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum OutputMode {
    Markdown,
    Html,
    Latex,
}

impl OutputMode {
    pub fn formatter(&self) -> Format {
        match self {
            OutputMode::Markdown => format::markdown,
            OutputMode::Html => format::html,
            OutputMode::Latex => format::latex,
        }
    }
}

async fn download_dict_command(config: &Config) {
    fs::create_dir_all(&config.data_dir).unwrap();
    eprintln!("Downloading dictionary to {}...", config.db_path.display());
    download_dictionary(&config.db_path).await;
    eprintln!("Done downloading dictionary.");
}

#[tokio::main]
async fn main() {
    let args = Arguments::parse();
    let config: Config = envy::prefixed(ENV_PREFIX).from_env().unwrap();

    match args.command {
        Command::DownloadDict => {
            download_dict_command(&config).await;
        }
        Command::Clean { yes } => {
            if yes {
                eprintln!("Removing {}...", config.data_dir.display());
                fs::remove_dir_all(config.data_dir).unwrap();
                eprintln!("Done.");
            } else {
                eprintln!(
                    "No action performed. Pass the --yes flag to remove {}.",
                    config.data_dir.display()
                );
            }
        }
        Command::Annotate(a) => {
            if !config.db_path.exists() {
                if a.auto_download {
                    download_dict_command(&config).await;
                } else {
                    panic!("Dictionary not found: {}. Run `autoruby download-dict` to automatically download the dictionary, or re-run this command with the --auto-download flag.", config.db_path.display());
                }
            }

            let input_text = input(a.input_path);

            let processor = autoruby::annotate::Annotator::new(&config.db_path, !a.include_common);

            let generated = processor.annotate(a.mode.formatter(), &input_text);

            output(a.output_path)
                .write_all(generated.as_bytes())
                .expect("Could not write output.");
        }
    }
}

#[cfg(test)]
mod tests {
    use autoruby::format;

    #[test]
    fn test_complex_short() {
        let processor =
            autoruby::annotate::Annotator::new("../autoruby/data/annotations.db3", true);
        let result = processor.annotate(format::markdown, "全単射。");
        assert_eq!(result, "[全]{ぜん}[単]{たん}[射]{しゃ}。",);
    }

    #[test]
    fn test_simple() {
        let processor =
            autoruby::annotate::Annotator::new("../autoruby/data/annotations.db3", false);
        let result = processor.annotate(
            format::markdown,
            "神は「光あれ」と言われた。すると光があった。",
        );
        assert_eq!(
            result,
            "[神]{かみ}は「[光]{ひかり}あれ」と[言]{い}われた。すると[光]{ひかり}があった。",
        );
    }

    #[test]
    fn test_complex_long() {
        let processor =
            autoruby::annotate::Annotator::new("../autoruby/data/annotations.db3", true);
        let result = processor.annotate(
            format::markdown,
            "数学において、全単射あるいは双射とは、写像であって、その写像の終域となる集合の任意の元に対し、その元を写像の像とする元が、写像の定義域となる集合に常にただ一つだけ存在するようなもの、すなわち単射かつ全射であるような写像のことを言う。",
        );
        assert_eq!(
            result,
            "数学において、[全]{ぜん}[単]{たん}[射]{しゃ}あるいは[双]{そう}[射]{しゃ}とは、[写]{しゃ}[像]{ぞう}であって、その[写]{しゃ}[像]{ぞう}の[終]{おわり}[域]{いき}となる集合の任意の[元]{もと}に[対]{たい}し、その[元]{もと}を[写]{しゃ}[像]{ぞう}の像とする[元]{もと}が、[写]{しゃ}[像]{ぞう}の[定]{てい}[義]{ぎ}[域]{いき}となる集合に常にただ一つだけ存在するようなもの、すなわち[単]{たん}[射]{しゃ}かつ[全]{ぜん}[射]{しゃ}であるような[写]{しゃ}[像]{ぞう}のことを言う。"
        );
    }
}
