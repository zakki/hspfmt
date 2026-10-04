# hspfmt — HSP formatter

HSP（Hot Soup Processor）およびcHSPのマクロ展開前のソースコードを整形する、Rust製の高速なフォーマッタライブラリおよびCLIです。
外部ツールやランタイムに依存せず、CRLF/LFの改行形式や文字エンコーディング（UTF-8/CP932）を正確に保持しながらソースコードを安全に整形します。

## インストールとビルド

### ソースコードからのビルド

Rust 1.70以上（Cargo）が必要です。

```sh
cargo build --release
```

ビルドされた実行ファイルは `target/release/hspfmt` に生成されます。

テストの実行:

```sh
cargo test --release
```

### Windows用バイナリ (ZIP)

GitHub Actionsの [CIワークフロー](.github/workflows/ci.yml) でビルドされたWindows 64bit用バイナリをダウンロードできます。

**Actions → CI → 対象のワークフロー実行 → Artifacts → hspfmt-windows-x64.zip**

ZIPには `hspfmt.exe`、`README.md`、`CONFIGURATION.md`、`DESIGN.md`、`LICENSE`、`.hspfmt.example`、および `presets/` が同梱されています。

---

## 使い方

```sh
# 標準出力へ整形結果を出力
target/release/hspfmt script.hsp

# ファイルを直接上書き更新
target/release/hspfmt --write script.hsp

# 整形差分の有無を検査（CI用）
target/release/hspfmt --check script.hsp

# 複数ファイルを一括更新
target/release/hspfmt -w file1.hsp file2.hsp

# 設定ファイルを指定して実行
target/release/hspfmt --config=.hspfmt.example script.hsp
```

### 入出力と終了コード

- ファイル名を省略するか `-` を指定すると標準入力から読み込みます。
- **通常実行**: 成功時は `0`、入出力エラーや構文解析エラー時は `2` を返します。
- **`--check`**: 整形差分がなければ `0`、差分があれば `1`、エラー時は `2` を返します。
- **`--write`, `-w`**: 整形結果で元ファイルを直接置換します。差分がない場合はファイルに触れず、タイムスタンプを維持します。成功時は `0` を返します。

`--write` はシンボリックリンクとハードリンクを拒否し、UnixのパーミッションとWindowsのACLを保持します。Unixでは置換後に親ディレクトリも同期します（親ディレクトリを読み取りで開けない場合は同期を省略します）。
置換後の同期に失敗した場合は、置換済みである旨のエラーを表示し、終了コード `2` を返します。ファイルパスはOSの文字列表現のまま扱うため、UnixのUTF-8以外のファイル名も指定できます。

設定ファイルのオプションはASCIIで記述します。`#` または `;` で始まるコメント行には、UTF-8/CP932の日本語を記述できます。

> [!WARNING]
> シェルの上書きリダイレクト（例: `hspfmt script.hsp > script.hsp`）は入力ファイルが空に切り詰められるため絶対に行わないでください。ファイルの上書きには必ず `--write` を使用してください。

---

## コマンドラインオプション

| オプション | 動作 | 既定値 |
| --- | --- | --- |
| `--write`, `-w` | 整形結果で入力ファイルを置換（複数ファイル指定可） | 無効 |
| `--check` | 整形差分の有無を検査（複数ファイル指定可） | 無効 |
| `--config=FILE` | 指定した設定ファイルを読み込む | `.hspfmt` |
| `--no-config` | 設定ファイルの自動読み込みを無効化 | 無効 |
| `--stdin-filepath=PATH` | 標準入力時の診断メッセージに表示する仮想ファイル名 | 空 |
| `--indent=N\|preserve` | インデント幅（0〜16）／元の字下げを維持 | `4` |
| `--base-indent=N` | 通常コードの基本字下げ段数（0〜16） | `0` |
| `--loop-indent=N` | ループ1つにつき増やす字下げ段数（0〜16） | `1` |
| `--tabs` | 1段につきタブ1文字でインデント | 空白 |
| `--indent-labels`, `--no-indent-labels` | ラベル以降の本文を1段下げる／下げない | 下げない |
| `--operator-spacing=preserve\|space\|compact` | 二項演算子の前後の空白（維持／1スペース／除去） | `space` |
| `--compact-operators` | 二項演算子の前後の余分な空白を除去（`--operator-spacing=compact` と同等） | 無効 |
| `--comma-spacing=preserve\|space\|compact` | カンマ周辺の空白（維持／カンマ後に1スペース／除去） | `space` |
| `--colon-spacing=preserve\|space\|compact` | コロンの前後の空白（維持／1スペース／除去） | `space` |
| `--comment-spacing=preserve\|space\|compact` | コードとコメントの間の空白（維持／1スペース／除去） | `space` |
| `--operator-style=preserve\|hsp\|c` | 二項演算子の表記（維持／HSPスタイル／Cスタイル） | `preserve` |
| `--increment-style=preserve\|hsp\|c` | 増減文の表記（維持／`a+`・`a-`／`a++`・`a--`） | `preserve` |
| `--hsp-prefixes` | 16進数/2進数の接頭辞をHSPスタイル（`$` / `%`）に統一 | 無効 |
| `--short-if` | 単純な文からなる `if` ブロックをコロン形式に変換 | 無効 |
| `--line-width=N` | コロン形式へ変換する行幅の上限バイト数 | `100` |
| `--condition-parens=preserve\|add\|remove` | `if`・`while` の式全体を囲む括弧（維持／追加／除去） | `preserve` |
| `--repeat-parens=preserve\|add\|remove` | `repeat` の各引数を囲む括弧（維持／追加／除去） | `preserve` |
| `--comment-style=preserve\|semicolon\|c` | 行コメント記号（維持／`;`／`//`） | `preserve` |
| `--block-comments=preserve\|lines\|block` | ブロックコメント形式（維持／行コメント化／ブロック化） | `preserve` |
| `--blank-lines-before-module=N` | モジュール宣言手前の空行数（0〜16、または維持） | 維持 |
| `--blank-lines-before-deffunc=N` | `#deffunc` 宣言手前の空行数（0〜16、または維持） | 維持 |
| `--blank-lines-before-defcfunc=N` | `#defcfunc` 宣言手前の空行数（0〜16、または維持） | 維持 |
| `--encoding=auto\|utf8\|cp932` | 入力文字エンコーディングの検証・文字境界処理（自動判定／UTF-8／CP932） | `auto` |
| `--full-width-spaces=preserve\|normalize` | 全角スペース（U+3000）を維持／半角スペースへ正規化 | `preserve` |
| `--roundtrip` | 字句解析した全トークンを無変更で連結して出力（デバッグ用） | 無効 |

---

## Rustライブラリとして使う

```rust
use hspfmt::{config::parse_config, format_utf8, Options};

let mut options = Options::default();
parse_config(b"indent=2\n", &mut options)?;
let mut diagnostics = Vec::new();
let result = format_utf8("repeat\nmes 1\nloop\n", &options, Some(&mut diagnostics))?;
```

UTF-8入力には `format_utf8(&str, ...)`、CP932や自動判定を使う入力には
`format(&[u8], ...)` を使います。`format_utf8` は `Options::encoding` をUTF-8に上書きして処理します。
ライブラリはファイルI/Oや標準出力への書き込みを行いません。

`Error::line()` は1始まりの行番号を `Option<usize>` で返します（位置なしは `None`）。
`Error::byte_offset()` は0始まりの入力バイト位置を返します。構造エラーでは該当行の先頭を指します。
入力上の位置を特定できない内部エラーでは、行番号とバイト位置はどちらも `None` です。
`Diagnostic` には警告種別 `kind`、行番号 `line`、原文行の先頭位置 `byte_offset`、
改行を除いた原文 `source` が含まれます。`DiagnosticKind` は `#[non_exhaustive]` で、警告種別は今後追加される場合があります。

設定の解析と探索は `config::parse_config`、`config::parse_formatting_option`、
`config::find_config` を使えます。`parse_config` はエラー時に `Options` を変更しません。`find_config` には探索ディレクトリと存在確認関数を渡し、
ファイルの読み込みは呼び出し元が行います。探索の優先順位はCLIと同じです。

### 0.2.0のAPI変更と互換性

0.2.0でRust移行を完了しました。0.1.0からのAPI変更として、`Error::line()` の戻り値を
`usize`（位置なしは0）から `Option<usize>`（位置なしは `None`）へ変更し、
`Error::new` を非公開にしました。`Error::byte_offset()` と `Diagnostic` の `kind`・
`byte_offset` フィールドを追加しています。`DiagnosticKind` のマッチにはワイルドカード分岐が必要です。
`format_utf8` と上記の設定解析・探索APIも追加しました。

`Options` は構造体リテラルと `..Options::default()` で構築できる公開構造体です。
公開フィールドの追加も破壊的変更として扱い、0.xではマイナーバージョンを更新します。
`#[non_exhaustive]` の `DiagnosticKind` には、互換性を保ったまま警告種別を追加できます。

---

## 関連ドキュメント

各機能のより詳しい説明や設計については、以下のドキュメントを参照してください。

- **[設定ガイド (CONFIGURATION.md)](CONFIGURATION.md)**
  設定ファイル（`.hspfmt`）やプリセット（`.hspfmt.example`, `presets/`）の使い方、各整形オプションの設定前後の実例（Before / After）、cHSP構文の整形、および整形除外コメント（`; hspfmt: off` / `; hspfmt: ignore`）について詳しく解説しています。
- **[設計思想と制約 (DESIGN.md)](DESIGN.md)**
  アーキテクチャ設計、マクロ展開前の構文解析方針、ソース保持とトークン不変性の自己検証、HSP構文の曖昧性に対するアプローチ、`--write` のアトミック置換機構、およびテスト・品質保証戦略について解説しています。

---

## ライセンス

BSD 2-Clause Licenseで公開しています。詳細は [LICENSE](LICENSE) を参照してください。
