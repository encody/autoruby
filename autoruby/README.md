# autoruby

Automatically generate furigana/ruby for various document formats.

## Setup

The tool works using an embedded database of the JMdict furigana as provided by [Doublevil](https://github.com/Doublevil/JmdictFurigana).

To generate the database for yourself, first download the text file either manually from the above link, or enable the `integrated` feature.

If the `integrated` feature is enabled, you can run the `build.rs` script to compile the dictionary into a binary database. (It will run automatically, either by your IDE or when running `cargo build`.) The dictionary and database files will be saved to path in the [`OUT_DIR` environment variable](https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-build-scripts).

## Authors

- Jacob Lindahl [@sudo_build](https://twitter.com/sudo_build)
