open Core
module Wire = Gpuio_protocol.Chart_style_wire

module Key = struct
  type t = Wire.Key.t [@@deriving equal, compare, sexp_of]

  let series id = Wire.Key.Series (Chart_data.Series_id.to_int64 id)
  let slice id = Wire.Key.Slice (Chart_data.Datum_id.to_int64 id)
  let node id = Wire.Key.Node (Chart_data.Node_id.to_int64 id)
  let rising = Wire.Key.Rising
  let falling = Wire.Key.Falling
end

module Ordinal = struct
  type t =
    { domain : Key.t list
    ; range : Color.t list
    ; unknown : Color.t option
    }
  [@@deriving equal, sexp_of]

  let create ~domain ~range ?unknown () =
    if List.length domain > 1024 || List.contains_dup domain ~compare:Key.compare
    then Or_error.error_string "ordinal color domain requires at most 1024 unique keys"
    else if List.is_empty range || List.length range > 32
    then Or_error.error_string "ordinal color range requires 1..32 colors"
    else Ok { domain; range; unknown }
  ;;

  let find t key =
    match List.findi t.domain ~f:(fun _ candidate -> Key.equal candidate key) with
    | None -> t.unknown
    | Some (index, _) -> List.nth t.range (index % List.length t.range)
  ;;

  let resolve t theme =
    let open Or_error.Let_syntax in
    let%bind range = List.map t.range ~f:(Theme.resolve theme) |> Or_error.all in
    let%map unknown =
      Option.value_map t.unknown ~default:(Ok None) ~f:(fun color ->
        Theme.resolve theme color |> Or_error.map ~f:Option.some)
    in
    { Wire.Ordinal.domain = t.domain; range; unknown }
  ;;
end

type t = Wire.t [@@deriving equal, sexp_of]

let create
      ?(palette =
        List.map
          [ 0x818cf8; 0x2dd4bf; 0xfbbf24; 0xf472b6; 0x38bdf8; 0xfb923c ]
          ~f:Color.rgb_exn)
      ?ordinal
      ?(inspection = Chart_inspection.default)
      ?(node_labels = Chart_node_labels.empty)
      ?(axis_color = Color.rgb_exn 0x64748b)
      ?(grid_color =
        Color.rgba ~red:100 ~green:116 ~blue:139 ~alpha:64 |> Or_error.ok_exn)
      ?(label_color = Color.rgb_exn 0x94a3b8)
      ?(selection_color = Color.rgb_exn 0xf8fafc)
      ?gradient_end
      ?(stroke_width = 2.)
      ?(point_radius = 3.5)
      ?(bar_radius = 3.)
      ?(area_opacity = 0.2)
      ?(theme = Theme.default)
      ()
  =
  let open Or_error.Let_syntax in
  if List.is_empty palette || List.length palette > 32
  then Or_error.error_string "chart palette requires 1..32 colors"
  else (
    let%bind palette = List.map palette ~f:(Theme.resolve theme) |> Or_error.all in
    let%bind axis_color = Theme.resolve theme axis_color in
    let%bind grid_color = Theme.resolve theme grid_color in
    let%bind label_color = Theme.resolve theme label_color in
    let%bind selection_color = Theme.resolve theme selection_color in
    let%bind gradient_end =
      Option.value_map gradient_end ~default:(Ok None) ~f:(fun color ->
        Theme.resolve theme color |> Or_error.map ~f:Option.some)
    in
    let%bind ordinal =
      Option.value_map ordinal ~default:(Ok None) ~f:(fun value ->
        Ordinal.resolve value theme |> Or_error.map ~f:Option.some)
    in
    let%bind inspection = Chart_inspection.Expert.to_wire inspection ~theme in
    let%bind node_labels = Chart_node_labels.Expert.to_wire node_labels ~theme in
    let t =
      { Wire.version = -2L
      ; palette
      ; axis_color
      ; grid_color
      ; label_color
      ; selection_color
      ; gradient_end
      ; stroke_width
      ; point_radius
      ; bar_radius
      ; area_opacity
      ; ordinal
      ; inspection
      ; node_labels
      }
    in
    if Wire.valid t
    then Ok t
    else Or_error.error_string "invalid chart stroke, point, corner radius or opacity")
;;

let default = create () |> Or_error.ok_exn

module Expert = struct
  let to_wire t = t

  let of_wire t =
    if Wire.valid t then Ok t else Or_error.error_string "invalid chart style"
  ;;
end
