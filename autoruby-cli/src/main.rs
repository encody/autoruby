use std::{fs, path::PathBuf};

use autoruby::format::{self, Format};
use clap::{Parser, ValueEnum};

#[derive(Parser, Debug)]
#[command(author, version)]
/// Command-line utility for adding ruby text to documents
struct Args {
    pub input_path: PathBuf,

    #[arg(value_enum, long, short)]
    pub mode: OutputMode,
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

fn main() {
    let args = Args::parse();

    let input_text = fs::read_to_string(args.input_path).unwrap();

    let processor = autoruby::text_processor::TextProcessor::new("../autoruby/data/furi.db3");

    let generated = processor.generate_rubies(args.mode.formatter(), &input_text);

    println!("{}", generated);
}

#[cfg(test)]
mod tests {
    use autoruby::format;

    #[test]
    fn test() {
        let processor = autoruby::text_processor::TextProcessor::new("../autoruby/data/furi.db3");
        let result = processor.generate_rubies(
            format::markdown,
            "神は「光あれ」と言われた。すると光があった。",
        );
        assert_eq!(
            result,
            "[神]{かみ}は「[光]{ひかり}あれ」と[言]{い}われた。すると[光]{ひかり}があった。"
        );
    }
}
