;;; emlisp-mode.el --- Major mode for editing emlisp files -*- lexical-binding: t; -*-

;; Author: emlisp contributors
;; Version: 0.1.0
;; Package-Requires: ((emacs "27.1"))
;; Keywords: languages, lisp

;;; Commentary:

;; `emlisp-mode' provides a lightweight major mode for the minimal Lisp
;; interpreter in this repository.

;;; Code:

(defgroup emlisp nil
  "Major mode for editing emlisp source files."
  :group 'languages)

(defcustom emlisp-mode-hook nil
  "Hook run when entering `emlisp-mode'."
  :type 'hook
  :group 'emlisp)

(defconst emlisp-font-lock-keywords
  '(("\\_<\\(define\\|if\\|print\\|\\|>\\)\\_>" . font-lock-keyword-face)
    ("\\_<\\(true\\|false\\|nil\\)\\_>" . font-lock-constant-face)
    ("\\_<:[[:word:]-]+\\_>" . font-lock-constant-face)
    ("\\_<[-+]?[0-9]+\\(?:\\.[0-9]+\\)?\\_>" . font-lock-constant-face)
    ("(\\s-*\\([+*/-]\\)\\_>" 1 font-lock-builtin-face))
  "Font-lock keywords for `emlisp-mode'.")

(defvar emlisp-mode-syntax-table
  (let ((table (make-syntax-table)))
    (modify-syntax-entry ?\; "<" table)
    (modify-syntax-entry ?\n ">" table)
    table)
  "Syntax table for `emlisp-mode'.")

;;;###autoload
(define-derived-mode emlisp-mode prog-mode "EmLisp"
  "Major mode for editing emlisp files."
  :syntax-table emlisp-mode-syntax-table
  (setq-local font-lock-defaults '(emlisp-font-lock-keywords))
  (setq-local comment-start ";")
  (setq-local comment-end "")
  (setq-local indent-line-function #'lisp-indent-line))

;;;###autoload
(add-to-list 'auto-mode-alist '("\\.emlisp\\'" . emlisp-mode))

(provide 'emlisp-mode)

;;; emlisp-mode.el ends here
