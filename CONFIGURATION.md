# hspfmt 設定ガイド

`hspfmt` の設定ファイルおよび整形オプションの詳細解説です。各設定項目について、設定ファイルでの記述方法、コマンドラインオプション、および整形前後の実例（Before / After）を記載しています。

## 目次

- [設定の適用方法](#設定の適用方法)
  - [設定ファイルの書式](#設定ファイルの書式)
  - [設定ファイルの読み込み規則](#設定ファイルの読み込み規則)
  - [プリセット設定](#プリセット設定)
- [インデント（字下げ）](#インデント字下げ)
  - [インデント幅 / 原文維持 (`--indent`)](#インデント幅--原文維持---indent)
  - [タブ文字インデント (`--tabs`)](#タブ文字インデント---tabs)
  - [通常コードの基本字下げ (`--base-indent`)](#通常コードの基本字下げ---base-indent)
  - [ループごとの字下げ (`--loop-indent`)](#ループごとの字下げ---loop-indent)
  - [ラベル以降の字下げ (`--indent-labels`)](#ラベル以降の字下げ---indent-labels)
  - [説明コメントのインデント揃え](#説明コメントのインデント揃え)
- [空白（スペーシング）](#空白スペーシング)
  - [二項演算子周辺の空白 (`--operator-spacing`)](#二項演算子周辺の空白---operator-spacing)
  - [カンマ周辺の空白 (`--comma-spacing`)](#カンマ周辺の空白---comma-spacing)
  - [コロン周辺の空白 (`--colon-spacing`)](#コロン周辺の空白---colon-spacing)
  - [コメント前の空白 (`--comment-spacing`)](#コメント前の空白---comment-spacing)
- [構文スタイル・表記の統一](#構文スタイル表記の統一)
  - [二項演算子の表記スタイル (`--operator-style`)](#二項演算子の表記スタイル---operator-style)
  - [増減文の表記スタイル (`--increment-style`)](#増減文の表記スタイル---increment-style)
  - [基数接頭辞のHSP表記統一 (`--hsp-prefixes`)](#基数接頭辞のhsp表記統一---hsp-prefixes)
- [構文変換と括弧](#構文変換と括弧)
  - [単一行ifブロックのコロン形式変換 (`--short-if`, `--line-width`)](#単一行ifブロックのコロン形式変換---short-if---line-width)
  - [条件式の括弧 (`--condition-parens`)](#条件式の括弧---condition-parens)
  - [repeat引数の括弧 (`--repeat-parens`)](#repeat引数の括弧---repeat-parens)
- [コメントと空行](#コメントと空行)
  - [行コメント記号の統一 (`--comment-style`)](#行コメント記号の統一---comment-style)
  - [ブロックコメント形式の変換 (`--block-comments`)](#ブロックコメント形式の変換---block-comments)
  - [宣言前の空行数 (`--blank-lines-before-*`)](#宣言前の空行数---blank-lines-before-)
- [文字コードと文字正規化](#文字コードと文字正規化)
  - [文字エンコーディングの指定 (`--encoding`)](#文字エンコーディングの指定---encoding)
  - [全角スペースの半角化 (`--full-width-spaces`)](#全角スペースの半角化---full-width-spaces)
- [cHSP構文の整形](#chsp構文の整形)
- [整形除外コメント（ディレクティブ）](#整形除外コメントディレクティブ)
  - [複数行の除外 (`hspfmt: off` / `hspfmt: on`)](#複数行の除外-hspfmt-off--hspfmt-on)
  - [単一行の除外 (`hspfmt: ignore`)](#単一行の除外-hspfmt-ignore)

---

## 設定の適用方法

### 設定ファイルの書式

設定ファイルは1行に1つのオプションを記述します。
先頭の `--` は省略可能です。`#` または `;` で始まる行はコメント行として扱われます。
オプションはASCIIで記述し、コメント行にはUTF-8/CP932の日本語を記述できます。行の前後の半角スペースとタブは無視され、LF/CRLFの改行に対応します。

```ini
# .hspfmt の例
tabs
base-indent=1
loop-indent=0
operator-spacing=compact
comma-spacing=compact
colon-spacing=space
comment-spacing=space
operator-style=hsp
increment-style=c
condition-parens=remove
repeat-parens=remove
```

> [!NOTE]
> `--write` や `--check` などの実行モード指定は、設定ファイルには記述できません。

### 設定ファイルの読み込み規則

- **自動読み込み**: カレントディレクトリに `.hspfmt` が存在する場合、既定で自動的に読み込まれます。
- **ファイル指定 (`--config=FILE`)**: 任意の設定ファイルを指定して読み込みます。
- **無効化 (`--no-config`)**: 設定ファイルの自動読み込みを行いません。
- **優先順位**: コマンドライン引数は、設定ファイルの設定内容を上書きします。

### プリセット設定

配布サンプルや一般的なコーディングスタイルに合わせた3種類のプリセットを用意しています。`.hspfmt` にコピーするか、`--config=FILE` で指定して利用できます。

| プリセット | 通常コードの基本字下げ | ループごとの字下げ | コロン周辺 | 主な構文変換 |
| --- | --- | --- | --- | --- |
| [`.hspfmt.example`](.hspfmt.example)（HSPサンプル準拠） | タブ1段 | 増やさない | 1スペース | 演算子をHSPスタイル、増減文を `c`（`++`/`--`）、条件式・repeatの括弧を除去 |
| [`presets/compact.hspfmt`](presets/compact.hspfmt) | タブ1段 | タブ1段 | 空白なし | 構文変換なし（空白を最小限にするスタイル） |
| [`presets/structured.hspfmt`](presets/structured.hspfmt) | なし（ラベル本体はタブ1段） | タブ1段 | 1スペース | 構文変換なし（構造化を重視するスタイル） |

---

## インデント（字下げ）

### インデント幅 / 原文維持 (`--indent`)

- **CLI**: `--indent=N` / `--indent=preserve`
- **設定ファイル**: `indent=N` / `indent=preserve`
- **指定可能な値**: `0` 〜 `16`、または `preserve`（既定値: `4`）

インデントに用いる半角スペースの数を指定します。`preserve` を指定すると、元の行頭の空白（タブ・スペース）をそのまま維持します。

```hsp
; 整形前
repeat 2
mes "hello"
loop

; --indent=4（既定値）
repeat 2
    mes "hello"
loop

; --indent=2
repeat 2
  mes "hello"
loop
```

### タブ文字インデント (`--tabs`)

- **CLI**: `--tabs`
- **設定ファイル**: `tabs`（または無効化時は `no-tabs`）

字下げに半角スペースではなく、1段につきタブ1文字（`\t`）を使用します。

```hsp
; 整形前
repeat 2
mes "hello"
loop

; --tabs 指定時（\t で字下げ）
repeat 2
	mes "hello"
loop
```

### 通常コードの基本字下げ (`--base-indent`)

- **CLI**: `--base-indent=N`
- **設定ファイル**: `base-indent=N`
- **指定可能な値**: `0` 〜 `16`（既定値: `0`）

スクリプト全体の基本字下げ段数を指定します。HSPの公式サンプルなどで見られる「スクリプト全体をタブ1段インデントして書き始める」スタイルに対応できます。ラベルやモジュール宣言自体は基本字下げの対象外です。また、ラベル本体や関数本体の字下げ段数とは大きい方を採用し、加算はされません。

```hsp
; 整形前
x = 1
repeat 2
x += cnt
loop

; --tabs --base-indent=1 指定時
	x = 1
	repeat 2
		x += cnt
	loop
```

### ループごとの字下げ (`--loop-indent`)

- **CLI**: `--loop-indent=N`
- **設定ファイル**: `loop-indent=N`
- **指定可能な値**: `0` 〜 `16`（既定値: `1`）

`repeat`、`foreach`、`while`、`for`、`do` のループ1つにつき増やす字下げ段数を指定します。HSPサンプル準拠のように「ループ本体では字下げを増やさない」場合は `0` を指定します（その場合でも `if` の波括弧や `switch` では字下げが増えます）。

```hsp
; 整形前
repeat 2
mes "loop"
if flag {
mes "if"
}
loop

; --loop-indent=1（既定値）
repeat 2
    mes "loop"
    if flag {
        mes "if"
    }
loop

; --loop-indent=0 指定時
repeat 2
mes "loop"
if flag {
    mes "if"
}
loop
```

### ラベル以降の字下げ (`--indent-labels`)

- **CLI**: `--indent-labels` / `--no-indent-labels`
- **設定ファイル**: `indent-labels` / `no-indent-labels`
- **既定値**: `no-indent-labels`（下げない）

ラベル（`*label`）以降、次のラベルや関数・モジュール境界までの本文を1段字下げします。

```hsp
; 整形前
*init
x = 0
y = 0
*main
x++
goto *main

; --no-indent-labels（既定値）
*init
x = 0
y = 0
*main
x++
goto *main

; --indent-labels 指定時
*init
    x = 0
    y = 0
*main
    x++
    goto *main
```

### 説明コメントのインデント揃え

関数宣言（`#deffunc` など）やモジュール宣言、ラベルの直前に連続する単独行コメントは、その宣言・ラベルに対する説明コメントとして認識され、宣言と同じ字下げレベルに自動的に揃えられます。

```hsp
; 整形前（--indent-labels 指定時）
*main
    x = 1
    ; 次の処理へ進む
*next
    x = 2

; 整形後（--indent-labels 指定時）
*main
    x = 1
; 次の処理へ進む
*next
    x = 2
```

---

## 空白（スペーシング）

### 二項演算子周辺の空白 (`--operator-spacing`)

- **CLI**: `--operator-spacing=preserve|space|compact`（`--compact-operators` は `compact` の短縮形）
- **設定ファイル**: `operator-spacing=preserve|space|compact`
- **既定値**: `space`

二項演算子（`+`, `-`, `*`, `/`, `\`, `=`, `+=`, `-=` など）の前後の空白を制御します。
`compact` を指定した場合でも、トークン同士が結合して意味が変わってしまう箇所（例: `x - -1` のハイフン重複）には必要な最小限のスペースが維持されます。

```hsp
; 整形前
a=b+1
c   =   d * 2
e+=3

; --operator-spacing=space（既定値）
a = b + 1
c = d * 2
e += 3

; --operator-spacing=compact 指定時
a=b+1
c=d*2
e+=3
```

### カンマ周辺の空白 (`--comma-spacing`)

- **CLI**: `--comma-spacing=preserve|space|compact`
- **設定ファイル**: `comma-spacing=preserve|space|compact`
- **既定値**: `space`

命令の引数や配列宣言などのカンマ周辺の空白を制御します。`space` はカンマの後に1スペースを配置し、カンマ前の余分な空白を除去します。

```hsp
; 整形前
mesbox t1,160,32,0
pos 10 , 20

; --comma-spacing=space（既定値）
mesbox t1, 160, 32, 0
pos 10, 20

; --comma-spacing=compact 指定時
mesbox t1,160,32,0
pos 10,20
```

### コロン周辺の空白 (`--colon-spacing`)

- **CLI**: `--colon-spacing=preserve|space|compact`
- **設定ファイル**: `colon-spacing=preserve|space|compact`
- **既定値**: `space`

1行に複数の文を並べるコロン（`:`）の前後の空白を制御します。

```hsp
; 整形前
a = 1:b = 2:c = 3
x = 1   :   y = 2

; --colon-spacing=space（既定値）
a = 1 : b = 2 : c = 3
x = 1 : y = 2

; --colon-spacing=compact 指定時
a = 1:b = 2:c = 3
x = 1:y = 2
```

### コメント前の空白 (`--comment-spacing`)

- **CLI**: `--comment-spacing=preserve|space|compact`
- **設定ファイル**: `comment-spacing=preserve|space|compact`
- **既定値**: `space`

文の末尾にある行コメント（`;` または `//`）と、その直前のコードとの間の空白を制御します。

```hsp
; 整形前
x = 1;初期化
y = 2       ;位置設定

; --comment-spacing=space（既定値）
x = 1 ;初期化
y = 2 ;位置設定

; --comment-spacing=compact 指定時
x = 1;初期化
y = 2;位置設定

; --comment-spacing=preserve 指定時
x = 1;初期化
y = 2       ;位置設定
```

---

## 構文スタイル・表記の統一

### 二項演算子の表記スタイル (`--operator-style`)

- **CLI**: `--operator-style=preserve|hsp|c`
- **設定ファイル**: `operator-style=preserve|hsp|c`
- **既定値**: `preserve`（原文維持）

式中の比較演算子・論理演算子の表記を統一します。代入演算子（`=`）や複合代入は変換されず、式中の比較・論理演算子が対象です。

| 表記 | HSPスタイル (`hsp`) | Cスタイル (`c`) |
| --- | --- | --- |
| 等価比較 | `=` | `==` |
| 不等比較 | `!` | `!=` |
| 論理積 / ビット積 | `&` | `&&` |
| 論理和 / ビット和 | <code>&#124;</code> | <code>&#124;&#124;</code> |

```hsp
; 整形前
if (a == 1) & (b != 2) : x = (c == d)

; --operator-style=hsp 指定時
if (a = 1) & (b ! 2) : x = (c = d)

; --operator-style=c 指定時
if (a == 1) && (b != 2) : x = (c == d)
```

> [!NOTE]
> HSPにおける `&&` と `||` はそれぞれ `&` と `|` の別表記であり、Cスタイルに統一してもビット演算のままで短絡評価（ショートサーキット評価）にはならない点にご注意ください。

### 増減文の表記スタイル (`--increment-style`)

- **CLI**: `--increment-style=preserve|hsp|c`
- **設定ファイル**: `increment-style=preserve|hsp|c`
- **既定値**: `preserve`（原文維持）

変数や配列要素のインクリメント・デクリメント文の表記を統一します。HSPにおいて増減は独立した文であり、式中の演算子は変換されません。

| 表記 | HSPスタイル (`hsp`) | Cスタイル (`c`) |
| --- | --- | --- |
| インクリメント | `a+` | `a++` |
| デクリメント | `a-` | `a--` |

```hsp
; 整形前
a+
b--
arr(cnt)+

; --increment-style=hsp 指定時
a+
b-
arr(cnt)+

; --increment-style=c 指定時
a++
b--
arr(cnt)++
```

### 基数接頭辞のHSP表記統一 (`--hsp-prefixes`)

- **CLI**: `--hsp-prefixes`
- **設定ファイル**: `hsp-prefixes`
- **既定値**: 無効（原文維持）

16進数リテラルの接頭辞 `0x` を `$` に、2進数リテラルの接頭辞 `0b` を `%` に統一します。桁数、英字の大文字/小文字、アンダースコア（桁区切り）は保持されます。

```hsp
; 整形前
color = 0xff00ff
flag = 0b1010_0011

; --hsp-prefixes 指定時
color = $ff00ff
flag = %1010_0011
```

---

## 構文変換と括弧

### 単一行ifブロックのコロン形式変換 (`--short-if`, `--line-width`)

- **CLI**: `--short-if` / `--line-width=N`
- **設定ファイル**: `short-if` / `line-width=N`
- **既定値**: 無効 / `line-width=100`

波括弧 `{ }` で囲まれた単純な文（代入、複合代入、増減文、`return`、通常の命令呼び出し）からなる `if` ブロックを、1行のコロン形式（`if 条件 : 文`）に短縮します。波括弧と本文が複数行に分かれている場合でも、本文が1文であれば1行に短縮されます。

`--line-width=N` は短縮後の1行の最大バイト数（インデント込み）を指定します。この上限を超える場合は短縮せず波括弧ブロックのまま残します。

```hsp
; 整形前
if (flag) {
    x = 1
}
if z == 0 { return }

; --short-if 指定時
if (flag) : x = 1
if z == 0 : return
```

> [!NOTE]
> `else` が後続するブロック、ループや `switch` を開閉する文、コメントやラベル、プリプロセッサ行を含むブロックは変換対象外です。

### 条件式の括弧 (`--condition-parens`)

- **CLI**: `--condition-parens=preserve|add|remove`
- **設定ファイル**: `condition-parens=preserve|add|remove`
- **既定値**: `preserve`（原文維持）

`if` および `while` の条件式全体を囲む外側の括弧を統一します。式内部の演算優先順位を決める括弧や関数呼び出しの括弧は保持されます。

```hsp
; 整形前
if (a > 0) : x = 1
while x < 10
    x++
wend

; --condition-parens=remove 指定時
if a > 0 : x = 1
while x < 10
    x++
wend

; --condition-parens=add 指定時
if (a > 0) : x = 1
while (x < 10)
    x++
wend
```

### repeat引数の括弧 (`--repeat-parens`)

- **CLI**: `--repeat-parens=preserve|add|remove`
- **設定ファイル**: `repeat-parens=preserve|add|remove`
- **既定値**: `preserve`（原文維持）

`repeat` 命令の各引数を囲む外側の括弧を統一します。

```hsp
; 整形前
repeat (10), (start + 1)
loop

; --repeat-parens=remove 指定時
repeat 10, start + 1
loop

; --repeat-parens=add 指定時
repeat (10), (start + 1)
loop
```

---

## コメントと空行

### 行コメント記号の統一 (`--comment-style`)

- **CLI**: `--comment-style=preserve|semicolon|c`
- **設定ファイル**: `comment-style=preserve|semicolon|c`
- **既定値**: `preserve`（原文維持）

行コメントの開始記号をセミコロン（`;`）またはC言語風スラッシュ（`//`）に統一します。行末のコメントも対象となります。

```hsp
; 整形前
// 初期設定
x = 0 ; カウンタ

; --comment-style=semicolon 指定時
; 初期設定
x = 0 ; カウンタ

; --comment-style=c 指定時
// 初期設定
x = 0 // カウンタ
```

### ブロックコメント形式の変換 (`--block-comments`)

- **CLI**: `--block-comments=preserve|lines|block`
- **設定ファイル**: `block-comments=preserve|lines|block`
- **既定値**: `preserve`（原文維持）

独立した行のブロックコメント（`/* ... */`）と連続した行コメントの相互変換を行います。

- `lines`: 単独行の `/* ... */` を改行とインデントを維持したまま行コメント（`;` または `//`）に展開します。
- `block`: 連続する独立した行コメントをひとまとめのブロックコメントに結合します。

```hsp
; 整形前
/*
 * 関数の説明
 * 引数: なし
 */
x = 1

; --block-comments=lines 指定時（--comment-style=semicolon 併用時）
;
; * 関数の説明
; * 引数: なし
;
x = 1
```

### 宣言前の空行数 (`--blank-lines-before-*`)

- **CLI**:
  - `--blank-lines-before-module=N|preserve`
  - `--blank-lines-before-deffunc=N|preserve`
  - `--blank-lines-before-defcfunc=N|preserve`
- **設定ファイル**:
  - `blank-lines-before-module=N|preserve`
  - `blank-lines-before-deffunc=N|preserve`
  - `blank-lines-before-defcfunc=N|preserve`
- **指定可能な値**: `0` 〜 `16`、または `preserve`（既定値: `preserve`）

モジュール宣言（`#module`, `#chsp_module`）やユーザー定義命令・関数宣言（`#deffunc`, `#defcfunc`, `#chsp_deffunc`, `#chsp_defcfunc`）の手前の空行数を統一します。
宣言の直前にドキュメントコメント（説明コメント）が連続している場合は、そのコメント群の手前に空行が挿入されます。ファイルの先頭行には空行は追加されません。

```hsp
; 整形前
x = 1
#deffunc my_func
    return

; --blank-lines-before-deffunc=2 指定時
x = 1


#deffunc my_func
    return
```

---

## 文字コードと文字正規化

### 文字エンコーディングの指定 (`--encoding`)

- **CLI**: `--encoding=auto` / `--encoding=utf8` / `--encoding=cp932`
- **設定ファイル**: `encoding=auto` / `encoding=utf8` / `encoding=cp932`
- **既定値**: `auto`（自動判定）

ソースファイルの文字コードを指定します。既定値の `auto` では、ファイルの内容を検査して UTF-8 と CP932（Shift_JIS）を自動判定します。

- UTF-8 BOM（`0xEF, 0xBB, 0xBF`）が存在する場合は確実に UTF-8 として処理されます。
- BOM がない場合でもバイト列を検証し、UTF-8 または CP932 を自動判別します（非ASCII文字を含まないASCIIのみのコードは UTF-8 として扱われます）。
- `--write` や `--check` で複数ファイルを一括指定した場合でも、ファイルごとに個別にエンコーディングが自動判定されます。
- 特定のエンコーディングとして厳密に検証したい場合は、明示的に `utf8` または `cp932` を指定してください。

なお、`hspfmt` は文字コードの変換（トランスコード）は行いません。UTF-8 / CP932 どちらの文字コードであっても元のバイト列を維持したまま、CP932 の2バイト目に現れる `0x5C`（`\`）などを演算子やエスケープと誤認しないよう正確な文字境界で整形します。

### 全角スペースの半角化 (`--full-width-spaces`)

- **CLI**: `--full-width-spaces=preserve|normalize`
- **設定ファイル**: `full-width-spaces=preserve|normalize`
- **既定値**: `preserve`（原文維持）

文字列リテラルやコメントの外側にある全角スペース（U+3000、CP932では `0x8140`）を半角スペースへ正規化します。

```hsp
; 整形前（全角スペースが含まれている）
repeat　2
　　mes　"こんにちは"
loop

; --full-width-spaces=normalize 指定時
repeat 2
    mes "こんにちは"
loop
```

---

## cHSP構文の整形

cHSP（OpenHSPのネイティブコード拡張）のファイル（`.chsp`）や構文は、特別なオプションなしで自動認識されます。

- `#chsp_module` 〜 `#chsp_module_end` および `#chsp_deffunc` / `#chsp_defcfunc` 〜 `#chsp_end` をブロックとして認識し、本体を適切にインデントします。
- 単一行の型付き宣言（`array[int]`, `local[int[2][3]]`, `-> int` など）のカンマ周辺に `--comma-spacing` を適用します。
- 埋め込みCコード（`#chsp_c {"..."}`）や `#chsp_cdecl`、`#chsp_clink` は原文のまま保持されます。

```hsp
; 整形前
#chsp_module "math" target=c
#chsp_defcfunc add_values array[int] vals,int n -> int
total=0
repeat n
total+=vals(cnt)
loop
return total
#chsp_end
#chsp_module_end

; 整形後（既定設定）
#chsp_module "math" target=c
#chsp_defcfunc add_values array[int] vals, int n -> int
    total = 0
    repeat n
        total += vals(cnt)
    loop
    return total
#chsp_end
#chsp_module_end
```

---

## 整形除外コメント（ディレクティブ）

手動で整列させたテーブルデータや、マクロによって構文が崩れる箇所を整形から除外できます。

### 複数行の除外 (`hspfmt: off` / `hspfmt: on`)

`; hspfmt: off` から `; hspfmt: on` で囲まれた範囲は、字下げや空白の変更を行わず、原文のまま出力します。この区間内ではブロックの解析も一時停止します。

```hsp
; 整形対象コード
x = 1 : y = 2

; hspfmt: off
; コロンや代入の列を手動で揃えたデータテーブル
item_id(0) = 101 : item_name(0) = "potion"  : item_price(0) =  100
item_id(1) = 102 : item_name(1) = "ether"   : item_price(1) =  250
item_id(2) = 103 : item_name(2) = "elixir"  : item_price(2) = 1500
; hspfmt: on

; 再び整形対象
item_count = 3
```

### 単一行の除外 (`hspfmt: ignore`)

直後の1行のみを原文のまま保持します。インデントや末尾空白も含めて完全に維持されます。`hspfmt: off` と異なりブロック解析は継続するため、直後の行が `repeat` や `if` の開始行であっても後続行のインデント計算は正常に行われます。

```hsp
; hspfmt: ignore
raw_matrix = 1.0,  0.0,  0.0,   0.0,  1.0,  0.0
next_var = 1
```

> [!NOTE]
> ディレクティブは `; hspfmt: off` または `// hspfmt: off` のように、行頭（インデント後）の独立した行コメントとして記述してください。行末コメントや文字列内の記述はディレクティブとして解釈されません。
