open Core
open Gpuio
module Q = Container_query
module W = Gpuio_protocol.Container_query_wire

let ok = Or_error.ok_exn
let id value = Q.Branch_id.of_string value |> ok
let compact = id "compact"
let wide = id "wide"
let tall = id "tall"
let range ?minimum ?maximum () = Q.Range.create ?minimum ?maximum () |> ok

let rule ?width ?height branch =
  Q.Rule.create ~condition:(Q.Predicate.create ?width ?height ()) ~branch
;;

let config () =
  Q.Config.create
    ~default:compact
    [ rule ~width:(range ~minimum:480.25 ()) ~height:(range ~minimum:200. ()) wide
    ; rule ~height:(range ~minimum:600. ()) tall
    ]
  |> ok
;;

let%expect_test "default, fractional boundaries, height and ordered overlap" =
  let config = config () in
  List.iter
    [ 480.24, 200.; 480.25, 200.; 480.26, 200.; 900., 199.99; 100., 600.; 900., 600. ]
    ~f:(fun (width, height) ->
      Q.Config.select config ~width ~height
      |> ok
      |> Q.Branch_id.to_string
      |> print_endline);
  let half_open =
    Q.Config.create
      ~default:compact
      [ rule ~width:(range ~minimum:480.25 ~maximum:900.5 ()) wide
      ; rule ~height:(range ~minimum:600. ()) tall
      ]
    |> ok
  in
  List.iter [ 900.49; 900.5; 901. ] ~f:(fun width ->
    Q.Config.select half_open ~width ~height:600.
    |> ok
    |> Q.Branch_id.to_string
    |> print_endline);
  let catch_all = Q.Config.create ~default:compact [ rule tall; rule wide ] |> ok in
  Q.Config.select catch_all ~width:900. ~height:600.
  |> ok
  |> Q.Branch_id.to_string
  |> print_endline;
  print_s [%sexp (Q.Config.branches config : Q.Branch_id.t list)];
  [%expect
    {|
    compact
    wide
    wide
    compact
    tall
    wide
    wide
    tall
    tall
    tall
    (compact wide tall)
    |}]
;;

let%expect_test "config matches independently constructed Rust and schema fixture" =
  let config = Q.Expert.to_wire (config ()) ~generation:42L |> ok in
  assert (W.Config.valid config);
  let bytes =
    Bin_prot.Utils.bin_dump [%bin_writer: W.Config.t] config |> Bigstring.to_string
  in
  let hex =
    String.to_list bytes
    |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> String.concat
  in
  Eio_main.run (fun env ->
    let path = Eio.Path.(Eio.Stdenv.fs env / "container-query.hex") in
    assert (String.equal hex (Eio.Path.load path |> String.strip)));
  let native_index = W.Config.select config ~width:480.25 ~height:600. in
  print_s [%sexp (native_index : int64)];
  [%expect {| 1 |}]
;;

let%expect_test "invalid bounds, IDs, dimensions and capacity limits" =
  List.iter [ Float.nan; Float.infinity; -1. ] ~f:(fun value ->
    assert (Result.is_error (Q.Range.create ~minimum:value ()));
    assert (Result.is_error (Q.Range.create ~maximum:value ()));
    assert (Result.is_error (Q.Config.select (config ()) ~width:value ~height:10.));
    assert (Result.is_error (Q.Config.select (config ()) ~width:10. ~height:value)));
  assert (Result.is_error (Q.Range.create ~minimum:2. ~maximum:2. ()));
  assert (Result.is_error (Q.Range.create ~minimum:2. ~maximum:1. ()));
  List.iter
    [ ""; "\000"; "\255"; String.make 129 'a' ]
    ~f:(fun name -> assert (Result.is_error (Q.Branch_id.of_string name)));
  assert (Result.is_ok (Q.Branch_id.of_string (String.make 128 'a')));
  assert (Result.is_ok (Q.Branch_id.of_string "宽"));
  assert (
    Result.is_ok (Q.Config.create ~default:compact (List.init 32 ~f:(fun _ -> rule wide))));
  assert (
    Result.is_error
      (Q.Config.create ~default:compact (List.init 33 ~f:(fun _ -> rule wide))));
  let rules n = List.init n ~f:(fun i -> rule (id (Int.to_string i))) in
  assert (Result.is_ok (Q.Config.create ~default:compact (rules 15)));
  assert (Result.is_error (Q.Config.create ~default:compact (rules 16)));
  List.iter [ 0L; -1L ] ~f:(fun generation ->
    assert (Result.is_error (Q.Expert.to_wire (config ()) ~generation)));
  let empty = Q.Config.create ~default:compact [] |> ok in
  print_s [%sexp (Q.Config.select empty ~width:0. ~height:0. |> ok : Q.Branch_id.t)];
  [%expect {| compact |}]
;;

let%expect_test "retained presentations preserve identity and fence selection delivery" =
  let module Wire = Gpuio_protocol.Wire in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let reconciler = Reconciler.create window in
  let presentations =
    [ compact, View.text "compact"; wide, View.text "wide"; tall, View.text "tall" ]
  in
  let view label config =
    View.container_query ~on_select:(fun selected -> label, selected) config presentations
    |> ok
  in
  let prepare value =
    Reconciler.prepare reconciler ~theme:Theme.default (Some value) |> ok
  in
  let first = prepare (view "first" (config ())) in
  let node, handler =
    match Reconciler.message first with
    | Some (Apply tx) ->
      List.find_map_exn tx.operations ~f:(function
        | Create (node, Container_query, _, Some handler) -> Some (node, handler)
        | _ -> None)
    | _ -> assert false
  in
  Reconciler.accept reconciler first |> ok;
  let latest = prepare (view "latest" (config ())) in
  assert (Option.is_none (Reconciler.message latest));
  Reconciler.accept reconciler latest |> ok;
  let snapshot : W.Snapshot.t =
    { generation = 1L; sequence = 1L; branch = 0L; width = 400.; height = 300. }
  in
  let event ?(revision = 1L) snapshot =
    Wire.Event.Container_selected (window, node, handler, revision, snapshot)
  in
  let deliver snapshot = Reconciler.dispatch reconciler (event snapshot) in
  let label, selected = deliver snapshot |> Option.value_exn in
  print_s [%sexp (label : string), (selected : Q.Selection.t)];
  assert (Option.is_none (deliver snapshot));
  List.iter
    [ { snapshot with generation = 2L; sequence = 2L }
    ; { snapshot with generation = 0L; sequence = 2L }
    ; { snapshot with branch = 1L; sequence = 2L }
    ; { snapshot with branch = 3L; sequence = 2L }
    ; { snapshot with width = Float.nan; sequence = 2L }
    ; { snapshot with sequence = 0L }
    ]
    ~f:(fun snapshot -> assert (Option.is_none (deliver snapshot)));
  assert (
    Option.is_none
      (Reconciler.dispatch
         reconciler
         (event ~revision:2L { snapshot with sequence = 2L })));
  assert (Option.is_some (deliver { snapshot with sequence = 2L }));
  let reordered = Q.Config.create ~default:compact [ rule tall; rule wide ] |> ok in
  let pending = prepare (view "reordered" reordered) in
  let revision =
    match Reconciler.message pending with
    | Some (Apply tx) ->
      assert (
        List.for_all tx.operations ~f:(function
          | Create _ | Remove _ | Bind _ -> false
          | _ -> true));
      assert (
        List.exists tx.operations ~f:(function
          | Set_container_query (_, c) -> Int64.equal c.generation 2L
          | _ -> false));
      tx.revision
    | _ -> assert false
  in
  (* Preparation does not replace the accepted callback/config. *)
  let label, _ = deliver { snapshot with sequence = 3L } |> Option.value_exn in
  assert (String.equal label "latest");
  Reconciler.accept reconciler pending |> ok;
  assert (Option.is_none (deliver { snapshot with sequence = 4L }));
  let next = { snapshot with generation = 2L; sequence = 4L; branch = 1L } in
  let label, selected =
    Reconciler.dispatch reconciler (event ~revision next) |> Option.value_exn
  in
  print_s [%sexp (label : string), (selected : Q.Selection.t)];
  assert (Option.is_none (Reconciler.dispatch reconciler (event ~revision next)));
  let gone = Reconciler.prepare reconciler ~theme:Theme.default None |> ok in
  Reconciler.accept reconciler gone |> ok;
  assert (
    Option.is_none
      (Reconciler.dispatch reconciler (event ~revision { next with sequence = 5L })));
  [%expect
    {|
    (latest ((branch compact) (width 400) (height 300)))
    (reordered ((branch tall) (width 400) (height 300)))
    |}]
;;

let%expect_test "presentation validation and independent transaction/event bytes" =
  let module Wire = Gpuio_protocol.Wire in
  List.iter
    [ []
    ; [ compact, View.text "x" ]
    ; [ compact, View.text "x"; wide, View.text "x"; wide, View.text "x" ]
    ; [ compact, View.text "x"; wide, View.text "x"; id "unknown", View.text "x" ]
    ]
    ~f:(fun presentations ->
      assert (Result.is_error (View.container_query (config ()) presentations)));
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok in
  let handler = Gpuio_protocol.Handler_id.create ~slot:0L ~generation:1L |> ok in
  let config = Q.Expert.to_wire (config ()) ~generation:42L |> ok in
  let message =
    Wire.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Create (node, Container_query, "", Some handler)
          ; Set_container_query (node, config)
          ]
      }
  in
  let request = Wire.Message.encode message |> ok in
  let snapshot : W.Snapshot.t =
    { generation = 42L; sequence = 9L; branch = 1L; width = 480.25; height = 600. }
  in
  let event snapshot =
    Wire.Event.Container_selected (window, node, handler, 7L, snapshot)
  in
  let encode snapshot =
    Bin_prot.Utils.bin_dump [%bin_writer: Wire.Event.t list] [ event snapshot ]
    |> Bigstring.to_string
  in
  let events = encode snapshot in
  Eio_main.run (fun env ->
    List.iter
      [ "container-query-request.hex", request; "container-query-events.hex", events ]
      ~f:(fun (name, bytes) ->
        let expected =
          Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / name) |> String.strip
        in
        let hex =
          String.to_list bytes
          |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
          |> String.concat
        in
        assert (String.equal hex expected)));
  assert (List.equal Wire.Event.equal (Wire.Event.decode events |> ok) [ event snapshot ]);
  for length = 0 to String.length events - 1 do
    assert (Result.is_error (Wire.Event.decode (String.prefix events length)))
  done;
  assert (Result.is_error (Wire.Event.decode (events ^ "\000")));
  List.iter
    [ { snapshot with generation = 0L }
    ; { snapshot with branch = 16L }
    ; { snapshot with sequence = 0L }
    ; { snapshot with width = Float.infinity }
    ; { snapshot with height = -1. }
    ]
    ~f:(fun invalid -> assert (Result.is_error (Wire.Event.decode (encode invalid))));
  print_s [%sexp (Q.Expert.selection_of_wire config snapshot |> ok : Q.Selection.t)];
  [%expect {| ((branch wide) (width 480.25) (height 600)) |}]
;;

let%expect_test "query capability handshake above 32 bits" =
  let module Wire = Gpuio_protocol.Wire in
  assert (Int64.equal (Int64.bit_and Wire.capabilities 8589934592L) 8589934592L);
  let bytes = Wire.Message.encode (Hello (Wire.version, Wire.capabilities)) |> ok in
  String.iter bytes ~f:(fun byte -> printf "%02x" (Char.to_int byte));
  print_endline "";
  [%expect {| 0001fcffffffffffff1f00 |}]
;;
