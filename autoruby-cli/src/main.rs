#![doc = include_str!("../README.md")]
#![warn(clippy::pedantic)]
#![warn(missing_docs)]

use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

use autoruby::{
    format::{self, Format, WithKatakana},
    select::{self, Select},
};
use clap::{Args, Parser, Subcommand, ValueEnum};

/// Command-line utility for adding ruby text to documents
#[derive(Parser, Debug)]
#[command(author, version)]
struct Arguments {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Annotate text
    Annotate(AnnotateArgs),
}

#[derive(Args, Debug)]
struct AnnotateArgs {
    /// File to read input from, otherwise STDIN
    input_path: Option<PathBuf>,

    /// File to write output to, otherwise STDOUT
    output_path: Option<PathBuf>,

    /// Include common kanji readings.
    #[arg(short = 'c', long)]
    include_common: bool,

    /// Output format
    #[arg(value_enum, long, short = 'f')]
    format: OutputFormat,

    /// Generated furigana will use katakana instead of hiragana.
    #[arg(long, short = 'k')]
    katakana: bool,

    /// Only annotate the first occurrence of a word.
    #[arg(long, short = '1')]
    only_first: bool,
}

fn input(input_path: Option<impl AsRef<Path>>) -> String {
    input_path.map_or_else(
        || {
            let mut buf = String::new();
            std::io::stdin()
                .read_to_string(&mut buf)
                .expect("Must specify input file or STDIN.");
            buf
        },
        |p| fs::read_to_string(p).expect("Could not read input file."),
    )
}

fn output(output_path: Option<impl AsRef<Path>>) -> Box<dyn Write> {
    output_path.map_or_else(
        || Box::new(std::io::stdout()) as Box<dyn Write>,
        |o| Box::new(fs::File::create(o).expect("Could not create output file.")) as Box<dyn Write>,
    )
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, ValueEnum, Debug)]
enum OutputFormat {
    #[value(alias = "md")]
    Markdown,
    Html,
    #[value(alias = "tex")]
    Latex,
}

impl OutputFormat {
    pub fn formatter(self) -> Box<dyn Format> {
        match self {
            OutputFormat::Markdown => Box::new(format::Markdown),
            OutputFormat::Html => Box::new(format::Html),
            OutputFormat::Latex => Box::new(format::Latex),
        }
    }
}

#[tokio::main]
async fn main() {
    let args = Arguments::parse();

    match args.command {
        Command::Annotate(a) => {
            let input_text = input(a.input_path);

            let annotator = autoruby::annotate::Annotator::new_with_integrated_dictionary();

            let annotated = annotator.annotate(&input_text);

            let formatter = a.format.formatter();
            let formatter = {
                if a.katakana {
                    Box::new(WithKatakana(&*formatter))
                } else {
                    formatter
                }
            };

            let selector = match (a.only_first, a.include_common) {
                (true, true) => {
                    Box::new(select::filter::FirstOccurrence::new(select::heuristic::All))
                        as Box<dyn Select>
                }
                (true, false) => Box::new(select::filter::FirstOccurrence::new(
                    select::heuristic::UncommonOnly,
                )) as Box<dyn Select>,
                (false, true) => Box::new(select::heuristic::All) as Box<dyn Select>,
                (false, false) => Box::new(select::heuristic::UncommonOnly) as Box<dyn Select>,
            };

            let generated = annotated.render(&*selector, &*formatter);

            output(a.output_path)
                .write_all(generated.as_bytes())
                .expect("Could not write output.");
        }
    }
}
