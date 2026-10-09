module Ordinal_colors = Ordinal_colors
module Stacked = Stacked
open Core
module D = Gpuio.Chart_data
module Categorical = Categorical

let ok = Or_error.ok_exn
let datum n = D.Datum_id.of_int64 (Int64.of_int n) |> ok
let series_id n = D.Series_id.of_int64 (Int64.of_int n) |> ok
let node_id n = D.Node_id.of_int64 (Int64.of_int n) |> ok
let edge_id n = D.Edge_id.of_int64 (Int64.of_int n) |> ok

module Family = struct
  type t =
    | Line
    | Area
    | Bar
    | Pie
    | Radar
    | Candlestick
    | Sankey
  [@@deriving equal, sexp_of]

  let all = [ Line; Area; Bar; Pie; Radar; Candlestick; Sankey ]

  let of_index index =
    List.nth all index |> Result.of_option ~error:(Error.of_string "Unknown chart family")
  ;;

  let label = function
    | Line -> "Line"
    | Area -> "Area"
    | Bar -> "Bar"
    | Pie -> "Pie"
    | Radar -> "Radar"
    | Candlestick -> "Candlestick"
    | Sankey -> "Sankey"
  ;;

  let description = function
    | Line -> "Response quality over time"
    | Area -> "Capacity throughout the day"
    | Bar -> "Throughput by evaluation run"
    | Pie -> "Where the work happens"
    | Radar -> "A balanced model scorecard"
    | Candlestick -> "The shape of market movement"
    | Sankey -> "From request to resolution"
  ;;
end

let series ~phase ~id ~name =
  D.Series.create
    ~id:(series_id id)
    ~name
    (List.init 24 ~f:(fun i ->
       let x = Float.of_int i in
       D.Point.create
         ~id:(datum (i + 1))
         ~x
         ~y:(Some (30. +. (12. *. Float.sin ((x +. phase) /. 4.)) +. (x *. 1.4)))
         ()
       |> ok))
  |> ok
;;

let validate_phase phase =
  if (not (Float.is_finite phase)) || Float.(phase < 0.)
  then invalid_arg "Chart sample phase must be finite and nonnegative"
;;

let data_exn family phase =
  validate_phase phase;
  match family with
  | Family.Line ->
    D.line
      [ series ~phase ~id:1 ~name:"Atlas"
      ; series ~phase:(phase +. 8.) ~id:2 ~name:"Nova"
      ]
    |> ok
  | Family.Area -> D.area [ series ~phase ~id:1 ~name:"Active capacity" ] |> ok
  | Family.Bar -> D.bar [ series ~phase ~id:1 ~name:"Completed evaluations" ] |> ok
  | Family.Pie ->
    D.pie
      (List.mapi
         [ "Reasoning", 44.; "Code", 28.; "Research", 18.; "Other", 10. ]
         ~f:(fun i (label, value) ->
           D.Slice.create ~id:(datum (i + 1)) ~label ~value |> ok))
    |> ok
  | Family.Radar ->
    let axes =
      List.mapi
        [ "Quality"; "Speed"; "Cost"; "Context"; "Reliability" ]
        ~f:(fun i label ->
          D.Radar_axis.create ~id:(datum (i + 1)) ~label ~maximum:100. |> ok)
    in
    D.radar
      ~axes
      [ D.Radar_series.create
          ~id:(series_id 1)
          ~name:"Atlas"
          (List.mapi [ 88.; 72.; 65.; 94.; 82. ] ~f:(fun i v -> datum (i + 1), v))
        |> ok
      ; D.Radar_series.create
          ~id:(series_id 2)
          ~name:"Nova"
          (List.mapi [ 80.; 91.; 84.; 72.; 86. ] ~f:(fun i v -> datum (i + 1), v))
        |> ok
      ]
    |> ok
  | Family.Candlestick ->
    D.candlestick
      (List.init 24 ~f:(fun i ->
         let x = Float.of_int i in
         let open_ = 30. +. (10. *. Float.sin (x /. 5.)) in
         let close = open_ +. (4. *. Float.cos (x +. phase)) in
         D.Candle.create
           ~id:(datum (i + 1))
           ~x
           ~label:(sprintf "Session %d" (i + 1))
           ~open_
           ~close
           ~high:(Float.max open_ close +. 3.)
           ~low:(Float.min open_ close -. 3.)
         |> ok))
    |> ok
  | Family.Sankey ->
    let nodes =
      List.mapi [ "Incoming"; "Reasoning"; "Tools"; "Complete" ] ~f:(fun i label ->
        D.Node.create ~id:(node_id (i + 1)) ~label |> ok)
    in
    let edges =
      List.mapi
        [ 1, 2, 65.; 1, 3, 35.; 2, 4, 65.; 3, 4, 35. ]
        ~f:(fun i (src, dst, value) ->
          D.Edge.create
            ~id:(edge_id (i + 1))
            ~source:(node_id src)
            ~target:(node_id dst)
            ~value
          |> ok)
    in
    D.sankey ~nodes ~edges |> ok
;;

(* The event identifies a publication. Call only after the registration reports
   its desired data published; never apply an old source offset to pending data. *)
let describe_selection data (selection : Gpuio.Chart_selection.t) =
  let open Option.Let_syntax in
  match D.Expert.contents data, selection with
  | Cartesian layers, Cartesian { series; span; aggregation } ->
    let%bind series =
      List.find_map layers ~f:(fun (Line s | Area s | Bar s) ->
        Option.some_if (D.Series_id.equal (D.Series.id s) series) s)
    in
    let points = D.Series.points series in
    let%bind first = List.nth points span.start_index in
    let%bind last = List.nth points (span.start_index + span.length - 1) in
    if
      not
        (D.Datum_id.equal (D.Point.id first) span.first
         && D.Datum_id.equal (D.Point.id last) span.last)
    then None
    else (
      match aggregation with
      | Exact ->
        let%map value = D.Point.y first in
        sprintf "%s · x %.3g · value %.3g" (D.Series.name series) (D.Point.x first) value
      | Sum | Mean ->
        Some (sprintf "%s · %d samples selected" (D.Series.name series) span.length))
  | Categorical _, _ -> Categorical.describe_selection data selection
  | Pie slices, Slice id ->
    let%map slice = List.find slices ~f:(fun s -> D.Datum_id.equal (D.Slice.id s) id) in
    sprintf "%s · %.3g" (D.Slice.label slice) (D.Slice.value slice)
  | Radar (axes, series), Radar { series = id; axis = axis_id } ->
    let%bind series =
      List.find series ~f:(fun s -> D.Series_id.equal (D.Radar_series.id s) id)
    in
    let%bind axis =
      List.find axes ~f:(fun a -> D.Datum_id.equal (D.Radar_axis.id a) axis_id)
    in
    let%map _, value =
      List.find (D.Radar_series.values series) ~f:(fun (id, _) ->
        D.Datum_id.equal id axis_id)
    in
    sprintf
      "%s · %s · %.3g / %.3g"
      (D.Radar_series.name series)
      (D.Radar_axis.label axis)
      value
      (D.Radar_axis.maximum axis)
  | Candlestick candles, Candlestick { span; aggregated } ->
    let%bind candle = List.nth candles span.start_index in
    if not (D.Datum_id.equal (D.Candle.id candle) span.first)
    then None
    else if aggregated
    then Some (sprintf "OHLC · %d sessions selected" span.length)
    else Some (sprintf "%s · close %.3g" (D.Candle.label candle) (D.Candle.close candle))
  | Sankey (nodes, _), Node id ->
    let%map node = List.find nodes ~f:(fun n -> D.Node_id.equal (D.Node.id n) id) in
    D.Node.label node
  | Sankey (nodes, edges), Edge id ->
    let%bind edge = List.find edges ~f:(fun e -> D.Edge_id.equal (D.Edge.id e) id) in
    let node id = List.find nodes ~f:(fun n -> D.Node_id.equal (D.Node.id n) id) in
    let%bind source = node (D.Edge.source edge) in
    let%map target = node (D.Edge.target edge) in
    sprintf
      "%s → %s · %.3g"
      (D.Node.label source)
      (D.Node.label target)
      (D.Edge.value edge)
  | (Cartesian _ | Pie _ | Radar _ | Candlestick _ | Sankey _), _ -> None
;;

let edge_data family =
  match family with
  | Family.Line ->
    D.line
      [ D.Series.create
          ~id:(series_id 1)
          ~name:"Original values"
          (List.init 100_000 ~f:(fun i ->
             D.Point.create
               ~id:(datum (100_000 - i))
               ~x:(Float.of_int i)
               ~y:(if i % 1000 = 0 then None else Some (Float.of_int (i % 17) -. 8.))
               ()
             |> ok))
        |> ok
      ]
    |> ok
  | Family.Area -> D.area [] |> ok
  | Family.Bar ->
    D.bar
      [ D.Series.create
          ~id:(series_id 1)
          ~name:"Signed values"
          (List.mapi [ -5.; 0.; 7. ] ~f:(fun i y ->
             D.Point.create ~id:(datum (i + 1)) ~x:(Float.of_int i) ~y:(Some y) () |> ok))
        |> ok
      ]
    |> ok
  | Family.Pie ->
    D.pie
      [ D.Slice.create ~id:(datum 1) ~label:"Zero slice" ~value:0. |> ok
      ; D.Slice.create ~id:(datum 2) ~label:"Visible slice" ~value:1. |> ok
      ]
    |> ok
  | Family.Radar ->
    D.radar
      ~axes:
        (List.mapi [ "First axis"; "Second axis"; "Third axis" ] ~f:(fun i label ->
           D.Radar_axis.create ~id:(datum (i + 1)) ~label ~maximum:1. |> ok))
      [ D.Radar_series.create
          ~id:(series_id 1)
          ~name:"Reordered axes"
          [ datum 3, 0.125; datum 1, 0.; datum 2, 1. ]
        |> ok
      ]
    |> ok
  | Family.Candlestick ->
    D.candlestick
      [ D.Candle.create
          ~id:(datum 1)
          ~x:0.
          ~label:"Flat session"
          ~open_:(-2.)
          ~high:(-2.)
          ~low:(-2.)
          ~close:(-2.)
        |> ok
      ]
    |> ok
  | Family.Sankey ->
    D.sankey
      ~nodes:
        (List.mapi [ "Source"; "Target"; "Isolated" ] ~f:(fun i label ->
           D.Node.create ~id:(node_id (i + 1)) ~label |> ok))
      ~edges:
        [ D.Edge.create ~id:(edge_id 1) ~source:(node_id 1) ~target:(node_id 2) ~value:0.
          |> ok
        ]
    |> ok
;;

module Preset = struct
  type t =
    | Standard
    | Mixed
    | Horizontal
    | Dense_legend
  [@@deriving equal]
end

let preset_data_exn preset family phase =
  validate_phase phase;
  match (preset : Preset.t) with
  | Standard -> data_exn family phase
  | Mixed ->
    D.cartesian
      [ Area (series ~phase ~id:1 ~name:"Capacity")
      ; Bar (series ~phase:(phase +. 4.) ~id:2 ~name:"Throughput")
      ; Line (series ~phase:(phase +. 8.) ~id:3 ~name:"Demand")
      ]
    |> ok
  | Horizontal ->
    D.bar
      [ series ~phase ~id:1 ~name:"Atlas"
      ; series ~phase:(phase +. 8.) ~id:2 ~name:"Nova"
      ]
    |> ok
  | Dense_legend ->
    D.pie
      (List.init
         (128 + Float.to_int (Float.min 128. phase))
         ~f:(fun i ->
           D.Slice.create
             ~id:(datum (i + 1))
             ~label:(sprintf "Channel %03d" (i + 1))
             ~value:(1. +. (Float.of_int (i % 3) *. phase))
           |> ok))
    |> ok
;;

let description preset family =
  match (preset : Preset.t) with
  | Standard -> Family.description family
  | Mixed -> "Capacity, throughput and demand"
  | Horizontal -> "Comparing evaluation throughput"
  | Dense_legend -> "Channel allocation"
;;

module Inspection = Inspection
module Sankey_presentation = Sankey_presentation
