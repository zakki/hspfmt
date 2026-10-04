# Rust移行作業計画

C++実装をRustで再実装し、C++実装を廃止する。現在の利用形態はCLIで、長期的には
`hsp3-analyzer-mini`（hsp3-ginger）などを通じたIDE組み込みを視野に入れる。

## 方針

- 各段階で「C++版と同一の振る舞い」を検証可能な状態を保ち、振る舞いの変更は意図したものだけに限る。
- 互換性の基準は言語中立なゴールデンテストとし、C++とRustで同じテスト資産を使う。
- 設計の改善はRust側（段階4）で行い、C++側の変更は移植しやすさに必要な範囲に留める。
- IDE向け機能そのものは作らず、ライブラリとして切り出せる形だけを先に確保する。
- `AGENTS.md` の保持要件（バイト境界、BOM、CRLF/LF、末尾改行、コメント・文字列の内容、
  トークン不変性、冪等性、`--write` の安全性）は全段階を通じて維持する。

## 段階1: テストの充実と言語中立化

現在の回帰テスト（`test/test_hspfmt.cpp` の約300件）はC++ APIを直接呼んでおり、Rustで再利用できない。
テストを増やす前に形式を移す。

### 作業

- [x] **互換性の契約範囲を決める**
  - 整形結果（バイト単位）: 完全一致（改行コードCRLF/LF、BOM、末尾改行、CP932/UTF-8のバイト列を保持）。
  - エラーの有無・行番号・メッセージ文字列:
    - 終了ステータス: 正常時 0, `--check` で差分あり時 1, エラー時 2。
    - 形式: `hspfmt: [<path>:]<line>: <message>\n`（行番号不明またはCLI引数エラー時は line なし）。
    - Rust移植時も同一のエラーメッセージ文字列を出力することを契約とする。
  - 曖昧箇所のdiagnostics:
    - 終了ステータス: 0（警告が出ても成功）。
    - 形式: `<path>:<line>: warning: ambiguous label or multiplication; preserving whitespace\n<line preview>\n`。
  - `--write` の安全性契約:
    - 複数ファイル指定時、すべてのファイルの解析・整形が成功するまでどのファイルも上書きしない（アトミック性）。
    - 整形差分がないファイルは書き込み・更新を行わない（mtime を保持）。
    - シンボリックリンクやハードリンク、ディレクトリの指定を拒否する。
  - 標準出力への書き込み失敗（`--help` を含む）:
    - `hspfmt: [<path>: ]output write failed` を出力し、終了ステータス 2。
    - 出力先のパイプが閉じられた場合（`hspfmt a.hsp | head` など）も同じ扱いとする。
      C++版はSIGPIPEで終了（ステータス141）しており、`--help` の書き込み失敗は検出していなかった。
      Rust版で契約に合わせた意図した変更。
- [x] **ゴールデンケースの形式を定める**
  - `test/cases/<name>/`:
    - `input.hsp`: 入力ソースコード（バイナリ保持）。
    - `args` (任意): CLI引数（1行1引数、デフォルトで `--no-config` が前置される）。
    - `config` (任意): `--config` に渡す設定ファイル。
    - `stdin` (任意): 存在する場合は標準入力経由で入力。
    - `expected.hsp`: 期待標準出力（バイナリ保持）。
    - `expected.err`: 期待標準エラー出力（エラーメッセージまたは診断警告）。
    - `status`: 期待終了ステータス（省略時は `expected.err` があり `expected.hsp` がなければ 2、それ以外は 0）。
    - `no_idempotent` (任意): 冪等性チェックをスキップする場合に配置。
  - `.gitattributes` で `test/cases/** -text` を指定し、Git による改行コード・文字コード変換を完全抑止。
- [x] **CLI経由のテストランナーを作る**
  - `test/run_tests.py`: Python 3 製。フォーマッタの実行ファイルパスを受け取り、458ケースとCLI振る舞いテストを実行。
  - 出力一致、エラー一致、診断一致、終了コード、および冪等性（整形出力を再入力として不変であること）を検証。
  - 当初はCTest（`hspfmt_cases`）に登録。段階3以降は `tests/golden.rs` から `cargo test` で実行する。
- [x] **既存テストを移行する**
  - `test/test_hspfmt.cpp` の全 416 ケース、`cli_errors.cmake`、`cli_spacing.cmake`、`output_failure.cmake` をゴールデンケース（計 458 ケース + 8 つの振る舞いテスト）へ移行完了。
  - 字句解析の往復テスト（lex roundtrip）は当初 `test/test_hspfmt.cpp` 内に残した。段階3で同ファイルを削除し、
    `src/bin/corpus.rs`（`cargo test` から `hspfmt_corpus test` として実行）の往復検証が引き継いでいる。
- [x] **コーパスのスナップショットを取る**
  - 対象: `tmp/sample`、`tmp/chsp`、`hsp3-ginger`（326ファイル）。
  - `test/corpus_snapshot.py` を作成し、6つのプロファイル（`default`, `compact`, `structured`, `tabs`, `c_style`, `roundtrip`）で計 1956 件の整形結果 SHA-256 ハッシュを `test/corpus_snapshot.json` に記録。
  - `--verify` で全 1956/1956 件の一致を確認。
- [x] **カバレッジを測定して穴を埋める**
  - gcov で測定:
    - `src/hspfmt.cpp`: **99.19%** (975/983 lines)
    - `src/main.cpp`: **99.58%** (237/238 lines, 未通過行 0)
  - 未通過分岐とその理由:
    - `hspfmt.cpp:50-51`: `validate_utf8` のマルチバイト途中不正。detect_encoding の他候補判定が先行するため到達しない。
    - `hspfmt.cpp:161`: `invalid UTF-8 code point`（サロゲート等）。字句解析以前のエンコーディング検証で拒否されるため。
    - `hspfmt.cpp:314`: 入力ソースが空の場合の早期リターン。
    - `hspfmt.cpp:856`: `spacing changed token boundaries`。トークン不変性の防御的アサーション。

### 完了条件

- [x] 既存のC++テストがすべてゴールデンケースとして表現されている。
- [x] C++版でランナー（459/459 pass）、コーパス比較（1956/1956 pass）、`test/integration.sh` が通る。
- [x] 主要な分岐のカバレッジが確認されている（未通過のものは理由を記録）。

## 段階2: C++版のリファクタリング（移植しやすさに限定）

段階1のテストが揃ってから着手する。設計改善はRust側（段階4）で行うので、ここでは行わない。

### 作業

- [x] `format()` 内の `flush` ラムダが多数の状態をキャプチャしている構造を、明示的な状態構造体（`FormatContext`）とメンバ関数（`flush`）に分離完了。
  （Rustの借用規則でそのまま写せないクロージャの多重キャプチャを解消）
- [x] `std::runtime_error` を投げて外側で `Error(..., line_start)` に包み直す方式を、
  発生箇所（`close_block`、`parse_line`、`flush`）で直接 `Error(..., line)` を投げる形に整理完了。
- [x] 前段パス（全角スペース正規化、コメント書き換え）、メインパス（字句解析・インデント・行内整形）、後段パス（`short_if_lines`、`declaration_layout`）の
  順序と入出力をコメントで明文化完了。
- [x] 移植時の対応表として、関数一覧と役割を記録完了（下記参照）。

### 関数対応表（C++ -> Rust）

段階3では関数単位で一対一に移植し、ライブラリは `src/lib.rs` の1ファイルにまとめた。
当初予定していたモジュール分割（`encoding`、`lexer`、`parser`、`formatter`）は段階4の候補とする。

| C++ 関数 / 構造体 | 役割 | Rust 実装 |
| --- | --- | --- |
| `hspfmt::detect_encoding` | UTF-8 / CP932 / BOM / Undetermined 判定 | `pub fn detect_encoding(&[u8]) -> Result<Encoding, Error>` |
| `validate_utf8`, `validate_cp932` | バイト列の妥当性検証 | `check_utf8`, `check_cp932` |
| `normalize_full_width_spaces` | 全角スペース（`\u3000`, `\x81\x40`）の正規化（前段パス） | `normalize_full_width_spaces`（非公開） |
| `rewrite_comments` | 行・ブロックコメント記法変換（前段パス） | `rewrite_comments`（非公開） |
| `hspfmt::lex` | ソース全体の完全字句解析（trivia・BOM保持） | `pub fn lex(&[u8], Encoding) -> Result<Vec<Token>, Error>` |
| `character_end` | 1文字境界の判定（CP932/UTF-8対応） | `character_end`（判定済みの文字コードを受け取る） |
| `close_block`, `parse_line` | ブロック構造・ステートメントの追跡 | `close_block`, `parse_line` |
| `short_if` | 単一行 if 文への折りたたみ処理 | `short_if` |
| `expression_parens` | 条件式・repeat式の括弧追加/削除 | `expression_parens` |
| `operator_spelling` | 演算子スタイル（HSP/C）、インクリメントスタイルの変換 | `operator_spelling` |
| `print_items` | トークン間の空白正規化、曖昧箇所（`*`）の検出 | `print_items` |
| `declaration_commas` | `#deffunc` 等の引数カンマ空白正規化 | `declaration_commas` |
| `FormatContext::flush` | 1論理行のインデント決定・出力バッファフラッシュ | `FormatContext::flush` |
| `short_if_lines` | 複数行 if の単一行折りたたみ（後段パス1） | `short_if_lines`（非公開） |
| `declaration_layout` | モジュール・関数宣言前の空行調整（後段パス2） | `declaration_layout`（非公開） |
| `hspfmt::format` | 公開整形API（前段・メイン・後段パイプライン調整） | `pub fn format(&[u8], &Options, Option<&mut Vec<Diagnostic>>) -> Result<Vec<u8>, Error>` |
| `main.cpp` 引数・設定解析 | CLIオプション・`.hspfmt` 設定ファイル解析 | `src/main.rs` の `parse_formatting_option`, `load_config`, `run` |
| `replace_file` | 一時ディレクトリ・アトミック書き換え処理 | `src/main.rs` の `replace_file` |

### 完了条件

- [x] 段階1のテストがすべて通り、コーパスのスナップショットが変わらない。
  - CTest 5/5 PASSED
  - ゴールデンケース 459/459 PASSED
  - コーパススナップショット 1956/1956 PASSED
  - 統合テスト（`test/integration.sh`） PASSED

## 段階3: 忠実なRust実装への差し替え

### 作業

- [x] Cargoプロジェクトを作る
  - **lib crateとbin crateに分ける**
  - ライブラリはファイルI/O、stderr出力、`process::exit`、入力に起因するpanicを行わない
- [x] 関数単位でC++と一対一に移植する（名前と構成を揃え、左右で見比べてレビューできるようにする）
  - ソースは `&[u8]` / `Vec<u8>` で扱う（CP932とバイト境界を保持するため `str` にしない）
  - 大文字小文字の変換などはASCII限定であることを確認して移植する
  - 例外は `Result` に置き換える
- [x] CLI（`main.cpp`）を移植する。引数解析、設定ファイル、プリセット、エラー文言を段階1の契約どおりに再現する
- [x] `test/corpus.cpp` をRustに移植する（`src/bin/corpus.rs`）
- [x] **C++版をオラクルにした差分検証**
  - ゴールデンケース（459件）とコーパスのスナップショット（1956件）を両実装で実行し全件一致を確認
  - 差分ファジング（`test/diff_fuzz.py`）によりランダム入力5,000回でstdout、stderr、終了コードの完全一致を確認
  - C++版の基準点はコミット `28d57e4`。再検証が必要な場合はこのコミットをCMakeでビルドし、
    `python3 test/diff_fuzz.py <C++版hspfmt> target/release/hspfmt 5000` で比較する
    （`diff_fuzz.py` は任意の2つのビルドを比較する汎用ツールとして残している）
- [x] 差し替える
  - ビルドをCMakeからCargoへ移し、C++ソースとCMake定義を削除
  - `test/integration.sh`、`test/integration_chsp.sh` が参照するバイナリのパスを更新
  - `AGENTS.md`、`README.md`、`DESIGN.md` のビルド・テスト手順と構成を更新
  - CI設定（`.github/workflows/ci.yml`）をCargoベースに更新

### 完了条件

- [x] ゴールデンケース（459/459 PASSED）、コーパス比較（1956/1956 PASSED）、統合テスト（`test/integration.sh` PASSED）がRust版で通る。
  - 差し替え後のケース追加により、段階3完了時は 462/462（461ケース + 振る舞いテスト）。
  - `test/integration_chsp.sh` はcHSP環境がある場合に実行する（未実施の場合はその旨を記録する）。
- [x] 差分ファジングを5,000回実行し、不一致がない。
- [x] リポジトリにC++実装が残っていない。

## 段階4: C++移植由来のひずみの除去

### 実施内容

- [x] 各パスの入力と出力を、借用可能なトークン列 `Document` に統一する
  - 全角スペース正規化、コメント変換、行内整形、複数行 `short_if`、宣言前の空行調整は、
    ソース全体のバイト列への書き出し・再lexを挟まずにトークンを受け渡す。
  - 文字コードは入口で一度だけ解決し、`Auto` を持たない内部型 `ResolvedEncoding` を渡す。
  - 安全性のため、行内整形結果のトークン境界照合は残す。
    正規化で分割されたWordと隣接トークンも、小数・指数表記等へ分類するため局所的にlexする。
- [x] `src/lib.rs` を `encoding`、`lexer`、`parser`、`formatter`、`document`、
  `options`、`config`、`error`、`util` に分割する
- [x] `Error::line()` と `Error::new()` の行番号を `Option<usize>` にする
  - 行番号は1始まり、位置なしは `None`。`byte_offset()` は0始まりの入力バイト位置を返す。
    字句エラーは検出位置、構造エラーは該当する原文行の先頭、設定エラーはオプションの先頭。
- [x] `Item` のテキストと空白を `Cow<[u8]>` にし、未変更部分は原文を借用する
- [x] `State` の関数・ラベル状態をbool、cHSP開始行を `Option<usize>` にする
  - プリプロセッサの分岐状態も `Option<State>` にし、別の整数・有効フラグを持たない。
- [x] CLIの解析方式を検討し、標準ライブラリによる現行方式を維持する
  - clapの導入は不要。外部依存ゼロと既存のCLIエラー文言を維持する。
- [x] IDE組み込みに備えた公開APIを用意する
  - `format_utf8(&str, ...) -> Result<String, Error>` はUTF-8を明示して処理する。
  - `Diagnostic` は警告種別、原文の行番号・行先頭バイト位置・行プレビューを返す。
  - `Options`、`config::parse_config`、`config::parse_formatting_option`、`config::find_config` を公開する。
    設定探索はカレントディレクトリを引数で受け取り、存在確認を呼び出し元へ委譲し、
    ライブラリ自体はファイルI/Oを行わない。
  - 内部パスは非公開。`detect_encoding` と `lex` は引き続き公開する。
- [x] Unixの `--write` で置換後に親ディレクトリを同期する
  - パーミッションも一時ファイルの同期前に設定する。
  - 置換後の同期失敗は「置換済み」を明記して終了ステータス2にする。
    置換前の失敗時の原文保持と無変更時の無書き込みは維持する。
- [x] stableのCIに `cargo fmt --check` と `cargo clippy --all-targets -- -D warnings` を追加する
  - Linux stableのCIには `cargo check --lib --target wasm32-unknown-unknown` も追加する。

### 完了条件と検証結果

- [x] ゴールデンとCLI振る舞いテスト: **464/464 PASSED**（463ケース + 振る舞いテスト）。
  既存の期待値は変更していない。UTF-8/BOM/CRLFとCP932の複合変換ケース2件を追加し、
  期待値は段階3のバイナリで作成した。
- [x] ライブラリAPIテスト5件、テスト資産のトークン不変性・冪等性検証が成功。
- [x] コーパススナップショット: **1956/1956 PASSED**。
- [x] 段階3のRustバイナリとの差分ファジング: **5,000回 PASSED**。
  追加のUTF-8/CP932複合入力2,000件も一致。
- [x] OpenHSP統合テストとcHSP統合テスト（plugin/Cの両モード）が成功。
- [x] `cargo build --release`、`cargo test --release`、`cargo fmt --check`、
  `cargo clippy --all-targets -- -D warnings` が成功。

ローカルにはwasm32とRust 1.70のツールチェーンがないため、これらのビルドは未実施。
CIでwasm32、Rust 1.70、Windowsを検証する。

## IDE組み込みへの備え

IDE向けの機能（LSPサーバー、範囲整形、インクリメンタル解析、ham-coreとのパーサ共有）は作らない。
長期的には、hspfmtのcrateを `hsp3-analyzer-mini` に組み込み、
`ham-core/src/ide/formatting.rs` の簡易フォーマッタを置き換える形を想定する。

短期的には、既存CLIのstdin入力と `--stdin-filepath` を使えば、設計を変えずにエディタから呼び出せる。

後から困らないよう、次を守る。

| 項目 | 時期 |
| --- | --- |
| lib/binの分離、ライブラリでI/O・exit・panicを行わない | 段階3 |
| エラーとdiagnosticsを構造化データ（行番号、可能ならバイトオフセット）で返す | 段階4 |
| UTF-8の経路を一級に扱う（`&[u8]` を基本に、`&str` の入口も用意） | 段階4 |
| `Options`、設定ファイルの解析、設定ファイルの探索をライブラリ側に置く | 段階4 |
| ライブラリをwasm32でビルド可能に保つ（スレッドやOS依存APIを使わない） | 段階3以降 |

ham-coreに組み込む際は、hspfmtとhsp3-gingerのライセンスの互換性を確認する。

## リスクと対策

| リスク | 対策 |
| --- | --- |
| テストが捉えていない挙動の差 | カバレッジ測定、コーパスのスナップショット、差分ファジング |
| エラー文言などCLI挙動の意図しない変化 | 段階1で契約範囲を明文化し、ゴールデンで固定 |
| 文字コード処理の崩れ | バイト列で処理、CP932/BOM/CRLFのバイナリケースを維持 |
| C++側リファクタリングでの退行 | 段階1完了後に着手し、範囲を移植しやすさに限定 |
| 段階2と段階4の二度手間 | 設計改善はRust側に集約 |
| OpenHSP環境がなく統合テストを実行できない | 未実施であることを報告し、実行できる環境で後追い検証 |
