open Core
open Gpuio
module A = Chart_appearance
module D = Chart_data

type t =
  | Default
  | Paths
  | Markers
  | Signed_bars
  | Domain_bars
  | Value_bars
  | Uniform_buckets
  | Slash_pattern
  | Checkerboard
  | Raised_area
[@@deriving equal]

let all =
  [ Default
  ; Paths
  ; Markers
  ; Signed_bars
  ; Domain_bars
  ; Value_bars
  ; Uniform_buckets
  ; Slash_pattern
  ; Checkerboard
  ; Raised_area
  ]
;;

let label = function
  | Default -> "Default marks"
  | Paths -> "Styled paths"
  | Markers -> "Styled markers"
  | Signed_bars -> "Base-to-tip bars"
  | Domain_bars -> "Domain-colored bars"
  | Value_bars -> "Value-colored bars"
  | Uniform_buckets -> "Uniform aggregate colors"
  | Slash_pattern -> "Slash pattern"
  | Checkerboard -> "Checkerboard pattern"
  | Raised_area -> "Area baseline 20"
;;

let series data =
  match D.Expert.contents data with
  | Cartesian layers ->
    List.map layers ~f:(fun (Line s | Area s | Bar s) ->
      D.Series.id s, List.map (List.take (D.Series.points s) 32) ~f:D.Point.id)
  | Categorical (_, layers) ->
    List.map layers ~f:(fun (Line s | Area s | Bar s) ->
      ( D.Categorical_series.id s
      , List.map (List.take (D.Categorical_series.points s) 32) ~f:D.Categorical_point.id
      ))
  | Radar (_, series) ->
    List.map series ~f:(fun s ->
      D.Radar_series.id s, List.map (List.take (D.Radar_series.values s) 32) ~f:fst)
  | Pie _ | Candlestick _ | Sankey _ -> []
;;

let configuration t palette data =
  let ok = Or_error.ok_exn in
  let accent = Palette.accent palette in
  let muted = Palette.muted palette in
  let foreground = Palette.foreground palette in
  let gold = Color.rgb_exn 0xfbbf24 in
  let corners = A.Corners.create ~top_left:16. ~bottom_right:8. () |> ok in
  let pattern =
    match t with
    | Slash_pattern -> Some (Background.pattern_slash accent ~width:2. ~interval:4. |> ok)
    | Checkerboard -> Some (Background.checkerboard accent ~size:8. |> ok)
    | Default
    | Paths
    | Markers
    | Signed_bars
    | Domain_bars
    | Value_bars
    | Uniform_buckets
    | Raised_area -> None
  in
  let path =
    A.Path.create
      ~stroke:(A.Stroke.create ~width:4. (Background.solid accent) |> ok)
      ~fill:
        (Option.value
           pattern
           ~default:
             (Background.linear_gradient
                ~angle:180.
                ~from:(Color.with_opacity accent 0.65 |> ok, 0.)
                ~to_:(Color.with_opacity accent 0.08 |> ok, 1.)
              |> ok))
      ~curve:Natural
      ()
  in
  let marker =
    A.Marker.create ~radius:8. ~fill:accent ~stroke:foreground ~stroke_width:3. () |> ok
  in
  let bar_fill =
    match t with
    | Signed_bars -> A.Bar_fill.base_to_tip ~from:muted ~to_:accent
    | Domain_bars -> A.Bar_fill.domain ~from:accent ~to_:gold
    | Value_bars -> A.Bar_fill.values ~from:(0., accent) ~to_:(20., gold) |> ok
    | Slash_pattern | Checkerboard -> A.Bar_fill.background (Option.value_exn pattern)
    | Default | Paths | Markers | Uniform_buckets | Raised_area ->
      A.Bar_fill.background (Background.solid muted)
  in
  if equal t Default
  then A.empty
  else (
    let source = series data in
    let series =
      List.map source ~f:(fun (series, _) ->
        A.Series.create
          ~series
          ?path:(Option.some_if (equal t Paths || Option.is_some pattern) path)
          ?marker:(Option.some_if (equal t Markers) marker)
          ?bar:
            (Option.some_if
               (List.mem
                  [ Signed_bars
                  ; Domain_bars
                  ; Value_bars
                  ; Uniform_buckets
                  ; Slash_pattern
                  ; Checkerboard
                  ]
                  t
                  ~equal)
               (A.Bar.create ~fill:bar_fill ~corners ()))
          ?area_baseline:
            (Option.some_if (equal t Raised_area) (A.Baseline.create 20. |> ok))
          ?legend:(Option.some_if (not (equal t Raised_area)) accent)
          ())
    in
    let data =
      List.concat_map source ~f:(fun (series, points) ->
        match t with
        | Markers ->
          List.mapi (List.take points 2) ~f:(fun i datum ->
            A.Datum.create
              ~series
              ~datum
              ~marker:
                (A.Marker.create ~radius:(if i = 0 then 12. else 18.) ~fill:gold () |> ok)
              ())
        | Uniform_buckets ->
          List.map points ~f:(fun datum ->
            A.Datum.create
              ~series
              ~datum
              ~bar:
                (A.Bar.create ~fill:(A.Bar_fill.background (Background.solid accent)) ())
              ())
        | Default
        | Paths
        | Signed_bars
        | Domain_bars
        | Value_bars
        | Slash_pattern
        | Checkerboard
        | Raised_area -> [])
    in
    A.create ~series ~data ~aggregates:Uniform () |> ok)
;;

let dots t = equal t Markers

let sampling = function
  | Uniform_buckets ->
    Chart_sampling.create
      ~bars:(Chart_sampling.Bar.sum ~max_buckets:2 |> Or_error.ok_exn)
      ()
  | Default
  | Paths
  | Markers
  | Signed_bars
  | Domain_bars
  | Value_bars
  | Slash_pattern
  | Checkerboard
  | Raised_area -> Chart_sampling.default
;;
