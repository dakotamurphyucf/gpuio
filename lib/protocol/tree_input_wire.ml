open Core

let valid_typeahead_text text =
  let rec printable offset =
    if offset = String.length text
    then true
    else (
      let decoded = Stdlib.String.get_utf_8_uchar text offset in
      let code = Stdlib.Uchar.to_int (Stdlib.Uchar.utf_decode_uchar decoded) in
      code >= 32
      && (not (code >= 127 && code <= 159))
      && printable (offset + Stdlib.Uchar.utf_decode_length decoded))
  in
  (not (String.is_empty text))
  && String.length text <= 256
  && Stdlib.String.is_valid_utf_8 text
  && printable 0
;;

module Navigation = struct
  type t =
    | Previous
    | Next
    | First
    | Last
    | Parent
    | Child
  [@@deriving bin_io, equal, sexp_of]
end

module Selection = struct
  type t =
    | Replace
    | Toggle
    | Range of { extend : bool }
  [@@deriving bin_io, equal, sexp_of]
end

module Request = struct
  type t =
    | Navigate of Navigation.t * Selection.t option
    | Select of int64 * Selection.t
    | Focus of int64
    | Set_expanded of int64 * bool
    | Activate of int64
    | Select_active of Selection.t
    | Activate_active
    | Typeahead of
        { text : string
        ; reset : bool
        ; cycle : bool
        }
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Navigate _ | Select_active _ | Activate_active -> true
    | Typeahead { text; _ } -> valid_typeahead_text text
    | Select (id, _) | Focus id | Set_expanded (id, _) | Activate id -> Int64.(id > 0L)
  ;;
end
