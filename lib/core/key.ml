open Core
type t = string [@@deriving compare, equal, sexp_of]
let of_string value =
  if String.is_empty value || String.length value > 256 then Or_error.error_string "key must contain 1..256 bytes"
  else Ok value
;;
let of_string_exn value = of_string value |> Or_error.ok_exn
let of_int value = Int.to_string value
let to_string t = t
