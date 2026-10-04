# hspfmt 開発ガイド

このリポジトリは、マクロ展開前のHSPソースを整形するRust製のライブラリとCLIです。OpenHSPから独立してビルドできます。

## 構成

- `README.md`: 概要、インストール・ビルド方法、CLIオプション一覧、ライセンス。
- `CONFIGURATION.md`: 設定ファイル・プリセット・各オプションの詳細と実例（Before/After）。
- `DESIGN.md`: 設計思想、構文解析と制約、安全性、テスト戦略。
- `Cargo.toml`: Cargoパッケージ定義（ライブラリおよびバイナリ）。
- `src/lib.rs`: 文字コードの検証、字句解析、ブロック解析、整形ライブラリ。
- `src/main.rs`: CLI、入出力、`--check`、`--write`。
- `src/bin/corpus.rs`: 指定したHSPソース群の不変条件を検証するツール。
- `tests/golden.rs`: `cargo test` で実行される統合テスト。
- `test/run_tests.py`: 言語中立なゴールデンテストランナー。
- `test/cases/`: 個別テストケース群。
- `test/corpus_snapshot.py`: コーパススナップショット検証ツール。
- `test/integration.sh`: 外部のOpenHSPツールでコンパイル・実行結果を比較するLinux用テスト。

## ビルドと検証

リポジトリのルートで実行します。

```sh
cargo build --release
cargo test --release
cargo run --release --bin hspfmt_corpus test
```

CLIや構文変換を変更した場合は、利用可能なOpenHSP環境で統合テストも実行します。

```sh
HSPCMP=/path/to/OpenHSP/hspcmp \
HSP3CL=/path/to/OpenHSP/hsp3cl \
HSP_COMMON=/path/to/OpenHSP/common \
sh test/integration.sh
```

`HSPCMP` と `HSP3CL` の既定値はPATH上のコマンドです。`HSP_COMMON` には同じOpenHSP環境の `common/` を指定します。統合テストはGNU coreutilsを使用し、生成物を表示された一時ディレクトリに残します。実行環境がない場合は、未実施であることを報告してください。

## 変更時の方針

- Rustと標準ライブラリを使い、既存のフォーマットと命名に合わせます。
- 通常のビルド・テストを他リポジトリやHSPランタイムに依存させないでください。外部ソース群とOpenHSPは追加検証用です。
- UTF-8/CP932のバイト境界、BOM、CRLF/LF、末尾改行の有無、コメントと文字列の内容を保持します。符号化変換は行いません。
- 通常整形のトークン不変性と冪等性を維持します。意図的な構文変換は対応するオプションで有効にします。
- マクロ展開や完全なHSPコンパイラとしての解析は行いません。対応外の構造を推測で修復しないでください。
- 整形規則の変更には `test/cases/` にゴールデンケースを追加します。CLIオプションや制約の変更は `README.md` にも反映します。
- `--write` の変更では、置換前の書き込み完了、失敗時の原文保持、変更がない場合の無書き込みを維持します。
- ビルド生成物をコミットせず、`LICENSE` の著作権表示と条件を保持してください。
