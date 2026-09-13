open Core

type t =
  | Unchecked
  | Checked
  | Indeterminate
[@@deriving equal, sexp_of]

let activate = function
  | Unchecked | Indeterminate -> Checked
  | Checked -> Unchecked
;;

let of_bool value = if value then Checked else Unchecked
