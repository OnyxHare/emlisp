# emlisp

Rust で書いた最小構成の Lisp 実装です。REPL で式を評価できます。

## できること

- 数値リテラル（任意精度の有理数。`42`, `3.14`）
- シンボル参照 (`answer`)
- 変数定義 (`(define answer 42)`) ※ 再定義不可（不変）
- 四則演算 (`+`, `-`, `*`, `/`)
- 比較 (`<`)
- 条件分岐 (`(if cond then else)`)
- ブール/`nil` リテラル (`true`, `false`, `nil`)
- Atom リテラル (`:ok` など)
- 無名関数 (`(fn (x) (+ x 1))`)
- 無名再帰関数（`(fn (self n) ...)` のように先頭引数を `self` にする）
- `fn` の末尾再帰最適化（深い再帰でスタックを消費しにくい）
- 一部の `fn` 非末尾再帰（`if` + `+/*` + 単一自己呼び出し）を自動で末尾再帰化
- 評価時に大きめの専用スタックを使うため、非末尾再帰の上限も引き上げ
- パイプライン演算 (`(|> value (+ 1) (* 2))`)
- 出力 (`(print expr)`)

## 実行

```bash
cargo run
```

`.emlisp` ファイルを読み込む場合:

```bash
cargo run -- examples/sample.emlisp
```

REPL 起動後に読み込む場合:

```text
:load examples/sample.emlisp
```

REPL 例:

```lisp
(define answer (+ 40 2))
answer
(* answer 2)
(if true :ok :error)
(|> 5 (+ 3) (* 2))
((fn (x) (+ x 1)) 41)
((fn (self n) (if (< n 2) 1 (* n (self (- n 1))))) 5)
```

## ハノイの塔は解ける？

解けます。再帰関数で「最小手数」を計算できます。

```bash
cargo run -- examples/hanoi.emlisp
```

`examples/hanoi.emlisp` では次を実行します。

- `3` 枚の最小手数を表示（`7`）
- 最後の式として `10` 枚の最小手数（`1023`）を評価

## emlisp-mode (Emacs major mode)

`emlisp-mode.el` を同梱しています。`.emlisp` ファイルで次を提供します。

- `define` / `if` / `print` / `fn` / `|>` のキーワードハイライト
- `+ - * / <` の組み込み演算子ハイライト
- 数値・`true` / `false` / `nil`・`:atom` のハイライト
- `;` から行末までのコメント
- `lisp-indent-line` によるインデント

### 設定例

```elisp
(add-to-list 'load-path "~/path/to/emlisp")
(require 'emlisp-mode)

;; 拡張子を追加したい場合
(add-to-list 'auto-mode-alist '("\\.elispm\\'" . emlisp-mode))
```

## Emacs で開発しやすくする設定

このリポジトリには `.dir-locals.el` があり、`rust-mode` / `rustic-mode` で次を設定します。

- 保存時フォーマット
- `fill-column` を 100 に設定
- `M-x compile` で `cargo test -- --nocapture`

### おすすめパッケージ

- `rustic` または `rust-mode`
- `eglot` (または `lsp-mode`) + `rust-analyzer`
- `flycheck` / `flymake`

最低限の例（`init.el`）:

```elisp
(use-package rustic
  :ensure t
  :config
  (setq rustic-lsp-client 'eglot))

(use-package eglot
  :ensure t
  :hook (rustic-mode . eglot-ensure))
```

## テスト

```bash
cargo test
```
