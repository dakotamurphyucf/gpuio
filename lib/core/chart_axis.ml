open Core
module Wire = Gpuio_protocol.Chart_axis_wire
module Label_side = Wire.Label_side
module Label_align = Wire.Label_align

let resolve_color theme value =
  Option.value_map value ~default:(Ok None) ~f:(fun c ->
    Theme.resolve theme c |> Or_error.map ~f:Option.some)
;;

module Tick_position = struct
  type t = Wire.Tick_position.t [@@deriving equal, sexp_of]

  let checked value =
    if Wire.Tick_position.valid value
    then Ok value
    else Or_error.error_string "invalid chart tick position"
  ;;

  let value n = checked (Value n)
  let category id = Wire.Tick_position.Category (Chart_data.Category_id.to_int64 id)
  let fraction n = checked (Fraction n)
end

module Tick = struct
  type t =
    { config : Wire.Tick.t
    ; color : Color.t option
    }
  [@@deriving equal, sexp_of]

  let create ~position ~text ?color ?font_size ?(align = Label_align.Auto) () =
    let config = { Wire.Tick.position; text; color = None; font_size; align } in
    if Wire.Tick.valid config
    then Ok { config; color }
    else Or_error.error_string "invalid chart tick text or font size"
  ;;

  let resolve t theme =
    let open Or_error.Let_syntax in
    let%map color = resolve_color theme t.color in
    { t.config with color }
  ;;
end

type t =
  { config : Wire.t
  ; ticks : Tick.t list option
  ; line_color : Color.t option
  ; label_color : Color.t option
  }
[@@deriving equal, sexp_of]

let create
      ?(line = true)
      ?(labels = true)
      ?position
      ?ticks
      ?tick_count
      ?(label_side = Label_side.Auto)
      ?(label_align = Label_align.Auto)
      ?label_gap
      ?label_width
      ?(font_size = 11.)
      ?(line_width = 1.)
      ?line_color
      ?label_color
      ()
  =
  let config =
    { Wire.line
    ; labels
    ; position
    ; ticks = Option.map ticks ~f:(List.map ~f:(fun t -> t.Tick.config))
    ; tick_count = Option.map tick_count ~f:Int64.of_int
    ; label_side
    ; label_align
    ; label_gap
    ; label_width
    ; font_size
    ; line_width
    ; line_color = None
    ; label_color = None
    }
  in
  if Wire.valid config
  then Ok { config; ticks; line_color; label_color }
  else Or_error.error_string "invalid chart axis dimensions or tick count"
;;

let default = create () |> Or_error.ok_exn

module Expert = struct
  let position_to_wire t = t

  let to_wire t ~theme =
    let open Or_error.Let_syntax in
    let%bind line_color = resolve_color theme t.line_color in
    let%bind label_color = resolve_color theme t.label_color in
    let%map ticks =
      Option.value_map t.ticks ~default:(Ok None) ~f:(fun ticks ->
        List.map ticks ~f:(fun tick -> Tick.resolve tick theme)
        |> Or_error.all
        |> Or_error.map ~f:Option.some)
    in
    { t.config with line_color; label_color; ticks }
  ;;
end
