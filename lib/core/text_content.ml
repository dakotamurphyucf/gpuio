open Core
module Wire = Gpuio_protocol.Text_content_wire

module Span = struct
  type t =
    { start_byte : int
    ; end_byte : int
    ; foreground : Color.t
    }
  [@@deriving equal, sexp_of]

  let create ~start_byte ~end_byte ~foreground =
    if start_byte < 0 || end_byte <= start_byte || end_byte > Wire.max_text_bytes
    then Or_error.error_string "text span requires 0 <= start_byte < end_byte <= 262144"
    else Ok { start_byte; end_byte; foreground }
  ;;

  let start_byte t = t.start_byte
  let end_byte t = t.end_byte
  let foreground t = t.foreground

  let wire t foreground =
    { Wire.Span.start_byte = Int64.of_int t.start_byte
    ; end_byte = Int64.of_int t.end_byte
    ; foreground
    }
  ;;
end

type t =
  { text : string
  ; spans : Span.t list
  }
[@@deriving equal, sexp_of]

let create ?(spans = []) text =
  if String.length text > Wire.max_text_bytes || List.length spans > Wire.max_spans
  then Or_error.error_string "text content exceeds 262144 bytes or 4096 foreground spans"
  else (
    let wire = { Wire.text; spans = List.map spans ~f:(fun span -> Span.wire span 0L) } in
    if Wire.valid wire
    then Ok { text; spans }
    else
      Or_error.error_string
        "text requires bounded UTF-8 with sorted disjoint scalar-boundary foreground \
         spans")
;;

let text t = t.text
let spans t = t.spans

module Expert = struct
  let to_wire t ~theme =
    let open Or_error.Let_syntax in
    let%map spans =
      List.map t.spans ~f:(fun span ->
        let%map foreground = Theme.resolve theme (Span.foreground span) in
        Span.wire span foreground)
      |> Or_error.all
    in
    { Wire.text = t.text; spans }
  ;;

  let of_wire (wire : Wire.t) =
    if not (Wire.valid wire)
    then Or_error.error_string "invalid text content wire value"
    else (
      let spans =
        List.map wire.spans ~f:(fun span ->
          let channel shift =
            Int64.(bit_and (shift_right_logical span.foreground shift) 0xffL)
            |> Int64.to_int_exn
          in
          let foreground =
            Color.rgba
              ~red:(channel 24)
              ~green:(channel 16)
              ~blue:(channel 8)
              ~alpha:(channel 0)
            |> Or_error.ok_exn
          in
          { Span.start_byte = Int64.to_int_exn span.start_byte
          ; end_byte = Int64.to_int_exn span.end_byte
          ; foreground
          })
      in
      Ok { text = wire.text; spans })
  ;;
end
