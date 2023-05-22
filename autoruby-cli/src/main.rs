use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

use autoruby::format::{self, Format};
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

#[tokio::main]
async fn main() {
    let args = Arguments::parse();

    match args.command {
        Command::Annotate(a) => {
            let input_text = input(a.input_path);

            let annotator = autoruby::annotate::Annotator::new_with_integrated_dictionary();

            let annotated = annotator.annotate(&input_text);

            let generated = if a.include_common {
                annotated.apply_all_with_first(a.mode.formatter())
            } else {
                annotated.apply_uncommon_with_first(a.mode.formatter())
            };

            output(a.output_path)
                .write_all(generated.as_bytes())
                .expect("Could not write output.");
        }
    }
}
