open Core

type t =
  { sequence : int64
  ; query_revision : int64
  ; query : string
  ; composing : bool
  ; selected : string option
  ; matched_count : int
  }
[@@deriving bin_io, equal, sexp_of]

let valid t =
  Int64.(t.sequence > 0L && t.query_revision > 0L && t.query_revision <= t.sequence)
  && String.length t.query <= 4096
  && Stdlib.String.is_valid_utf_8 t.query
  && (not
        (String.exists t.query ~f:(function
           | '\000' | '\r' | '\n' -> true
           | _ -> false)))
  && t.matched_count >= 0
  && t.matched_count <= 1024
  && Option.for_all t.selected ~f:(fun id ->
    t.matched_count > 0
    && String.length id <= 256
    && (not (String.is_empty (String.strip id)))
    && Stdlib.String.is_valid_utf_8 id
    && not (String.contains id '\000'))
;;
