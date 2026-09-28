open Core
module D = Gpuio.Chart_data

let ok = Or_error.ok_exn
let datum n = D.Datum_id.of_int64 (Int64.of_int n) |> ok
let series_id n = D.Series_id.of_int64 (Int64.of_int n) |> ok
let node_id n = D.Node_id.of_int64 (Int64.of_int n) |> ok
let edge_id n = D.Edge_id.of_int64 (Int64.of_int n) |> ok
let names = [| "Line"; "Area"; "Bar"; "Pie"; "Radar"; "Candlestick"; "Sankey" |]

let descriptions =
  [| "Response quality over time"
   ; "Capacity throughout the day"
   ; "Throughput by evaluation run"
   ; "Where the work happens"
   ; "A balanced model scorecard"
   ; "The shape of market movement"
   ; "From request to resolution"
  |]
;;

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

let data family phase =
  match family with
  | 0 ->
    D.line
      [ series ~phase ~id:1 ~name:"Atlas"
      ; series ~phase:(phase +. 8.) ~id:2 ~name:"Nova"
      ]
    |> ok
  | 1 -> D.area [ series ~phase ~id:1 ~name:"Active capacity" ] |> ok
  | 2 -> D.bar [ series ~phase ~id:1 ~name:"Completed evaluations" ] |> ok
  | 3 ->
    D.pie
      (List.mapi
         [ "Reasoning", 44.; "Code", 28.; "Research", 18.; "Other", 10. ]
         ~f:(fun i (label, value) ->
           D.Slice.create ~id:(datum (i + 1)) ~label ~value |> ok))
    |> ok
  | 4 ->
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
      ]
    |> ok
  | 5 ->
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
  | 6 ->
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
  | _ -> invalid_arg "unknown gallery family"
;;
