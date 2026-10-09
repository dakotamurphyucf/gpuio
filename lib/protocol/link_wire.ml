open Core

let max_label_bytes = 4096
let max_tab_index = 1000000L

type t =
  { label : string
  ; disabled : bool
  ; loading : bool
  ; tab_stop : bool
  ; tab_index : int64
  }
[@@deriving bin_io, equal, sexp_of]

let valid t =
  String.length t.label <= max_label_bytes
  && Stdlib.String.is_valid_utf_8 t.label
  && (not (String.contains t.label '\000'))
  && String.exists t.label ~f:(fun ch -> not (Char.is_whitespace ch))
  && Int64.(t.tab_index >= neg max_tab_index && t.tab_index <= max_tab_index)
;;
