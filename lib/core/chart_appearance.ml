open Core
module Wire = Gpuio_protocol.Chart_appearance_wire

let checked valid value message =
  if valid value then Ok value else Or_error.error_string message
;;

let resolve_optional value ~f =
  Option.value_map value ~default:(Ok None) ~f:(fun value ->
    Or_error.map (f value) ~f:Option.some)
;;

let resolve_brush brush theme =
  let open Or_error.Let_syntax in
  match Background.Expert.describe brush with
  | Solid color ->
    let%map color = Theme.resolve theme color in
    Wire.Brush.Solid color
  | Pattern_slash (color, width, interval) ->
    let%map color = Theme.resolve theme color in
    Wire.Brush.Pattern_slash (color, width, interval)
  | Checkerboard (color, size) ->
    let%map color = Theme.resolve theme color in
    Wire.Brush.Checkerboard (color, size)
  | Linear_gradient (space, angle, (from, start), (to_, stop)) ->
    let%bind from = Theme.resolve theme from in
    let%map to_ = Theme.resolve theme to_ in
    Wire.Brush.Linear
      { oklab = Background.Color_space.equal space Oklab; angle; from; start; to_; stop }
;;

module Corners = struct
  type t = Wire.Corners.t [@@deriving equal, sexp_of]

  let create
        ?(top_left = 0.)
        ?(top_right = 0.)
        ?(bottom_right = 0.)
        ?(bottom_left = 0.)
        ()
    =
    checked
      Wire.Corners.valid
      { Wire.Corners.top_left; top_right; bottom_right; bottom_left }
      "chart corners must be finite radii in [0,32]"
  ;;

  let all n = create ~top_left:n ~top_right:n ~bottom_right:n ~bottom_left:n ()
end

module Bar_fill = struct
  type t =
    | Background of Background.t
    | Base_to_tip of Color.t * Color.t
    | Domain of Color.t * Color.t
    | Values of float * Color.t * float * Color.t
  [@@deriving equal, sexp_of]

  let background b = Background b
  let base_to_tip ~from ~to_ = Base_to_tip (from, to_)
  let domain ~from ~to_ = Domain (from, to_)

  let values ~from:(lo, a) ~to_:(hi, b) =
    if Wire.Bar_fill.valid (Values (lo, 0L, hi, 0L))
    then Ok (Values (lo, a, hi, b))
    else Or_error.error_string "chart fill values must increase within [-1e100,1e100]"
  ;;

  let resolve t theme =
    let open Or_error.Let_syntax in
    match t with
    | Background b ->
      let%map b = resolve_brush b theme in
      Wire.Bar_fill.Background b
    | Base_to_tip (a, b) ->
      let%bind a = Theme.resolve theme a in
      let%map b = Theme.resolve theme b in
      Wire.Bar_fill.Base_to_tip (a, b)
    | Domain (a, b) ->
      let%bind a = Theme.resolve theme a in
      let%map b = Theme.resolve theme b in
      Wire.Bar_fill.Domain (a, b)
    | Values (lo, a, hi, b) ->
      let%bind a = Theme.resolve theme a in
      let%map b = Theme.resolve theme b in
      Wire.Bar_fill.Values (lo, a, hi, b)
  ;;
end

module Stroke = struct
  type t =
    { visible : bool
    ; width : float option
    ; brush : Background.t
    }
  [@@deriving equal, sexp_of]

  let create ?(visible = true) ?width brush =
    if Option.for_all width ~f:(fun n -> Wire.within n 0.5 8.)
    then Ok { visible; width; brush }
    else Or_error.error_string "chart stroke width must be finite in [0.5,8]"
  ;;

  let resolve t theme =
    let%map.Or_error brush = resolve_brush t.brush theme in
    { Wire.Stroke.visible = t.visible; width = t.width; brush }
  ;;
end

module Path = struct
  type t =
    { stroke : Stroke.t option
    ; fill : Background.t option
    ; curve : Chart_options.Curve.t option
    }
  [@@deriving equal, sexp_of]

  let create ?stroke ?fill ?curve () = { stroke; fill; curve }

  let resolve t theme =
    let open Or_error.Let_syntax in
    let%bind stroke = resolve_optional t.stroke ~f:(fun s -> Stroke.resolve s theme) in
    let%map fill = resolve_optional t.fill ~f:(fun b -> resolve_brush b theme) in
    let curve =
      Option.map t.curve ~f:(function
        | Chart_options.Curve.Linear -> Gpuio_protocol.Chart_options_wire.Curve.Linear
        | Natural -> Natural
        | Step_after -> Step_after)
    in
    { Wire.Path.stroke; fill; curve }
  ;;
end

module Marker = struct
  type t =
    { config : Wire.Marker.t
    ; fill : Color.t option
    ; stroke : Color.t option
    }
  [@@deriving equal, sexp_of]

  let create ?visible ?radius ?fill ?stroke ?stroke_width () =
    let config =
      { Wire.Marker.visible; radius; fill = None; stroke = None; stroke_width }
    in
    if Wire.Marker.valid config
    then Ok { config; fill; stroke }
    else Or_error.error_string "chart marker radius must be in [1,24], border in [0,8]"
  ;;

  let resolve t theme =
    let open Or_error.Let_syntax in
    let%bind fill = resolve_optional t.fill ~f:(Theme.resolve theme) in
    let%map stroke = resolve_optional t.stroke ~f:(Theme.resolve theme) in
    { t.config with fill; stroke }
  ;;
end

module Bar = struct
  type t =
    { fill : Bar_fill.t option
    ; corners : Corners.t option
    }
  [@@deriving equal, sexp_of]

  let create ?fill ?corners () = { fill; corners }

  let resolve t theme =
    let%map.Or_error fill =
      resolve_optional t.fill ~f:(fun f -> Bar_fill.resolve f theme)
    in
    { Wire.Bar.fill; corners = t.corners }
  ;;
end

module Baseline = struct
  type t = float [@@deriving equal, sexp_of]

  let create value =
    if Wire.within value (-1e100) 1e100
    then Ok value
    else Or_error.error_string "area baseline must be finite within [-1e100,1e100]"
  ;;
end

module Series = struct
  type t =
    { series : Chart_data.Series_id.t
    ; path : Path.t option
    ; marker : Marker.t option
    ; bar : Bar.t option
    ; legend : Color.t option
    ; area_baseline : Baseline.t option
    }
  [@@deriving equal, sexp_of]

  let create ~series ?path ?marker ?bar ?legend ?area_baseline () =
    { series; path; marker; bar; legend; area_baseline }
  ;;

  let resolve t theme =
    let open Or_error.Let_syntax in
    let%bind path = resolve_optional t.path ~f:(fun p -> Path.resolve p theme) in
    let%bind marker = resolve_optional t.marker ~f:(fun m -> Marker.resolve m theme) in
    let%bind bar = resolve_optional t.bar ~f:(fun b -> Bar.resolve b theme) in
    let%map legend = resolve_optional t.legend ~f:(Theme.resolve theme) in
    { Wire.Series.series = Chart_data.Series_id.to_int64 t.series
    ; path
    ; marker
    ; bar
    ; legend
    ; area_baseline = t.area_baseline
    }
  ;;
end

module Datum = struct
  type t =
    { series : Chart_data.Series_id.t
    ; datum : Chart_data.Datum_id.t
    ; marker : Marker.t option
    ; bar : Bar.t option
    }
  [@@deriving equal, sexp_of]

  let create ~series ~datum ?marker ?bar () = { series; datum; marker; bar }

  let resolve t theme =
    let open Or_error.Let_syntax in
    let%bind marker = resolve_optional t.marker ~f:(fun m -> Marker.resolve m theme) in
    let%map bar = resolve_optional t.bar ~f:(fun b -> Bar.resolve b theme) in
    { Wire.Datum.series = Chart_data.Series_id.to_int64 t.series
    ; datum = Chart_data.Datum_id.to_int64 t.datum
    ; marker
    ; bar
    }
  ;;
end

module Aggregates = Wire.Aggregates

type t =
  { series : Series.t list
  ; data : Datum.t list
  ; aggregates : Aggregates.t
  }
[@@deriving equal, sexp_of]

let create ?(series = []) ?(data = []) ?(aggregates = Aggregates.Inherit_series) () =
  if List.length series > 128 || List.length data > 1024
  then
    Or_error.error_string
      "chart appearance allows at most 128 series and 1024 datum pairs"
  else if
    List.contains_dup
      (List.map series ~f:(fun s -> s.Series.series))
      ~compare:Chart_data.Series_id.compare
  then Or_error.error_string "chart appearance series IDs must be unique"
  else if
    List.contains_dup
      (List.map data ~f:(fun d -> d.Datum.series, d.datum))
      ~compare:[%compare: Chart_data.Series_id.t * Chart_data.Datum_id.t]
  then Or_error.error_string "chart appearance datum pairs must be unique"
  else Ok { series; data; aggregates }
;;

let empty = create () |> Or_error.ok_exn

module Expert = struct
  let to_wire t ~theme =
    let open Or_error.Let_syntax in
    let%bind series =
      List.map t.series ~f:(fun s -> Series.resolve s theme) |> Or_error.all
    in
    let%map data = List.map t.data ~f:(fun d -> Datum.resolve d theme) |> Or_error.all in
    { Wire.series; data; aggregates = t.aggregates }
  ;;
end
