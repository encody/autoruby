# autoruby

Automatically generate furigana/ruby for various document formats.

Currently planning to support:

- Markdown
- HTML
- LaTeX

Maybe:

- .DOCX
- EPUB
- MOBI
- Google Docs (WASM + extension?)

Not:

- PDF

## Setup

The tool works using an embedded database of the Jmdict furigana as provided by [Doublevil](https://github.com/Doublevil/JmdictFurigana).

To generate the database for yourself, first download the text file either manually from the above link, or enable the `dict-autodownload` feature on the `autoruby` crate.

If the `dict-autodownload` feature is enabled, you can run the `autoruby/build.rs` script to compile the dictionary into an SQLite database. (It will run automatically, either by your IDE or when running `cargo build`.) The dictionary and database files will be saved to the paths specified in the environment variables `AUTORUBY_DICT_PATH` and `AUTORUBY_DB_PATH`, respectively. See [`autoruby/.env.example`](./autoruby/.env.example) for defaults.

## Usage

**Download dictionary database**

```text
$ autoruby download-dict
```

**Delete database**

```text
$ autoruby clean
```

**Auto download dictionary, input and output paths specified, Markdown**

```text
$ cat ./input.txt
神は「光あれ」と言われた。すると光があった。

$ autoruby annotate ./input.txt ./output.md --mode markdown --auto-download
$ cat ./output.md
[神]{かみ}は「[光]{ひかり}あれ」と[言]{い}われた。すると[光]{ひかり}があった。

```

**STDIN&rarr;STDOUT, HTML**

```text
$ echo '何時ですか。' | autoruby annotate --mode html
<ruby>何<rp>(</rp><rt>なん</rt><rp>)</rp></ruby><ruby>時<rp>(</rp><rt>じ</rt><rp>)</rp></ruby>ですか。
```

**STDIN&rarr;STDOUT, LaTeX**

```text
$ echo '彼は大丈夫ですか。' | autoruby annotate --mode latex
\ruby{彼}{かれ}は\ruby{大}{だい}\ruby{丈}{じょう}\ruby{夫}{ぶ}ですか。
```

## Authors

- Jacob Lindahl [@sudo_build](https://twitter.com/sudo_build)
