open Core

let max_query_bytes = 2048
let max_text_bytes = 262_144

let valid_query query =
  String.length query <= max_query_bytes
  && Stdlib.String.is_valid_utf_8 query
  && not (String.contains query '\000')
;;

module Case = struct
  type t =
    | Sensitive
    | Ascii_insensitive
  [@@deriving bin_io, equal, sexp_of]
end

module Mode = struct
  type t =
    | Closed
    | Find
    | Replace
  [@@deriving bin_io, equal, sexp_of]
end

module Stamp = struct
  type t =
    { editor_revision : int64
    ; search_revision : int64
    }
  [@@deriving bin_io, equal, sexp_of]

  let is_valid t = Int64.(t.editor_revision >= 0L && t.search_revision >= 0L)
end

module Occurrence = struct
  type t =
    { index : int64
    ; byte_start : int64
    ; byte_end : int64
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Snapshot = struct
  type t =
    { stamp : Stamp.t
    ; activation_revision : int64
    ; mode : Mode.t
    ; query : string
    ; case : Case.t
    ; text_bytes : int64
    ; match_count : int64
    ; current : Occurrence.t option
    ; can_replace : bool
    }
  [@@deriving bin_io, equal, sexp_of]

  let is_valid t =
    Stamp.is_valid t.stamp
    && valid_query t.query
    && Int64.(
         t.activation_revision >= 0L && t.activation_revision <= t.stamp.search_revision)
    && Int64.(t.text_bytes >= 0L && t.text_bytes <= of_int max_text_bytes)
    && Int64.(t.match_count >= 0L && t.match_count <= t.text_bytes)
    && ((not t.can_replace) || not (Mode.equal t.mode Closed))
    && (if String.is_empty t.query
        then Int64.equal t.match_count 0L
        else Int64.(t.match_count * of_int (String.length t.query) <= t.text_bytes))
    &&
    match t.current with
    | None -> Int64.equal t.match_count 0L
    | Some { index; byte_start; byte_end } ->
      Int64.(
        index >= 0L
        && index < t.match_count
        && byte_start >= 0L
        && byte_end > byte_start
        && byte_end <= t.text_bytes
        && byte_end - byte_start = of_int (String.length t.query))
  ;;
end

module Command = struct
  type t =
    | Read
    | Open of bool
    | Close
    | Set_query of string * Case.t
    | Next
    | Previous
    | Replace_current of Stamp.t * string
    | Replace_all of Stamp.t * string
    | Close_and_focus of int64
    | Set_query_text of string
    | Set_case of Case.t
    | Toggle_case
  [@@deriving bin_io, equal, sexp_of]
end
