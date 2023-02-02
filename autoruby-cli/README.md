# autoruby-cli

Automatically generate furigana/ruby for various document formats.

## Usage

**Input and output paths specified, Markdown**

```text
$ cat ./input.txt
神は「光あれ」と言われた。すると光があった。

$ autoruby annotate ./input.txt ./output.md --mode markdown
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
