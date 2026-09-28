open Core
module D = Gpuio.Chart_data

let datum n = D.Datum_id.of_int64 (Int64.of_int n) |> Or_error.ok_exn
let series_id n = D.Series_id.of_int64 (Int64.of_int n) |> Or_error.ok_exn
let node_id n = D.Node_id.of_int64 (Int64.of_int n) |> Or_error.ok_exn
let edge_id n = D.Edge_id.of_int64 (Int64.of_int n) |> Or_error.ok_exn

let point ?(label = "") id x y =
  D.Point.create ~id:(datum id) ~x ~y ~label () |> Or_error.ok_exn
;;

let series id points =
  D.Series.create ~id:(series_id id) ~name:"Requests" points |> Or_error.ok_exn
;;

let slice id value =
  D.Slice.create ~id:(datum id) ~label:"日本語 🦀" ~value |> Or_error.ok_exn
;;

let axis id =
  D.Radar_axis.create ~id:(datum id) ~label:(Int.to_string id) ~maximum:10.
  |> Or_error.ok_exn
;;

let node id = D.Node.create ~id:(node_id id) ~label:(Int.to_string id) |> Or_error.ok_exn

let edge id source target value =
  D.Edge.create ~id:(edge_id id) ~source:(node_id source) ~target:(node_id target) ~value
  |> Or_error.ok_exn
;;

let candle id x =
  D.Candle.create
    ~id:(datum id)
    ~x
    ~label:"Day"
    ~open_:(-2.)
    ~high:0.
    ~low:(-4.)
    ~close:(-1.)
  |> Or_error.ok_exn
;;

let accepted result = Result.is_ok result

let report cases =
  print_s
    [%sexp
      (List.map cases ~f:(fun (name, result) -> name, accepted result)
       : (string * bool) list)]
;;

let%expect_test "all seven families admit empty and well-formed degenerate data" =
  report
    [ "line", D.line []
    ; "area", D.area []
    ; "bar", D.bar []
    ; "pie", D.pie []
    ; "radar", D.radar ~axes:[] []
    ; "candle", D.candlestick []
    ; "flow", D.sankey ~nodes:[] ~edges:[]
    ];
  let s = series 1 [ point 1 0. (Some 0.) ] in
  report
    [ "single line", D.line [ s ]
    ; "zero area", D.area [ s ]
    ; "zero bar", D.bar [ s ]
    ; "all-zero pie", D.pie [ slice 1 0.; slice 2 0. ]
    ; "negative candle", D.candlestick [ candle 1 0. ]
    ; "isolated nodes", D.sankey ~nodes:[ node 1; node 2 ] ~edges:[]
    ; "zero flow", D.sankey ~nodes:[ node 1; node 2 ] ~edges:[ edge 1 1 2 0. ]
    ];
  [%expect
    {|
    ((line true) (area true) (bar true) (pie true) (radar true) (candle true)
     (flow true))
    (("single line" true) ("zero area" true) ("zero bar" true)
     ("all-zero pie" true) ("negative candle" true) ("isolated nodes" true)
     ("zero flow" true))
    |}]
;;

let%expect_test "non-finite and over-range coordinates never enter data values" =
  let values =
    [ Float.nan; Float.infinity; Float.neg_infinity; 1e101; -1e101; -1e100; 1e100; 0. ]
  in
  print_s
    [%sexp
      (List.map values ~f:(fun x ->
         accepted (D.Point.create ~id:(datum 1) ~x ~y:(Some 0.) ()))
       : bool list)];
  print_s
    [%sexp
      (List.map values ~f:(fun y ->
         accepted (D.Point.create ~id:(datum 1) ~x:0. ~y:(Some y) ()))
       : bool list)];
  print_s
    [%sexp
      (List.map values ~f:(fun value ->
         accepted (D.Slice.create ~id:(datum 1) ~label:"Value" ~value))
       : bool list)];
  print_s
    [%sexp
      (List.map values ~f:(fun maximum ->
         accepted (D.Radar_axis.create ~id:(datum 1) ~label:"Axis" ~maximum))
       : bool list)];
  print_s
    [%sexp
      (List.map values ~f:(fun value ->
         accepted
           (D.Edge.create ~id:(edge_id 1) ~source:(node_id 1) ~target:(node_id 2) ~value))
       : bool list)];
  [%expect
    {|
    (false false false false false true true true)
    (false false false false false true true true)
    (false false false false false false true true)
    (false false false false false false true false)
    (false false false false false false true true) |}]
;;

let%expect_test "identity, order and explicit gaps are validated independently" =
  let gap = series 1 [ point 1 0. (Some (-3.)); point 2 1. None; point 3 2. (Some 4.) ] in
  let other = series 2 [ point 1 0. (Some 2.) ] in
  report
    [ "gap line", D.line [ gap ]
    ; "gap area", D.area [ gap ]
    ; "gap bar", D.bar [ gap ]
    ; "duplicate series", D.line [ other; other ]
    ; "series-local datum IDs", D.line [ gap; other ]
    ; "mixed plot", D.cartesian [ D.Layer.Line gap; Bar other ]
    ];
  print_s
    [%sexp
      (List.map
         [ [ point 1 0. None; point 1 1. None ]
         ; [ point 1 0. None; point 2 0. None ]
         ; [ point 1 1. None; point 2 0. None ]
         ]
         ~f:(fun points ->
           accepted (D.Series.create ~id:(series_id 1) ~name:"Series" points))
       : bool list)];
  print_s
    [%sexp
      (List.map [ -1L; 0L; 1L; Int64.max_value ] ~f:(fun value ->
         accepted (D.Datum_id.of_int64 value))
       : bool list)];
  [%expect
    {|
    (("gap line" true) ("gap area" true) ("gap bar" false)
     ("duplicate series" false) ("series-local datum IDs" true)
     ("mixed plot" true))
    (false false false)
    (false false true true)
    |}]
;;

let%expect_test "radar requires exact axes and per-axis value domains" =
  let axes = List.init 3 ~f:(fun index -> axis (index + 1)) in
  let make values =
    D.Radar_series.create
      ~id:(series_id 1)
      ~name:"Model"
      (List.map values ~f:(fun (id, value) -> datum id, value))
    |> Or_error.ok_exn
  in
  report
    [ "reordered complete", D.radar ~axes [ make [ 3, 8.; 1, 2.; 2, 0. ] ]
    ; "missing axis", D.radar ~axes [ make [ 1, 2.; 2, 0. ] ]
    ; "unknown axis", D.radar ~axes [ make [ 1, 2.; 2, 0.; 4, 1. ] ]
    ; "over maximum", D.radar ~axes [ make [ 1, 11.; 2, 0.; 3, 0. ] ]
    ; "duplicate axis", D.radar ~axes:[ axis 1; axis 1; axis 2 ] []
    ; "only two axes", D.radar ~axes:[ axis 1; axis 2 ] []
    ];
  print_s
    [%sexp
      (accepted
         (D.Radar_series.create
            ~id:(series_id 1)
            ~name:"Model"
            [ datum 1, 1.; datum 1, 2. ])
       : bool)];
  [%expect
    {|
    (("reordered complete" true) ("missing axis" false) ("unknown axis" false)
     ("over maximum" false) ("duplicate axis" false) ("only two axes" false))
    false |}]
;;

let%expect_test "candle OHLC invariants include zero-height and negative prices" =
  let make ~open_ ~high ~low ~close =
    D.Candle.create ~id:(datum 1) ~x:0. ~label:"Price" ~open_ ~high ~low ~close
  in
  print_s
    [%sexp
      (List.map
         [ 0., 0., 0., 0.
         ; -3., -1., -4., -2.
         ; 2., 1., 0., 1.
         ; 0., 2., 1., 2.
         ; 1., Float.nan, 0., 2.
         ]
         ~f:(fun (open_, high, low, close) -> accepted (make ~open_ ~high ~low ~close))
       : bool list)];
  report
    [ "duplicate ID", D.candlestick [ candle 1 0.; candle 1 1. ]
    ; "duplicate x", D.candlestick [ candle 1 0.; candle 2 0. ]
    ; "decreasing x", D.candlestick [ candle 1 1.; candle 2 0. ]
    ];
  [%expect
    {|
    (true true false false false)
    (("duplicate ID" false) ("duplicate x" false) ("decreasing x" false)) |}]
;;

let%expect_test "Sankey topology is validated without assuming flow conservation" =
  let nodes = [ node 1; node 2; node 3 ] in
  report
    [ ( "parallel edges"
      , D.sankey ~nodes ~edges:[ edge 1 1 2 10.; edge 2 1 2 3.; edge 3 2 3 1. ] )
    ; "cycle", D.sankey ~nodes ~edges:[ edge 1 1 2 1.; edge 2 2 3 1.; edge 3 3 1 1. ]
    ; "zero cycle", D.sankey ~nodes ~edges:[ edge 1 1 2 0.; edge 2 2 1 0. ]
    ; "missing node", D.sankey ~nodes ~edges:[ edge 1 1 4 1. ]
    ; "duplicate node", D.sankey ~nodes:[ node 1; node 1 ] ~edges:[]
    ; "duplicate edge", D.sankey ~nodes ~edges:[ edge 1 1 2 1.; edge 1 2 3 1. ]
    ];
  print_s
    [%sexp
      (accepted
         (D.Edge.create ~id:(edge_id 1) ~source:(node_id 1) ~target:(node_id 1) ~value:0.)
       : bool)];
  [%expect
    {|
    (("parallel edges" true) (cycle false) ("zero cycle" false)
     ("missing node" false) ("duplicate node" false) ("duplicate edge" false))
    false |}]
;;

let%expect_test "UTF-8 byte bounds and aggregate data/text budgets" =
  print_s
    [%sexp
      (List.map
         [ "日本語 🦀"
         ; "bad\000"
         ; "bad\n"
         ; "bad\r"
         ; "\255"
         ; String.make 256 'x'
         ; String.make 257 'x'
         ]
         ~f:(fun label -> accepted (D.Point.create ~id:(datum 1) ~x:0. ~y:None ~label ()))
       : bool list)];
  let points =
    List.init 100_000 ~f:(fun i ->
      point (i + 1) (Float.of_int i) (Some (Float.of_int (i mod 17))))
  in
  let large = series 1 points in
  let data = D.line [ large ] |> Or_error.ok_exn in
  print_s [%sexp (D.value_count data : int), (D.text_bytes data : int)];
  report
    [ "point budget across series", D.line [ large; series 2 [ point 1 0. None ] ]
    ; "32 series", D.line (List.init 32 ~f:(fun i -> series (i + 1) []))
    ; "33 series", D.line (List.init 33 ~f:(fun i -> series (i + 1) []))
    ; "256 slices", D.pie (List.init 256 ~f:(fun i -> slice (i + 1) 0.))
    ; "257 slices", D.pie (List.init 257 ~f:(fun i -> slice (i + 1) 0.))
    ; "257 nodes", D.sankey ~nodes:(List.init 257 ~f:(fun i -> node (i + 1))) ~edges:[]
    ];
  let labelled =
    series
      2
      (List.init 32768 ~f:(fun i ->
         point ~label:(String.make 256 'x') (i + 1) (Float.of_int i) None))
  in
  report [ "text budget includes series name", D.line [ labelled ] ];
  [%expect
    {|
    (true false false false false true false)
    (100000 8)
    (("point budget across series" false) ("32 series" true) ("33 series" false)
     ("256 slices" true) ("257 slices" false) ("257 nodes" false))
    (("text budget includes series name" false)) |}]
;;

let%expect_test "radar values and negative bars keep their explicit domains" =
  print_s
    [%sexp
      (List.map [ Float.nan; Float.infinity; -1.; 0.; 1e100; 1e101 ] ~f:(fun value ->
         accepted
           (D.Radar_series.create ~id:(series_id 1) ~name:"Axis" [ datum 1, value ]))
       : bool list)];
  report
    [ "negative bar", D.bar [ series 1 [ point 1 0. (Some (-4.)) ] ]
    ; ( "unordered DAG input"
      , D.sankey ~nodes:[ node 3; node 1; node 2 ] ~edges:[ edge 1 3 2 1.; edge 2 1 3 1. ]
      )
    ; ( "2048 parallel links"
      , D.sankey
          ~nodes:[ node 1; node 2 ]
          ~edges:(List.init 2048 ~f:(fun i -> edge (i + 1) 1 2 0.)) )
    ; ( "2049 links"
      , D.sankey
          ~nodes:[ node 1; node 2 ]
          ~edges:(List.init 2049 ~f:(fun i -> edge (i + 1) 1 2 0.)) )
    ];
  [%expect
    {|
    (false false false true true false)
    (("negative bar" true) ("unordered DAG input" true)
     ("2048 parallel links" true) ("2049 links" false)) |}]
;;

let%expect_test "Sankey acceptance matches reachability for every four-node graph" =
  let pairs =
    List.concat_map (List.range 0 4) ~f:(fun source ->
      List.filter_map (List.range 0 4) ~f:(fun target ->
        if source = target then None else Some (source, target)))
  in
  let mismatches = ref 0 in
  let dag_count = ref 0 in
  for mask = 0 to (1 lsl List.length pairs) - 1 do
    let included = List.filteri pairs ~f:(fun bit _ -> mask land (1 lsl bit) <> 0) in
    let reach = Array.init 4 ~f:(fun _ -> Array.create ~len:4 false) in
    List.iter included ~f:(fun (source, target) -> reach.(source).(target) <- true);
    for via = 0 to 3 do
      for source = 0 to 3 do
        for target = 0 to 3 do
          reach.(source).(target)
          <- reach.(source).(target) || (reach.(source).(via) && reach.(via).(target))
        done
      done
    done;
    let is_dag = List.for_all (List.range 0 4) ~f:(fun node -> not reach.(node).(node)) in
    let edges =
      List.mapi included ~f:(fun index (source, target) ->
        edge (index + 1) (source + 1) (target + 1) 1.)
    in
    let admitted = accepted (D.sankey ~nodes:[ node 4; node 2; node 1; node 3 ] ~edges) in
    if not (Bool.equal is_dag admitted) then Int.incr mismatches;
    if is_dag then Int.incr dag_count
  done;
  print_s [%sexp (!mismatches : int), (!dag_count : int)];
  [%expect {| (0 543) |}]
;;
