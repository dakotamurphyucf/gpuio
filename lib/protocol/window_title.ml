open! Core

type t = string [@@deriving equal, sexp_of]

let of_string text =
  if String.contains text '\000'
  then Or_error.error_string "Window title must not contain NUL bytes"
  else Ok text
;;

let to_string t = t
