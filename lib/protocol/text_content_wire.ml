open Core

let max_text_bytes = 262144
let max_spans = 4096

module Span = struct
  type t =
    { start_byte : int64
    ; end_byte : int64
    ; foreground : int64
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Int64.(
      t.start_byte >= 0L
      && t.end_byte > t.start_byte
      && t.end_byte <= of_int max_text_bytes
      && t.foreground >= 0L
      && t.foreground <= 0xffff_ffffL)
  ;;
end

type t =
  { text : string
  ; spans : Span.t list
  }
[@@deriving bin_io, equal, sexp_of]

let boundary text offset =
  offset = String.length text || Char.to_int text.[offset] land 0xc0 <> 0x80
;;

let valid t =
  String.length t.text <= max_text_bytes
  && Stdlib.String.is_valid_utf_8 t.text
  && List.length t.spans <= max_spans
  &&
  let rec loop previous_end = function
    | [] -> true
    | span :: rest ->
      Span.valid span
      && Int64.(
           span.start_byte >= previous_end
           && span.end_byte <= of_int (String.length t.text))
      && boundary t.text (Int64.to_int_exn span.start_byte)
      && boundary t.text (Int64.to_int_exn span.end_byte)
      && loop span.end_byte rest
  in
  loop 0L t.spans
;;
