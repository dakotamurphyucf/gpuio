open Core

type t = string [@@deriving equal, compare, sexp_of]

let max_bytes = 16_384

let of_string path =
  if String.length path > max_bytes
  then Or_error.error_string "native path exceeds 16384 bytes"
  else if String.is_empty path || not (Char.equal path.[0] '/')
  then Or_error.error_string "native path must be absolute"
  else if String.contains path '\000'
  then Or_error.error_string "native path contains NUL"
  else Ok path
;;

let to_string t = t
