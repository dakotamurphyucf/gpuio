open Core
open Gpuio
module W = Gpuio_protocol.Wire
module T = Gpuio_protocol.Table_wire
module R = Reconciler

let key = Key.of_string_exn
let col name = Table_column.Id.of_string name |> Or_error.ok_exn
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn

let columns ?(width = 160.) () =
  [ Table_column.create ~id:(col "name") ~label:"Name" ~width ~sortable:true ()
    |> Or_error.ok_exn
  ; Table_column.create ~id:(col "value") ~label:"Value" ~resizable:false ()
    |> Or_error.ok_exn
  ]
  |> Table_column.Collection.create
  |> Or_error.ok_exn
;;

let config ?width ?disabled ?selection_mode ?sort () =
  Table.Config.create
    ~columns:(columns ?width ())
    ~label:"Results"
    ~max_active_rows:4
    ~max_active_cells:8
    ?disabled
    ?selection_mode
    ?sort
    ()
  |> Or_error.ok_exn
;;

let cell column text =
  Table.Cell.create ~column:(col column) ~copy_text:text |> Or_error.ok_exn
;;

type action =
  | Input of Key.t Table.Request.t
  | Viewport of Virtual_list.Viewport.t
  | Retain of Key.t list
[@@deriving sexp_of]

let view
      ?(config = config ())
      ?(query = 0L)
      ?commands
      ?(rows = [ "a", "hello" ])
      ?(keys = [ "a"; "b" ])
      ()
  =
  View.Expert.managed_table
    ~config
    ~query_generation:query
    ?commands
    ~order:(Virtual_list.Order.create (List.map keys ~f:key) |> Or_error.ok_exn)
    ~on_input:(fun input -> Input input)
    ~on_viewport:(fun viewport -> Viewport viewport)
    ~on_retain:(fun rows -> Retain rows)
    (List.map rows ~f:(fun (id, text) ->
       ( key id
       , [ cell "name" text, View.text text; cell "value" "日本語👨‍👩‍👧‍👦", View.text "value" ] )))
  |> Or_error.ok_exn
;;

let prepare r v = R.prepare r ~theme:Theme.default (Some v) |> Or_error.ok_exn
let accept r u = R.accept r u |> Or_error.ok_exn

let ops u =
  match R.message u with
  | Some (W.Message.Apply tx) -> tx.operations
  | None -> []
  | Some _ -> assert false
;;

let route u =
  List.find_map_exn (ops u) ~f:(function
    | W.Op.Create (n, Virtual_list, _, Some h) -> Some (n, h)
    | _ -> None)
;;

let event (node, handler) ?(schema = 1L) ?(query = 0L) ?(revision = 1L) request =
  W.Event.Table_input
    ( window
    , node
    , handler
    , revision
    , { T.Input.schema_revision = schema; query_generation = query; request } )
;;

let command ?(query = 0L) serial target =
  Table.Command.create ~serial ~query_generation:query target |> Or_error.ok_exn
;;

let%expect_test "100k logical rows materialize only explicit cells, not native callbacks" =
  let r = R.create window in
  let keys = List.init 100_000 ~f:Int.to_string in
  let initial = prepare r (view ~keys ~rows:[ "50000", "first" ] ()) in
  let operations = ops initial in
  let created =
    List.count operations ~f:(function
      | W.Op.Create _ -> true
      | _ -> false)
  in
  let cells =
    List.count operations ~f:(function
      | W.Op.Set_table_cell _ -> true
      | _ -> false)
  in
  let table =
    List.find_map_exn operations ~f:(function
      | W.Op.Set_table (_, c) -> Some c
      | _ -> None)
  in
  assert (Int64.equal table.schema_revision 1L);
  assert (
    List.exists operations ~f:(function
      | W.Op.Set_list_config (_, c) -> c.managed && Int64.equal c.max_active 4L
      | _ -> false));
  print_s [%sexp (created : int), (cells : int)];
  accept r initial;
  let update = prepare r (view ~keys ~rows:[ "50000", "streamed" ] ()) in
  assert (
    List.for_all (ops update) ~f:(function
      | W.Op.Set_text _ | Set_table_cell _ -> true
      | _ -> false));
  assert (List.length (ops update) = 2);
  accept r update;
  let retained =
    R.retain_list_rows r [ { node = fst (route initial); rows = [ 50_001L ] } ]
    |> Or_error.ok_exn
  in
  print_s [%sexp (retained : action list)];
  [%expect
    {|
    (6 2)
    ((Retain (50000)))
    |}]
;;

let%expect_test
    "query and schema fence queued input; removed and reincarnated rows stay retired"
  =
  let r = R.create window in
  let initial = prepare r (view ()) in
  let node, handler = route initial in
  accept r initial;
  print_s
    [%sexp
      (R.dispatch r (event (node, handler) (Select (Cell (1L, "value")))) : action option)];
  let resized = prepare r (view ~config:(config ~width:240. ()) ()) in
  assert (
    List.exists (ops resized) ~f:(function
      | W.Op.Set_table (_, c) -> Int64.equal c.schema_revision 2L
      | _ -> false));
  accept r resized;
  assert (Option.is_none (R.dispatch r (event (node, handler) (Select (Row 1L)))));
  assert (
    Option.is_some (R.dispatch r (event (node, handler) ~schema:2L (Select (Row 1L)))));
  let reset = prepare r (view ~config:(config ~width:240. ()) ~query:1L ()) in
  let next_handler =
    List.find_map_exn (ops reset) ~f:(function
      | W.Op.Bind (_, Some h) -> Some h
      | _ -> None)
  in
  accept r reset;
  assert (
    Option.is_none
      (R.dispatch r (event (node, handler) ~schema:2L ~query:1L (Select (Row 1L)))));
  assert (
    Option.is_none
      (R.dispatch r (event (node, next_handler) ~schema:2L (Select (Row 1L)))));
  let current request = event (node, next_handler) ~schema:2L ~query:1L request in
  assert (Option.is_some (R.dispatch r (current (Select (Row 1L)))));
  accept
    r
    (prepare r (view ~config:(config ~width:240. ()) ~query:1L ~keys:[ "b" ] ~rows:[] ()));
  assert (Option.is_none (R.dispatch r (current (Select (Row 1L)))));
  accept r (prepare r (view ~config:(config ~width:240. ()) ~query:1L ()));
  assert (Option.is_none (R.dispatch r (current (Select (Row 1L)))));
  print_s [%sexp (R.dispatch r (current (Select (Row 3L))) : action option)];
  R.close r;
  assert (Option.is_none (R.dispatch r (current (Select (Row 3L)))));
  [%expect
    {|
    ((Input (Select (Cell a value))))
    ((Input (Select (Row a))))
    |}]
;;

let%expect_test
    "commands execute once in order and failed prepares do not consume serials"
  =
  let r = R.create window in
  let commands =
    [ command 1L (Set_selection (Cell (key "b", col "value")))
    ; command 2L (Reveal (key "b", Some (col "value")))
    ]
  in
  let initial = prepare r (view ~commands ()) in
  let sent u =
    List.filter_map (ops u) ~f:(function
      | W.Op.Table_command (_, c) -> Some c.serial
      | _ -> None)
  in
  print_s [%sexp (sent initial : int64 list)];
  accept r initial;
  let repeat = prepare r (view ~commands ()) in
  assert (List.is_empty (sent repeat));
  accept r repeat;
  accept r (prepare r (view ()));
  let rejected commands =
    Or_error.is_error (R.prepare r ~theme:Theme.default (Some (view ~commands ())))
  in
  assert (rejected commands);
  assert (rejected [ command 3L Scroll_to_end; command 4L (Reveal (key "absent", None)) ]);
  assert (rejected [ command ~query:1L 3L Scroll_to_end ]);
  assert (rejected [ command 3L (Scroll_to (key "b", 32.)) ]);
  let next = prepare r (view ~commands:[ command 3L Reset_columns ] ()) in
  print_s [%sexp (sent next : int64 list)];
  accept r next;
  accept r (prepare r (view ~query:1L ()));
  assert (Or_error.is_error (R.prepare r ~theme:Theme.default (Some (view ()))));
  [%expect
    {|
    (1 2)
    (3)
    |}]
;;

let%expect_test
    "request policy validates current columns, widths, mode and disabled state"
  =
  let c = config () in
  let decode config request =
    Table.Expert.request_of_wire config request ~find_key:(function
      | 1L -> Some "row"
      | _ -> None)
  in
  let rejected =
    [ T.Request.Resize [ "name", Float.nan ]
    ; Resize [ "name", 1. ]
    ; Resize [ "value", 200. ]
    ; Resize [ "name", 180.; "name", 180. ]
    ; Select (Column "name")
    ; Copy Empty
    ; Sort ("value", Some Ascending)
    ; Activate (2L, None)
    ; Move ("unknown", None)
    ]
  in
  assert (List.for_all rejected ~f:(fun request -> Option.is_none (decode c request)));
  assert (Option.is_none (decode (config ~disabled:true ()) (Select (Row 1L))));
  assert (
    Option.is_none (decode (config ~selection_mode:Rows ()) (Select (Cell (1L, "name")))));
  print_s
    [%sexp
      (decode c (Resize [ "name", 200.; "value", 160. ]) : string Table.Request.t option)];
  print_s
    [%sexp (decode c (Sort ("name", Some Descending)) : string Table.Request.t option)];
  assert (
    Or_error.is_error
      (Table.Cell.create ~column:(col "name") ~copy_text:(String.make 65_537 'a')));
  assert (Or_error.is_error (Table.Cell.create ~column:(col "name") ~copy_text:"\000"));
  assert (Or_error.is_error (Table.Cell.create ~column:(col "name") ~copy_text:"\255"));
  [%expect
    {|
    ((Resize ((name 200) (value 160))))
    ((Sort name (Descending)))
    |}]
;;

let%expect_test "table row admission and specialization replacement are atomic" =
  let cfg = config () in
  let order =
    Virtual_list.Order.create [ key "a"; key "b"; key "c"; key "d"; key "e" ]
    |> Or_error.ok_exn
  in
  let build rows =
    View.Expert.managed_table
      ~config:cfg
      ~query_generation:0L
      ~order
      ~on_input:(fun input -> Input input)
      ~on_viewport:(fun v -> Viewport v)
      ~on_retain:(fun keys -> Retain keys)
      rows
  in
  let cells = [ cell "name" "n", View.text "n"; cell "value" "v", View.text "v" ] in
  assert (Or_error.is_error (build [ key "a", List.rev cells ]));
  assert (Or_error.is_error (build [ key "a", [ List.hd_exn cells ] ]));
  assert (Or_error.is_error (build [ key "a", cells; key "a", cells ]));
  assert (Or_error.is_error (build [ key "missing", cells ]));
  assert (
    Or_error.is_error
      (build (List.map [ "a"; "b"; "c"; "d"; "e" ] ~f:(fun id -> key id, cells))));
  let r = R.create window in
  let first = prepare r (build [ key "a", cells ] |> Or_error.ok_exn) in
  let root, _ = route first in
  accept r first;
  let list_config = Table.Expert.list_config cfg in
  let list =
    View.Expert.managed_virtual_list
      ~config:list_config
      ~order
      ~on_viewport:(fun v -> Viewport v)
      ~on_retain:(fun keys -> Retain keys)
      []
    |> Or_error.ok_exn
  in
  let replacement = prepare r list in
  assert (
    List.exists (ops replacement) ~f:(function
      | W.Op.Remove id -> Gpuio_protocol.Node_id.equal id root
      | _ -> false));
  let next_root, _ = route replacement in
  assert (not (Gpuio_protocol.Node_id.equal root next_root));
  accept r replacement;
  let restored = prepare r (build [] |> Or_error.ok_exn) in
  assert (
    List.exists (ops restored) ~f:(function
      | W.Op.Set_table (_, c) -> Int64.equal c.schema_revision 1L
      | _ -> false));
  accept r restored;
  print_s
    [%sexp
      "invalid rows rejected; table/list specialization gets a fresh native identity"];
  [%expect
    {| "invalid rows rejected; table/list specialization gets a fresh native identity" |}]
;;

let%expect_test
    "schema reorder preserves keyed cells and rejected preparation preserves revision"
  =
  let cfg = config () in
  let r = R.create window in
  let first = prepare r (view ~config:cfg ()) in
  accept r first;
  let moved =
    Table_column.Collection.move
      (Table.Config.columns cfg)
      ~column:(col "value")
      ~before:(Some (col "name"))
    |> Or_error.ok_exn
  in
  let cfg = Table.Config.with_columns cfg moved |> Or_error.ok_exn in
  let build ?commands () =
    View.Expert.managed_table
      ~config:cfg
      ~query_generation:0L
      ?commands
      ~order:(Virtual_list.Order.create [ key "a"; key "b" ] |> Or_error.ok_exn)
      ~on_input:(fun input -> Input input)
      ~on_viewport:(fun v -> Viewport v)
      ~on_retain:(fun keys -> Retain keys)
      [ ( key "a"
        , [ cell "value" "日本語👨‍👩‍👧‍👦", View.text "value"
          ; cell "name" "hello", View.text "hello"
          ] )
      ]
    |> Or_error.ok_exn
  in
  assert (
    Or_error.is_error
      (R.prepare
         r
         ~theme:Theme.default
         (Some (build ~commands:[ command 1L (Reveal (key "gone", None)) ] ()))));
  let reordered = prepare r (build ()) in
  let changes = ops reordered in
  assert (
    List.for_all changes ~f:(function
      | W.Op.Set_table _ | Splice _ -> true
      | _ -> false));
  let c =
    List.find_map_exn changes ~f:(function
      | W.Op.Set_table (_, c) -> Some c
      | _ -> None)
  in
  assert (Int64.equal c.schema_revision 2L);
  accept r reordered;
  let unchanged = prepare r (build ()) in
  assert (List.is_empty (ops unchanged));
  print_s [%sexp (List.length changes : int), (c.schema_revision : int64)];
  [%expect {| (2 2) |}]
;;
