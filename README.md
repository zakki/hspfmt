# hspfmt — HSP formatter

HSPのマクロ展開前のソースを整形する、C++17製の試作CLIです。PythonやNode.jsを実行時に必要としません。既定では整形結果を標準出力に出し、`--write`（`-w`）指定時は入力ファイルを上書きします。

## ビルドと実行

CMake 3.16以上とC++17対応コンパイラが必要です。

```sh
cmake -S src -B build -DCMAKE_BUILD_TYPE=Release
cmake --build build --config Release
build/hspfmt script.hsp
build/hspfmt --check script.hsp
build/hspfmt --short-if --hsp-prefixes script.hsp
build/hspfmt --write script.hsp
```

Visual Studioなどの複数構成ジェネレータでは、実行ファイルは `build/Release/hspfmt.exe` に生成されます。テスト用ツールのビルドを省く場合は、CMakeの構成時に `-DBUILD_TESTING=OFF` を指定します。

ファイル名を省略するか `-` を指定すると標準入力を読みます。`--check` は整形済みなら0、差分があれば1、入出力・解析エラーなら2を返します。通常実行のエラーも2です。入力と同じファイルへシェルでリダイレクトしないでください。`--write` または `--check` 指定時は複数ファイルを一括で指定できます。標準出力モードでは複数ファイルを指定できません。

`--write` は整形成功後、同じディレクトリ配下の一時ファイルへ書き込み、クローズ成功後に元ファイルを置換します。差分がなければファイルに触れません。標準出力には何も出さず、成功時は0を返します。アクセス許可は引き継ぎますが、POSIXの所有者・拡張属性などその他のメタデータの保持は保証しません。標準入力、シンボリックリンク、複数のハードリンクがあるファイルは対象外です。`--check`、`--roundtrip`とは併用できません。

カレントディレクトリに `.hspfmt` が存在する場合、既定で整形オプションを読み込みます。`--config=FILE` で任意の設定ファイルを指定でき、`--no-config` で読み込みを無効化できます。設定ファイルは1行に1つのオプション（先頭の `--` は省略可、`#` や `;` で始まる行はコメント）を記述します。コマンドライン引数は設定ファイルの内容を上書きします。

## 整形規則

既定では4スペースでインデントし、二項演算子の左右、カンマの後、コロンの左右にスペースを入れます。既存の改行、短い複文、識別子の大小文字、リテラルの綴りを維持します。ラベルは行頭、関数本体は1段下げます。コメントの内容を再整形しません。

`if` の波括弧、`repeat` と `foreach` の `loop`、標準マクロの `while/wend`、`for/next`、`do/until`、`switch/case/default/swend` を認識します。`#if` の各分岐は同じ入口の状態から解析します。分岐間で開いたブロックが異なる場合はエラーにします。

| オプション | 動作 |
| --- | --- |
| `--write`, `-w` | 整形結果で入力ファイルを置換（複数ファイル可） |
| `--check` | 整形差分を検査（複数ファイル可） |
| `--config=FILE` | 指定した設定ファイルを読み込む（既定: `.hspfmt`） |
| `--no-config` | 設定ファイルの読み込みを無効化 |
| `--stdin-filepath=PATH` | 標準入力時の診断メッセージに表示する仮想ファイル名 |
| `--indent=N` | インデント幅。0〜16、既定4 |
| `--tabs` | 1段につきタブ1文字 |
| `--compact-operators` | 二項演算子の任意の空白を除去。トークンの結合を防ぐ空白は残す |
| `--hsp-prefixes` | 通常コードの `0x` を `$`、`0b` を `%` に変更。桁数・大小文字・桁区切りは維持 |
| `--operator-style=preserve\|hsp\|c` | 二項演算子の表記を維持／HSP／Cスタイルに統一。既定は維持 |
| `--increment-style=preserve\|hsp\|c` | 増減文の表記を維持／`a+`・`a-`／`a++`・`a--` に統一。既定は維持 |
| `--short-if` | 独立した一行の代入のみの `if` ブロックをコロン形式に変換 |
| `--line-width=N` | コロン化する行の上限。既定100。インデント込みのバイト数で数える |
| `--encoding=utf8` | UTF-8を検証して処理。既定値 |
| `--encoding=cp932` | CP932の文字境界で処理。符号化変換はしない |
| `--roundtrip` | 字句解析した全トークンを無変更で連結して出力 |
| `--indent-labels`, `--no-indent-labels` | ラベル以降の本文を1段下げる／下げない。既定は下げない |
| `--comment-style=preserve\|semicolon\|c` | 行コメントの記号を維持／`;`／`//` に統一。既定は維持 |
| `--block-comments=preserve\|lines\|block` | 独立したコメントのブロック形式を維持／行コメント化／ブロック化。既定は維持 |
| `--condition-parens=preserve\|add\|remove` | `if`・`while` の式全体を囲む括弧を維持／追加／除去。既定は維持 |
| `--repeat-parens=preserve\|add\|remove` | `repeat` の各引数を囲む括弧を維持／追加／除去。既定は維持 |
| `--blank-lines-before-module=N` | `#module` 前の空行を0〜16行に統一。省略時は維持 |
| `--blank-lines-before-deffunc=N` | `#deffunc` 前の空行を0〜16行に統一。省略時は維持 |
| `--blank-lines-before-defcfunc=N` | `#defcfunc` 前の空行を0〜16行に統一。省略時は維持 |

`--short-if` は次の変換に対応します。

```hsp
if (flag) { foo = bar : baz = 1 }
```

```hsp
if flag : foo = bar : baz = 1
```

後続文、コメント、入れ子、`else`、配列要素への代入、命令呼び出しを含むブロックは変換対象にしません。`else` が次行に続く場合も変換しません。複数行のブロックを一行にまとめたり、長いコロン形式を複数行に展開したりする処理は未実装です。行幅は一般的な折り返し指定ではありません。

ラベル自体は常に行頭に置きます。`--indent-labels` は次のラベルまたは関数・モジュール境界までの本文を1段下げます。関数本体の字下げとは重複させず、ループなどの字下げは加算します。`return` や `goto` では解除しません。

`--comment-style` は行末のコメントも対象にします。`--block-comments=lines` は独立した `/* … */` を改行と本文を維持したまま行コメントにします。記号は `--comment-style=c` なら `//`、それ以外なら `;` です。`--block-comments=block` は連続した独立行の `;`・`//` コメントをまとめます。コード途中のブロックコメント、本文に `/*`・`*/` を含む行コメント、`hspfmt:` を含むコメントはブロック化しません。プリプロセッサ行、行継続、整形無効領域は変換しません。

括弧の除去は式全体を囲むものだけで、演算の順序を決める内部の括弧や関数呼び出しの括弧は保持します。`repeat n, start` は `repeat (n), (start)` と引数ごとに扱い、引数省略は維持します。式途中のブロックコメントや括弧の対応が不明な式は変換しません。`--short-if` と併用した場合は、コロン化後にも括弧の設定を適用します。

空行数は各宣言の直前に連続する説明コメントの前に適用し、ファイル先頭には空行を追加しません。`=preserve` を指定すると元の空行数を維持します。追加する改行は直前の行の改行形式に合わせます。

```sh
build/hspfmt --indent-labels --comment-style=semicolon --block-comments=lines \
  --condition-parens=remove --repeat-parens=remove \
  --blank-lines-before-module=2 --blank-lines-before-deffunc=1 \
  --blank-lines-before-defcfunc=1 script.hsp
```

`--operator-style` は式中の `&`／`&&`、`|`／`||`、`!`／`!=`、`=`／`==` を切り替えます。代入・複合代入の演算子は保持し、代入の右辺や配列の添字にある比較は変換します。例えば `--operator-style=c` では `x=a=b` を `x = a == b` にします。HSPの `&&` と `||` は `&` と `|` の別表記なので、Cスタイルにしてもビット演算のままで、短絡評価にはなりません。`and`・`or` などの単語による演算子は変換しません。

`--increment-style` は文全体が変数または配列要素への増減である場合に適用します。式中の加減算、`a+2` のような省略形の加算代入、`+=`・`-=` は変換しません。HSPでは増減は文であり、`x=a++` や `++a` を有効な構文に修復する処理も行いません。

命令と変数の名前解決は行わないため、未知の `foo (a)=b` のように配列への代入とも命令引数の比較とも解釈できる形式では、文頭の変数に続く最初の演算子を保持します。文字列・コメント・プリプロセッサ行・行継続・整形無効領域内の演算子は変更しません。

```sh
build/hspfmt --operator-style=c --increment-style=c script.hsp
build/hspfmt --operator-style=hsp --increment-style=hsp script.hsp
```

## ソース保持と制約

字句解析結果は元ソース上のバイト範囲です。空白、改行、コメント、BOMもトークンとして保持します。文字列のエスケープを復元・再生成せず、CP932の2バイト目を演算子や引用符と誤認しないように読みます。CRLF/LFと末尾改行の有無を維持します。

現段階の構文解析は、文境界と標準ブロック構造を扱う小さな解析器です。完全な式CSTや型・名前解決はありません。通常整形では、空白の変更がトークン境界を変えないことを再字句解析で検査します。これはマクロ展開後の意味の同一性まで保証する検査ではありません。

`foo *bar` や `foo@module *bar` は、ユーザー定義命令へのラベル引数と、変数への旧来の乗算代入のどちらにもなり得ます。この場合は `*` の直前・直後の空白をタブも含めて保持し、行内のほかの箇所は通常どおり整形します。既定の二項演算子の空白規則、または `--compact-operators` の指定と異なる場合は、ファイル名・入力の行番号・元の行を標準エラーに表示します。診断は1行につき1回で、終了コードには影響しません。`--check` と `--write` でも表示し、`--roundtrip` では表示しません。ライブラリでは `format(source, options, &diagnostics)` で `Diagnostic` の配列に診断を受け取れます。

プリプロセッサ行と行継続、複数行文字列・複数行コメントを含む行は原文のまま出力します。ただし、宣言前の空行数や独立したコメント形式の変更を明示した場合は、その指定を適用します。インクルードやマクロ展開は行いません。標準構造マクロの再定義や、構文を生成する独自マクロには未対応です。`--short-if` と括弧の変更は、マクロが対象の構文を変えないコードで利用してください。

独自構文などを除外するには、単独行のコメントで囲みます。この領域内ではブロック解析も停止するため、領域の外まで開閉がまたがらないようにしてください。

```hsp
; hspfmt: off
custom_begin
    custom_body
custom_end
; hspfmt: on
```

構文エラーをすべて検出するコンパイラ代替ではありません。不明な構造を整形で修復しません。既存プロジェクトへの一括適用前には差分とコンパイル結果を確認してください。

## テスト

```sh
ctest --test-dir build -C Release --output-on-failure
build/hspfmt_corpus test
```

C++テストは期待出力、トークンの完全再現、冪等性、文字コード、エラーを検証します。既定の `BUILD_TESTING=ON` でビルドしてください。複数構成ジェネレータではcorpusツールも `build/Release/hspfmt_corpus.exe` を使用します。

corpusツールは指定した各ディレクトリ内の `.hsp` と `.as` を読み取り、整形拒否を報告し、整形可能な入力の冪等性と空白以外のトークン不変を検証します。不変条件違反で終了コード1、整形拒否だけなら0です。符号化はUTF-8を試し、失敗した場合CP932を試します。上の例ではエラー確認用の `test/invalid.hsp` の整形拒否が報告されます。任意のHSPプロジェクトも検査できます。

```sh
build/hspfmt_corpus /path/to/hsp-project /path/to/OpenHSP/sample /path/to/OpenHSP/common
```

コンパイル・実行結果を比較する統合テストには、Linux環境の `sh`、GNU coreutils、OpenHSPの `hspcmp`、`hsp3cl`、同じ環境の `common/` が別途必要です。

```sh
HSPCMP=/path/to/OpenHSP/hspcmp \
HSP3CL=/path/to/OpenHSP/hsp3cl \
HSP_COMMON=/path/to/OpenHSP/common \
sh test/integration.sh
```

`hspcmp` と `hsp3cl` がPATHにあれば、`HSPCMP` と `HSP3CL` は省略できます。`HSP_COMMON` は必ず指定します。スクリプトの第1引数には、別のビルド先のhspfmt実行ファイルを指定できます（既定は `build/hspfmt`）。

統合テストは原文と各オプションの整形結果、および複数のオプションを組み合わせた結果をコンパイルし、実行出力を比較します。生成物は表示された一時ディレクトリに残します。整形規則の変更時は `test/test_hspfmt.cpp` に期待出力を、実行結果の検証には `test/behavior.hsp` に入力を追加してください。

## ライセンス

BSD 2-Clause Licenseで公開しています。詳細は [LICENSE](LICENSE) を参照してください。
