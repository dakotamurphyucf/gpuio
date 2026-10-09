open Core
module Wire = Gpuio_protocol.Chart_pie_labels_wire

module Entry = struct
  type t =
    { slice : Chart_data.Datum_id.t
    ; text : string option
    ; line_color : Color.t option
    }
  [@@deriving equal, sexp_of]

  let create ~slice ?text ?line_color () =
    if Option.for_all text ~f:Wire.Entry.valid_text
    then Ok { slice; text; line_color }
    else
      Or_error.error_string
        "pie caption requires at most 256 UTF-8 bytes without ASCII controls"
  ;;
end

type t = Entry.t list [@@deriving equal, sexp_of]

let create entries =
  if List.length entries > 256
  then Or_error.error_string "pie labels require at most 256 entries"
  else if
    List.contains_dup
      (List.map entries ~f:(fun e -> e.Entry.slice))
      ~compare:Chart_data.Datum_id.compare
  then Or_error.error_string "pie label slice IDs must be unique"
  else if
    List.sum
      (module Int)
      entries
      ~f:(fun e -> Option.value_map e.Entry.text ~default:0 ~f:String.length)
    > 32768
  then Or_error.error_string "pie caption overrides exceed 32 KiB"
  else Ok entries
;;

let empty = []

module Expert = struct
  let to_wire t ~theme =
    List.map t ~f:(fun e ->
      let open Or_error.Let_syntax in
      let%map line_color =
        Option.value_map e.Entry.line_color ~default:(Ok None) ~f:(fun c ->
          Theme.resolve theme c |> Or_error.map ~f:Option.some)
      in
      { Wire.Entry.slice = Chart_data.Datum_id.to_int64 e.slice
      ; text = e.text
      ; line_color
      })
    |> Or_error.all
  ;;
end
