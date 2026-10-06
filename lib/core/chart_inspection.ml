open Core
module Wire = Gpuio_protocol.Chart_inspection_wire
module Placement = Wire.Placement
module Axis = Wire.Axis
module Pattern = Wire.Pattern

let resolve_color theme value =
  Option.value_map value ~default:(Ok None) ~f:(fun c ->
    Theme.resolve theme c |> Or_error.map ~f:Option.some)
;;

module Card = struct
  type t =
    { config : Wire.Card.t
    ; text_color : Color.t option
    ; background : Color.t option
    ; border_color : Color.t option
    }
  [@@deriving equal, sexp_of]

  let create
        ?(visible = true)
        ?(title = true)
        ?(values = true)
        ?(placement = Placement.Corner)
        ?(width = 280.)
        ?(gap = 8.)
        ?(padding = 8.)
        ?(radius = 6.)
        ?(font_size = 12.)
        ?(line_height = 17.)
        ?(border_width = 0.)
        ?text_color
        ?background
        ?border_color
        ()
    =
    let config =
      { Wire.Card.visible
      ; title
      ; values
      ; placement
      ; width
      ; gap
      ; padding
      ; radius
      ; font_size
      ; line_height
      ; border_width
      ; text_color = None
      ; background = None
      ; border_color = None
      }
    in
    if Wire.Card.valid config
    then Ok { config; text_color; background; border_color }
    else Or_error.error_string "invalid chart card dimensions"
  ;;

  let default = create () |> Or_error.ok_exn

  let resolve t theme =
    let open Or_error.Let_syntax in
    let%bind text_color = resolve_color theme t.text_color in
    let%bind background = resolve_color theme t.background in
    let%map border_color = resolve_color theme t.border_color in
    { t.config with text_color; background; border_color }
  ;;
end

module Crosshair = struct
  type t =
    { config : Wire.Crosshair.t
    ; color : Color.t option
    }
  [@@deriving equal, sexp_of]

  let create ?(axis = Axis.Off) ?(pattern = Pattern.Dashed) ?(thickness = 1.) ?color () =
    let config = { Wire.Crosshair.axis; pattern; thickness; color = None } in
    if Wire.Crosshair.valid config
    then Ok { config; color }
    else Or_error.error_string "invalid chart crosshair dimensions"
  ;;

  let default = create () |> Or_error.ok_exn

  let resolve t theme =
    let open Or_error.Let_syntax in
    let%map color = resolve_color theme t.color in
    { t.config with color }
  ;;
end

module Marker = struct
  type t =
    { config : Wire.Marker.t
    ; fill : Color.t option
    ; stroke : Color.t option
    }
  [@@deriving equal, sexp_of]

  let create
        ?(visible = true)
        ?(status = true)
        ?(size = 16.)
        ?(stroke_width = 0.)
        ?fill
        ?stroke
        ()
    =
    let config =
      { Wire.Marker.visible; status; size; stroke_width; fill = None; stroke = None }
    in
    if Wire.Marker.valid config
    then Ok { config; fill; stroke }
    else Or_error.error_string "invalid chart marker dimensions"
  ;;

  let default = create () |> Or_error.ok_exn

  let resolve t theme =
    let open Or_error.Let_syntax in
    let%bind fill = resolve_color theme t.fill in
    let%map stroke = resolve_color theme t.stroke in
    { t.config with fill; stroke }
  ;;
end

type t =
  { card : Card.t
  ; crosshair : Crosshair.t
  ; marker : Marker.t
  }
[@@deriving equal, sexp_of]

let create
      ?(card = Card.default)
      ?(crosshair = Crosshair.default)
      ?(marker = Marker.default)
      ()
  =
  { card; crosshair; marker }
;;

let default = create ()

module Expert = struct
  let to_wire t ~theme =
    let open Or_error.Let_syntax in
    let%bind card = Card.resolve t.card theme in
    let%bind crosshair = Crosshair.resolve t.crosshair theme in
    let%map marker = Marker.resolve t.marker theme in
    { Wire.card; crosshair; marker }
  ;;
end
