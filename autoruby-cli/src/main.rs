use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

use autoruby::format::{self, with_katakana, Format};
use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Parser, Debug)]
#[command(author, version)]
/// Command-line utility for adding ruby text to documents
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

    /// Generated furigana will use katakana instead of hiragana
    #[arg(long, short = 'k')]
    katakana: bool,
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
enum OutputFormat {
    Markdown,
    Html,
    Latex,
}

impl OutputFormat {
    pub fn formatter(&self) -> impl Format {
        match self {
            OutputFormat::Markdown => format::markdown,
            OutputFormat::Html => format::html,
            OutputFormat::Latex => format::latex,
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

            let hiragana_formatter = a.format.formatter();
            let katakana_formatter = with_katakana(a.format.formatter());

            let formatter: &dyn Format = if a.katakana {
                &katakana_formatter
            } else {
                &hiragana_formatter
            };

            let generated = if a.include_common {
                annotated.apply_all_with_first(formatter)
            } else {
                annotated.apply_uncommon_with_first(formatter)
            };

            output(a.output_path)
                .write_all(generated.as_bytes())
                .expect("Could not write output.");
        }
    }
}
