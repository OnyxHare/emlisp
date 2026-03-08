# emlisp

Rust で書いた最小構成の Lisp 実装です。REPL で式を評価できます。

## できること

- 数値リテラル (`42`, `3.14`)
- シンボル参照 (`answer`)
- 変数定義 (`(define answer 42)`)
- 四則演算 (`+`, `-`, `*`, `/`)
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
