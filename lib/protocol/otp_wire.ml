open Core

let max_input_bytes = 4096

module Alphabet = struct
  type t =
    | Digits
    | Ascii_alphanumeric
  [@@deriving bin_io, equal, sexp_of]
end

module Policy = struct
  type t =
    { length : int
    ; alphabet : Alphabet.t
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t = t.length >= 1 && t.length <= 32
end

module Input_error = struct
  type t =
    | Invalid_policy
    | Input_too_large
    | Invalid_utf8
    | Unexpected_character of { byte_offset : int }
    | Too_long
    | Invalid_value
    | Invalid_selection
  [@@deriving equal, sexp_of]
end

let allowed alphabet code =
  (code >= 48 && code <= 57)
  ||
  match alphabet with
  | Alphabet.Digits -> false
  | Ascii_alphanumeric -> (code >= 65 && code <= 90) || (code >= 97 && code <= 122)
;;

let canonical policy text =
  Policy.valid policy
  && String.length text <= policy.length
  && String.for_all text ~f:(fun ch -> allowed policy.alphabet (Char.to_int ch))
;;

let full_width code =
  if
    (code >= 0xff10 && code <= 0xff19)
    || (code >= 0xff21 && code <= 0xff3a)
    || (code >= 0xff41 && code <= 0xff5a)
  then code - 0xfee0
  else code
;;

let separator = function
  | 9 | 10 | 11 | 12 | 13 | 32 | 45 -> true
  | _ -> false
;;

let normalize policy ~paste text =
  let open Input_error in
  if not (Policy.valid policy)
  then Error Invalid_policy
  else if String.length text > max_input_bytes
  then Error Input_too_large
  else if not (Stdlib.String.is_valid_utf_8 text)
  then Error Invalid_utf8
  else (
    let buffer = Buffer.create policy.length in
    let rec loop offset =
      if offset = String.length text
      then Ok (Buffer.contents buffer)
      else (
        let decoded = Stdlib.String.get_utf_8_uchar text offset in
        let code = Stdlib.Uchar.to_int (Stdlib.Uchar.utf_decode_uchar decoded) in
        let next = offset + Stdlib.Uchar.utf_decode_length decoded in
        if paste && separator code
        then loop next
        else if not (allowed policy.alphabet (full_width code))
        then Error (Unexpected_character { byte_offset = offset })
        else if Buffer.length buffer = policy.length
        then Error Too_long
        else (
          Buffer.add_char buffer (Char.of_int_exn (full_width code));
          loop next))
    in
    loop 0)
;;

let replace policy ~value ~anchor ~head ~paste text =
  let open Input_error in
  if not (Policy.valid policy)
  then Error Invalid_policy
  else if not (canonical policy value)
  then Error Invalid_value
  else if
    anchor < 0 || head < 0 || anchor > String.length value || head > String.length value
  then Error Invalid_selection
  else
    let open Result.Let_syntax in
    let%bind inserted = normalize policy ~paste text in
    let start = Int.min anchor head
    and stop = Int.max anchor head in
    if paste && String.is_empty inserted
    then Ok (value, anchor, head)
    else if String.length value - (stop - start) + String.length inserted > policy.length
    then Error Too_long
    else
      Ok
        ( String.prefix value start ^ inserted ^ String.drop_prefix value stop
        , start + String.length inserted
        , start + String.length inserted )
;;
