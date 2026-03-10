# emlisp

Rust で書いた最小構成の Lisp 実装です。REPL で式を評価できます。

## できること

- 数値リテラル (`42`, `3.14`)
- シンボル参照 (`answer`)
- 変数定義 (`(define answer 42)`) ※ 再定義不可（不変）
- 四則演算 (`+`, `-`, `*`, `/`)
- 条件分岐 (`(if cond then else)`)
- ブール/`nil` リテラル (`true`, `false`, `nil`)
- Atom リテラル (`:ok` など)
- パイプライン演算 (`(|> value (+ 1) (* 2))`)
- 出力 (`(print expr)`)

## 実行

```bash
cargo run
```

REPL 例:

```lisp
(define answer (+ 40 2))
answer
(* answer 2)
(if true :ok :error)
(|> 5 (+ 3) (* 2))
```

## emlisp-mode (Emacs major mode)

`emlisp-mode.el` を同梱しています。`.emlisp` ファイルで次を提供します。

- `define` / `if` / `print` / `|>` のキーワードハイライト
- `+ - * /` の組み込み演算子ハイライト
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
