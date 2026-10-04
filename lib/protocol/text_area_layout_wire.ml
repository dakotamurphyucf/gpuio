open Core

module Wrapping_indent = struct
  type t =
    | Flush_left
    | Match_first_line
  [@@deriving bin_io, equal, sexp_of]
end

type t =
  { soft_wrap : bool
  ; wrapping_indent : Wrapping_indent.t
  ; show_whitespace : bool
  ; cursor_margin_lines : int64 option
  }
[@@deriving bin_io, equal, sexp_of]

let is_valid t =
  Option.for_all t.cursor_margin_lines ~f:(fun n -> Int64.(n >= 0L && n <= 256L))
;;
