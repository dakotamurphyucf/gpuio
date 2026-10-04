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
  | Columns of Table.Column_viewport.t
[@@deriving sexp_of]

let view
      ?(config = config ())
      ?(query = 0L)
      ?commands
      ?headers
      ?header_presentation
      ?row_presentations
      ?(rows = [ "a", "hello" ])
      ?(keys = [ "a"; "b" ])
      ()
  =
  View.Expert.managed_table
    ~config
    ~query_generation:query
    ?commands
    ?headers
    ?header_presentation
    ?row_presentations
    ~order:(Virtual_list.Order.create (List.map keys ~f:key) |> Or_error.ok_exn)
    ~on_input:(fun input -> Input input)
    ~on_column_viewport:(fun viewport -> Columns viewport)
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

let%expect_test "behavior changes retain nodes, retire queued input and reset explicitly" =
  let base =
    Table.Config.create ~columns:(columns ()) ~label:"Results" ~column_selection:true ()
    |> Or_error.ok_exn
  in
  let restricted =
    Table.Config.with_selectable_headers base (Some [ col "name" ]) |> Or_error.ok_exn
  in
  let restricted =
    Table.Config.with_boundary (Table.Config.with_row_header restricted false) Stop
  in
  let r = R.create window in
  let initial = prepare r (view ~config:base ()) in
  let route = route initial in
  accept r initial;
  let send ?(schema = 1L) request = R.dispatch r (event route ~schema request) in
  assert (Option.is_some (send (Select (Column "value"))));
  let candidate = prepare r (view ~config:restricted ()) in
  (* Preparing a candidate cannot retire the accepted route. *)
  assert (Option.is_some (send (Select (Column "value"))));
  let summarize u =
    List.filter_map (ops u) ~f:(function
      | W.Op.Set_table (_, c) -> Some ("schema", c.schema_revision)
      | Set_table_behavior (_, b) ->
        Some ((if Option.is_some b then "policy" else "clear"), 0L)
      | Create _ | Remove _ -> failwith "policy replaced retained nodes"
      | _ -> None)
  in
  print_s [%sexp (summarize candidate : (string * int64) list)];
  accept r candidate;
  assert (Option.is_none (send (Select (Column "name"))));
  List.iter
    [ T.Request.Select (Column "value"); Context (Column "value"); Copy (Column "value") ]
    ~f:(fun request -> assert (Option.is_none (send ~schema:2L request)));
  List.iter
    [ T.Request.Select (Column "name")
    ; Select (Cell (1L, "value"))
    ; Sort ("name", Some Ascending)
    ]
    ~f:(fun request -> assert (Option.is_some (send ~schema:2L request)));
  assert (List.is_empty (ops (prepare r (view ~config:restricted ()))));
  let reset = prepare r (view ~config:base ()) in
  print_s [%sexp (summarize reset : (string * int64) list)];
  accept r reset;
  assert (Option.is_some (send ~schema:3L (Select (Column "value"))));
  [%expect
    {|
    ((schema 2) (policy 0))
    ((schema 3) (clear 0))
  |}]
;;

let%expect_test
    "table colors preserve schema routes, padding retires geometry, tokens resolve \
     atomically"
  =
  let module A = Table.Appearance in
  let base = config () in
  let appearance ?padding colors =
    A.create ~striped:true ?padding ~colors () |> Or_error.ok_exn
  in
  let config appearance =
    Table.Config.with_appearance base appearance |> Or_error.ok_exn
  in
  let colored = config (appearance [ Header_background, Color.token_exn "header" ]) in
  let theme color = Theme.create [ "header", Color.rgb_exn color ] |> Or_error.ok_exn in
  let r = R.create window in
  let prepare theme config =
    R.prepare r ~theme (Some (view ~config ())) |> Or_error.ok_exn
  in
  let initial = prepare (theme 0xff0000) base in
  let route = route initial in
  accept r initial;
  let paint = prepare (theme 0xff0000) colored in
  assert (
    List.for_all (ops paint) ~f:(function
      | W.Op.Set_table_appearance _ -> true
      | _ -> false));
  accept r paint;
  let re_theme = prepare (theme 0x00ff00) colored in
  assert (List.length (ops re_theme) = 1);
  accept r re_theme;
  assert (Option.is_some (R.dispatch r (event route (Select (Row 1L)))));
  assert (
    Or_error.is_error (R.prepare r ~theme:Theme.default (Some (view ~config:colored ()))));
  let padded = config (appearance ~padding:A.Padding.zero []) in
  let update = prepare Theme.default padded in
  assert (
    List.exists (ops update) ~f:(function
      | W.Op.Set_table (_, c) -> Int64.equal c.schema_revision 2L
      | _ -> false));
  accept r update;
  assert (Option.is_none (R.dispatch r (event route (Select (Row 1L)))));
  assert (Option.is_some (R.dispatch r (event route ~schema:2L (Select (Row 1L)))));
  let reset = prepare Theme.default base in
  assert (
    List.exists (ops reset) ~f:(function
      | W.Op.Set_table_appearance (_, None) -> true
      | _ -> false));
  accept r reset;
  print_endline
    "paint/theme updates retain route; padding/reset advance geometry; missing token \
     rejects atomically";
  [%expect
    {| paint/theme updates retain route; padding/reset advance geometry; missing token rejects atomically |}]
;;

let%expect_test "column observations fence schema query revision and mounted identity" =
  let r = R.create window in
  let initial = prepare r (view ()) in
  let node, handler = route initial in
  accept r initial;
  let sample : T.Column_viewport.t =
    { schema_revision = 1L
    ; query_generation = 0L
    ; columns = [ "name", Unpinned, true; "value", Unpinned, false ]
    }
  in
  let observed ?(revision = 1L) ?(handler = handler) viewport =
    W.Event.Table_columns_observed (window, node, handler, revision, viewport)
  in
  (match R.dispatch r (observed sample) with
   | Some (Columns viewport) ->
     let columns = Table.Column_viewport.columns viewport in
     assert (
       List.equal
         Table_column.Id.equal
         (List.map columns ~f:Table.Column_viewport.Column.id)
         [ col "name"; col "value" ]);
     assert (
       List.equal
         Bool.equal
         (List.map columns ~f:Table.Column_viewport.Column.fully_visible)
         [ true; false ])
   | _ -> assert false);
  List.iter
    [ observed ~revision:0L sample
    ; observed ~revision:2L sample
    ; observed { sample with schema_revision = 2L }
    ; observed { sample with query_generation = 1L }
    ; observed { sample with columns = [ "absent", Unpinned, true ] }
    ; observed { sample with columns = [ "name", Left, true ] }
    ; observed
        { sample with columns = [ "name", Unpinned, true; "name", Unpinned, false ] }
    ; observed
        ~handler:
          (Gpuio_protocol.Handler_id.create ~slot:0L ~generation:9L |> Or_error.ok_exn)
        sample
    ]
    ~f:(fun event -> assert (Option.is_none (R.dispatch r event)));
  let update = prepare r (view ~config:(config ~width:240. ()) ()) in
  accept r update;
  assert (Option.is_none (R.dispatch r (observed sample)));
  assert (
    Option.is_some
      (R.dispatch
         r
         (observed ~revision:2L { sample with schema_revision = 2L; columns = [] })));
  print_endline
    "stable column IDs; empty snapshot valid; stale revision/schema/query/handler and \
     malformed identity rejected";
  [%expect
    {| stable column IDs; empty snapshot valid; stale revision/schema/query/handler and malformed identity rejected |}]
;;

let%expect_test "header slots retain keys separately from rows and fence remapping" =
  let r = R.create window in
  let header column text =
    Table_header.create
      ~key:(key "a")
      ~target:(Table_header.Target.column (col column))
      (View.text text)
  in
  let initial = prepare r (view ~headers:[ header "name" "rich" ] ()) in
  let slot =
    List.find_map_exn (ops initial) ~f:(function
      | W.Op.Set_table_header (node, Some _) -> Some node
      | _ -> None)
  in
  assert (
    List.exists (ops initial) ~f:(function
      | W.Op.Set_list_rows (_, [ row ]) ->
        not (Gpuio_protocol.Node_id.equal row.node slot)
      | _ -> false));
  accept r initial;
  let updated = prepare r (view ~headers:[ header "name" "streamed" ] ()) in
  assert (
    List.for_all (ops updated) ~f:(function
      | W.Op.Set_text _ -> true
      | _ -> false));
  accept r updated;
  let moved = prepare r (view ~headers:[ header "value" "streamed" ] ()) in
  assert (
    List.exists (ops moved) ~f:(function
      | W.Op.Set_table_header (node, Some (Column "value")) ->
        Gpuio_protocol.Node_id.equal node slot
      | _ -> false));
  assert (
    List.exists (ops moved) ~f:(function
      | W.Op.Set_table (_, c) -> Int64.equal c.schema_revision 2L
      | _ -> false));
  assert (
    not
      (List.exists (ops moved) ~f:(function
         | W.Op.Create _ | Remove _ | Set_list_rows _ -> true
         | _ -> false)));
  accept r moved;
  let removed = prepare r (view ()) in
  assert (
    List.exists (ops removed) ~f:(function
      | W.Op.Set_table (_, c) -> Int64.equal c.schema_revision 3L
      | _ -> false));
  assert (
    not
      (List.exists (ops removed) ~f:(function
         | W.Op.Set_list_rows _ -> true
         | _ -> false)));
  accept r removed;
  print_endline
    "Header/body key collision is safe; content updates retain placement; moving and \
     removing fence gestures without row churn";
  [%expect
    {| Header/body key collision is safe; content updates retain placement; moving and removing fence gestures without row churn |}]
;;

let%expect_test "header and keyed row paint changes preserve native table generations" =
  let module P = Table_presentation in
  let style = Style.create_exn [ Foreground (Color.token_exn "paint") ] in
  let header_presentation = P.Header.create style |> Or_error.ok_exn in
  let row = P.Row.create style |> Or_error.ok_exn in
  let r = R.create window in
  let themed color = Theme.create [ "paint", Color.rgb_exn color ] |> Or_error.ok_exn in
  let submit theme view =
    let update = R.prepare r ~theme (Some view) |> Or_error.ok_exn in
    accept r update;
    ops update
  in
  ignore (submit (themed 0xff0000) (view ()) : W.Op.t list);
  let styled () = view ~header_presentation ~row_presentations:[ key "a", row ] () in
  List.iter [ 0xff0000; 0x00ff00 ] ~f:(fun color ->
    let changes = submit (themed color) (styled ()) in
    assert (List.length changes = 2);
    assert (
      List.for_all changes ~f:(function
        | W.Op.Set_table_header_style _ | Set_table_row_style _ -> true
        | _ -> false)));
  let clear = submit Theme.default (view ()) in
  assert (List.length clear = 2);
  assert (
    List.for_all clear ~f:(function
      | W.Op.Set_table_header_style (_, []) | Set_table_row_style (_, []) -> true
      | _ -> false));
  print_endline
    "Theme, update and clear change only scoped paint metadata; no remount, schema, row \
     or handler churn";
  [%expect
    {| Theme, update and clear change only scoped paint metadata; no remount, schema, row or handler churn |}]
;;
