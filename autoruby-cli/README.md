# autoruby-cli

Automatically generate furigana/ruby for various document formats.

## Usage

**Input and output paths specified, Markdown (include common)**

```text
$ cat ./input.txt
神は「光あれ」と言われた。すると光があった。

$ autoruby annotate ./input.txt ./output.md --mode markdown -c
$ cat ./output.md
[神]{かみ}は「[光]{ひかり}あれ」と[言]{い}われた。すると[光]{ひかり}があった。
```

**STDIN&rarr;STDOUT, HTML**

```text
$ echo '宮崎のマンゴーとても美味しいです。' | autoruby annotate --mode html
<ruby>宮<rp>(</rp><rt>みや</rt><rp>)</rp></ruby><ruby>崎<rp>(</rp><rt>ざき</rt><rp>)</rp></ruby>のマンゴーとても美味しいです。
```

**STDIN&rarr;STDOUT, LaTeX**

```text
$ echo '千と千尋の神隠し' | autoruby annotate --mode latex
千と\ruby{千}{ち}\ruby{尋}{ひろ}の\ruby{神}{かみ}\ruby{隠}{かく}し
```

## Authors

- Jacob Lindahl [@sudo_build](https://twitter.com/sudo_build)
