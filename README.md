# autoruby

Automatically generate furigana/ruby for various document formats.

Currently planning to support:

- Markdown
- HTML
- LaTeX

Maybe:

- .DOCX

Not:

- PDF

## Setup

The tool works using an embedded database of the Jmdict furigana as provided by [Doublevil](https://github.com/Doublevil/JmdictFurigana).

To generate the database for yourself, first download the text file either manually from the above link, or by using the included `download-dictionary.sh` script.

Once you have the dictionary file, run the `build.rs` script to compile it into an SQLite database.

## Usage

Currently, it doesn't do much very well. Still a work-in-progress.

```text
$ cargo run -- --mode html ./test.txt > test.html
```

That's all for now!

## Authors

- Jacob Lindahl [@sudo_build](https://twitter.com/sudo_build)
