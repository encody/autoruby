# autoruby

Automatically generate furigana/ruby for various document formats.

## Example

```rust
use autoruby::{annotate, format, select};

let input = "神は「光あれ」と言われた。すると光があった。";
let expected = "[神]{かみ}は「[光]{ひかり}あれ」と[言]{い}われた。すると[光]{ひかり}があった。";
let annotator = annotate::Annotator::new_with_integrated_dictionary();
let annotated = annotator.annotate(input);
let result = annotated.render(&select::heuristic::All, &format::markdown);

assert_eq!(result, expected);
```

## Setup

The tool works using an embedded database of the JMdict furigana as provided by [Doublevil](https://github.com/Doublevil/JmdictFurigana).

To generate the database for yourself, first download the text file either manually from the above link, or enable the `integrated` feature.

If the `integrated` feature is enabled, you can run the `build.rs` script to compile the dictionary into a binary database. (It will run automatically, either by your IDE or when running `cargo build`.) The dictionary and database files will be saved to path in the [`OUT_DIR` environment variable](https://doc.rust-lang.org/cargo/reference/environment-variables.html#environment-variables-cargo-sets-for-build-scripts).

## Contributing

If you are making modifications to the code and rebuilding it often, you probably don't want to re-download the furigana dictionary for every rebuild. To avoid this, copy the `.env.example` file and rename it to `.env`. The build script will download the furigana dictionary to the directory specified by the `AUTORUBY_CACHE_DIR` (default: `./cache`) and reuse it for subsequent builds.

## Future work

- EPUB support

## Authors

- Jacob Lindahl [@sudo_build](https://twitter.com/sudo_build)
