((rust-mode
  . ((indent-tabs-mode . nil)
     (fill-column . 100)
     (rust-format-on-save . t)
     (compile-command . "cargo test -- --nocapture")))
 (rustic-mode
  . ((indent-tabs-mode . nil)
     (fill-column . 100)
     (rustic-format-on-save . t)
     (compile-command . "cargo test -- --nocapture"))))
