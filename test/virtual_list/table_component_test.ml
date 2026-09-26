open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module D = Gpuio.Table_data
module C = Gpuio.Table_column
module T = Gpuio.Table
module W = Gpuio_bonsai.Table
module View = Gpuio.View
module R = Gpuio.Reconciler

let ok = Or_error.ok_exn
let id n = D.Id.of_string (Int.to_string n) |> ok
let col s = C.Id.of_string s |> ok
let source n = D.create (List.init n ~f:(fun n -> id n, Int.to_string n)) |> ok

let config ?(max_active = 4) ?(names = [ "name"; "value" ]) () =
  let columns =
    List.map names ~f:(fun name ->
      C.create ~id:(col name) ~label:name ~sortable:true () |> ok)
    |> C.Collection.create
    |> ok
  in
  T.Config.create
    ~columns
    ~label:"Table"
    ~max_active_rows:max_active
    ~max_active_cells:(max_active * 2)
    ()
  |> ok
;;

let create component =
  Bonsai_driver.create
    ~action_history:Release_after_flush
    ~clock:(Bonsai.Time_source.create ~start:Time_ns.epoch)
    component
;;

let result d =
  Bonsai_driver.flush d;
  Bonsai_driver.result d |> ok
;;

let display d =
  Bonsai_driver.trigger_lifecycles d;
  ignore (result d : _ W.Output.t)
;;

let root output = View.Expert.describe (W.Output.view output)
let list output = (root output).virtual_list |> Option.value_exn
let table output = (list output).table |> Option.value_exn
let target o n = W.Output.target o (id n) |> ok
let key o n = D.Expert.row_key (target o n)

let observe d ?(pins = []) ?(start = false) ?(end_ = false) rows =
  let o = result d in
  let keys = List.map rows ~f:(key o) in
  let viewport : Gpuio.Virtual_list.Viewport.t =
    { visible_first = Option.value (List.hd rows) ~default:0
    ; visible_last = Option.value_map (List.last rows) ~default:0 ~f:Int.succ
    ; requested = keys
    ; pinned = List.map pins ~f:(key o)
    ; anchor = Option.map (List.hd keys) ~f:(fun key -> key, 0.)
    ; following_tail = false
    ; at_start = start
    ; at_end = end_
    ; budget_exhausted = false
    }
  in
  Bonsai_driver.schedule_event d (Option.value_exn (list o).on_viewport viewport)
;;

let text_cell ~row:_ ~data ~column ~lifetime:_ _ =
  B.map2 data column ~f:(fun data column -> W.Cell.text ~column:(C.id column) data)
;;

let execute d ui_effect =
  Bonsai_driver.schedule_event d ui_effect;
  ignore (result d : _ W.Output.t)
;;

let selected o =
  match W.Output.selection o with
  | T.Selection.Empty -> None
  | Row row | Cell (row, _) -> Some (D.Id.to_string (D.Row_ref.id row))
  | Column _ -> Some "column"
;;

let%expect_test "bounded cells, keyed updates, selection repair and query-local lifetimes"
  =
  let data = B.Expert.Var.create (source 20) in
  let cfg = B.Expert.Var.create (config ()) in
  let query = B.Expert.Var.create 0L in
  let mounted = ref 0
  and unmounted = ref 0 in
  let requests = ref 0 in
  let d =
    create (fun graph ->
      W.component
        (B.Expert.Var.value data)
        ~config:(B.Expert.Var.value cfg)
        ~query_generation:(B.Expert.Var.value query)
        ~on_request:(B.return (fun _ -> E.of_thunk (fun () -> Int.incr requests)))
        ~render_cell:(fun ~row ~data ~column ~lifetime graph ->
          B.Edge.lifecycle
            ~on_activate:(B.return (E.of_thunk (fun () -> Int.incr mounted)))
            ~on_deactivate:(B.return (E.of_thunk (fun () -> Int.incr unmounted)))
            graph;
          text_cell ~row ~data ~column ~lifetime graph)
        graph)
  in
  assert (W.Output.active_cells (result d) = 0);
  display d;
  observe d [ 0; 1; 2; 3; 4 ];
  assert (W.Output.active_rows (result d) = 4 && W.Output.active_cells (result d) = 8);
  assert (W.Output.budget_exhausted (result d));
  display d;
  assert (!mounted = 8 && !unmounted = 0);
  let o = result d in
  let old_key = key o 1 in
  execute d ((table o).on_input (T.Request.Select (Cell (old_key, col "value"))));
  assert (!requests = 1 && Option.equal String.equal (selected (result d)) (Some "1"));
  let controller = W.Output.controller (result d) in
  let delayed = W.Controller.reveal controller (target (result d) 1) in
  B.Expert.Var.set data (D.set (B.Expert.Var.get data) ~key:(id 1) ~data:"streamed" |> ok);
  ignore (result d : _ W.Output.t);
  display d;
  assert (!mounted = 8 && !unmounted = 0 && Gpuio.Key.equal old_key (key (result d) 1));
  observe d ~pins:[ 1 ] [ 10; 11; 12; 13 ];
  ignore (result d : _ W.Output.t);
  display d;
  assert (W.Output.active_rows (result d) = 4 && !mounted = 14 && !unmounted = 6);
  let old_native =
    (table (result d)).on_input (T.Request.Select (Row (key (result d) 10)))
  in
  B.Expert.Var.set query 1L;
  ignore (result d : _ W.Output.t);
  display d;
  assert (W.Output.active_rows (result d) = 1);
  assert (!mounted = 16 && !unmounted = 14);
  execute d delayed;
  execute d old_native;
  assert (List.is_empty (table (result d)).commands && !requests = 1);
  assert (Option.equal String.equal (selected (result d)) (Some "1"));
  B.Expert.Var.set cfg (config ~names:[ "name" ] ());
  assert (Option.is_none (selected (result d)));
  display d;
  B.Expert.Var.set cfg (config ());
  assert (Option.is_none (selected (result d)));
  display d;
  print_s
    [%sexp
      "bounded active cells; streaming preserves models; query resets cell lifetimes and \
       ignores old work"];
  Bonsai_driver.Expert.invalidate_observers d;
  [%expect
    {| "bounded active cells; streaming preserves models; query resets cell lifetimes and ignores old work" |}]
;;

let%expect_test
    "coalesced removal/reinsertion retires membership and pending batches acknowledge \
     once"
  =
  let data = B.Expert.Var.create (source 5) in
  let query = B.Expert.Var.create 0L in
  let d =
    create (fun graph ->
      W.component
        (B.Expert.Var.value data)
        ~config:(B.return (config ()))
        ~query_generation:(B.Expert.Var.value query)
        ~render_cell:text_cell
        graph)
  in
  ignore (result d : _ W.Output.t);
  display d;
  observe d [ 0; 1 ];
  ignore (result d : _ W.Output.t);
  display d;
  let old = target (result d) 1 in
  let old_key = D.Expert.row_key old in
  let controller = W.Output.controller (result d) in
  let batch =
    W.Controller.batch
      controller
      [ Set_selection (Row old); Reveal (old, Some (col "value")) ]
    |> ok
  in
  execute d batch;
  let commands = (table (result d)).commands in
  assert (List.length commands = 2);
  assert (List.equal Int64.equal (List.map commands ~f:T.Command.serial) [ 1L; 2L ]);
  display d;
  assert (List.is_empty (table (result d)).commands);
  let previous = B.Expert.Var.get data in
  let removed = D.splice previous ~at:1 ~remove:1 [] |> ok in
  let replaced = D.splice removed ~at:1 ~remove:0 [ id 1, "new" ] |> ok in
  B.Expert.Var.set data replaced;
  assert (not (Gpuio.Key.equal old_key (key (result d) 1)));
  assert (Option.is_none (selected (result d)));
  display d;
  execute d batch;
  assert (List.is_empty (table (result d)).commands);
  let fresh = target (result d) 1 in
  execute d (W.Controller.reveal (W.Output.controller (result d)) fresh);
  assert (
    List.equal
      Int64.equal
      (List.map (table (result d)).commands ~f:T.Command.serial)
      [ 3L ]);
  display d;
  let window = Gpuio_protocol.Window_id.create ~slot:1L ~generation:1L |> ok in
  let r = R.create window in
  let prepare () =
    R.prepare r ~theme:Gpuio.Theme.default (Some (W.Output.view (result d))) |> ok
  in
  let first = prepare () in
  R.accept r first |> ok;
  B.Expert.Var.set query 1L;
  let reset = prepare () in
  let ops =
    match R.message reset with
    | Some (Gpuio_protocol.Wire.Message.Apply tx) -> tx.operations
    | _ -> []
  in
  assert (
    List.exists ops ~f:(function
      | Bind _ -> true
      | _ -> false));
  assert (
    not
      (List.exists ops ~f:(function
         | Create (_, Virtual_list, _, _) -> true
         | _ -> false)));
  R.accept r reset |> ok;
  display d;
  execute d (W.Controller.reveal controller fresh);
  assert (List.is_empty (table (result d)).commands);
  assert (
    Or_error.is_error
      (W.Controller.batch controller (List.init 65 ~f:(fun _ -> T.Target.Scroll_to_end))));
  assert (Or_error.is_error (W.Controller.scroll_to controller ~offset:Float.nan fresh));
  print_s
    [%sexp
      "membership keys retire coalesced reincarnation; commands execute once; query \
       keeps native root"];
  Bonsai_driver.Expert.invalidate_observers d;
  [%expect
    {| "membership keys retire coalesced reincarnation; commands execute once; query keeps native root" |}]
;;

let%expect_test "retained controllers and delayed callbacks do not retain source payloads"
  =
  let weak = Stdlib.Weak.create 1 in
  let make_source () =
    let payload = String.init 4096 ~f:(fun n -> Char.of_int_exn (32 + (n mod 90))) in
    Stdlib.Weak.set weak 0 (Some payload);
    D.create [ id 0, payload ] |> ok
  in
  let data = B.Expert.Var.create (make_source ()) in
  let d =
    create (fun graph ->
      W.component
        (B.Expert.Var.value data)
        ~config:(B.return (config ()))
        ~render_cell:text_cell
        graph)
  in
  ignore (result d : _ W.Output.t);
  display d;
  let controller, late =
    let o = result d in
    W.Output.controller o, (table o).on_input (Select (Row (key o 0)))
  in
  B.Expert.Var.set data (source 0);
  ignore (result d : _ W.Output.t);
  display d;
  Gc.full_major ();
  assert (not (Stdlib.Weak.check weak 0));
  execute d (W.Controller.scroll_to_end controller);
  execute d late;
  assert (List.is_empty (table (result d)).commands);
  Bonsai_driver.Expert.invalidate_observers d;
  [%expect {| |}]
;;

let%expect_test "paged boundaries request only Ready, and retry is explicit" =
  let pager =
    Gpuio.Table_paging.create ~query:"first" (source 0) ~before:End ~after:(More None)
    |> ok
  in
  let snapshot = B.Expert.Var.create (Gpuio.Table_paging.snapshot pager) in
  let requests = ref [] in
  let paging =
    W.Paging.create
      ~request:(fun ~generation direction ->
        E.of_thunk (fun () -> requests := (generation, direction) :: !requests))
      ~retry:(fun ~generation:_ _ -> E.Ignore)
      ~cancel:(fun ~generation:_ _ -> E.Ignore)
  in
  let d =
    create (fun graph ->
      W.paged
        (B.Expert.Var.value snapshot)
        ~paging:(B.return paging)
        ~config:(B.return (config ()))
        ~render_cell:text_cell
        graph)
  in
  ignore (result d : _ W.Output.t);
  display d;
  assert (List.is_empty !requests);
  observe d ~start:true ~end_:true [];
  ignore (result d : _ W.Output.t);
  display d;
  assert (List.length !requests = 1);
  let request = Gpuio.Table_paging.request pager After |> ok |> Option.value_exn in
  ignore
    (Gpuio.Table_paging.fail pager request (Error.of_string "offline")
     : Gpuio.Table_paging.Completion.t);
  B.Expert.Var.set snapshot (Gpuio.Table_paging.snapshot pager);
  ignore (result d : _ W.Output.t);
  display d;
  observe d ~start:true ~end_:true [];
  ignore (result d : _ W.Output.t);
  display d;
  assert (List.length !requests = 1);
  print_s [%sexp (!requests : (int64 * Gpuio.Table_paging.Direction.t) list)];
  Bonsai_driver.Expert.invalidate_observers d;
  [%expect {| ((0 After)) |}]
;;

let%expect_test "100k rows traversed twice retain only bounded cell models" =
  let count = 100_000
  and page = 100 in
  let weak = Stdlib.Weak.create (count * 2) in
  let data = source count in
  let allocations = ref 0 in
  let d =
    create (fun graph ->
      W.component
        (B.return data)
        ~config:(B.return (config ~max_active:page ()))
        ~render_cell:(fun ~row ~data:_ ~column ~lifetime:_ graph ->
          let payload, set_payload = B.state_opt graph in
          let open B.Let_syntax in
          let activate =
            let%arr row = row
            and column = column
            and set_payload = set_payload in
            let open E.Let_syntax in
            let%bind text =
              E.of_thunk (fun () ->
                Int.incr allocations;
                let row = Int.of_string (D.Id.to_string (D.Row_ref.id row)) in
                let column = if C.Id.equal (C.id column) (col "name") then 0 else 1 in
                let text =
                  String.init 512 ~f:(fun i -> Char.of_int_exn (32 + ((i + row) mod 90)))
                in
                Stdlib.Weak.set weak ((row * 2) + column) (Some text);
                text)
            in
            set_payload (Some text)
          in
          B.Edge.lifecycle ~on_activate:activate graph;
          let%arr payload = payload
          and column = column in
          W.Cell.text
            ~column:(C.id column)
            (Option.value_map payload ~default:"cold" ~f:(fun p ->
               Int.to_string (String.length p))))
        graph)
  in
  ignore (result d : _ W.Output.t);
  display d;
  Gc.full_major ();
  let baseline = (Gc.stat ()).live_words in
  let live () = List.count (List.init (count * 2) ~f:Fn.id) ~f:(Stdlib.Weak.check weak) in
  for _ = 1 to 2 do
    for visit = 0 to (count / page) - 1 do
      observe d (List.init page ~f:(fun offset -> (visit * page) + offset));
      assert (W.Output.active_cells (result d) = page * 2);
      display d;
      if visit mod 50 = 49
      then (
        Gc.full_major ();
        assert (live () <= page * 2))
    done
  done;
  observe d [];
  ignore (result d : _ W.Output.t);
  display d;
  Gc.full_major ();
  assert (!allocations = count * 4 && live () = 0);
  assert ((Gc.stat ()).live_words - baseline < 200_000);
  assert (D.length data = count);
  Bonsai_driver.Expert.invalidate_observers d;
  print_endline
    "100k rows visited twice; 200 active cells maximum; no retired cell payloads; \
     bounded retained heap";
  [%expect
    {| 100k rows visited twice; 200 active cells maximum; no retired cell payloads; bounded retained heap |}]
;;

let%expect_test "superseded batches cannot publish a selection that was never displayed" =
  let d =
    create (fun graph ->
      W.component
        (B.return (source 5))
        ~config:(B.return (config ()))
        ~render_cell:text_cell
        graph)
  in
  ignore (result d : _ W.Output.t);
  display d;
  let c = W.Output.controller (result d) in
  execute d (W.Controller.select c (Row (target (result d) 0)));
  assert (Option.equal String.equal (selected (result d)) (Some "0"));
  execute d (W.Controller.reveal c (target (result d) 1));
  assert (Option.is_none (selected (result d)));
  display d;
  assert (Option.is_none (selected (result d)));
  execute d ((table (result d)).on_input (Select (Row (key (result d) 2))));
  execute d (W.Controller.select c (Row (target (result d) 3)));
  assert (Option.equal String.equal (selected (result d)) (Some "3"));
  execute d ((table (result d)).on_input (Select (Row (key (result d) 4))));
  assert (List.is_empty (table (result d)).commands);
  assert (Option.equal String.equal (selected (result d)) (Some "4"));
  display d;
  assert (Option.equal String.equal (selected (result d)) (Some "4"));
  Bonsai_driver.Expert.invalidate_observers d;
  [%expect {| |}]
;;

let%expect_test
    "public style owns one native box; source lineage resets independently of sibling key"
  =
  let module Wire = Gpuio_protocol.Wire in
  let caller_key = Gpuio.Key.of_string_exn (String.make 256 'k') in
  let data = B.Expert.Var.create (source 3) in
  let styled color =
    Gpuio.Style.create_exn
      [ Width (Gpuio.Length.px_exn 480.)
      ; Height (Gpuio.Length.px_exn 280.)
      ; Padding (Gpuio.Length.px_exn 12.)
      ; Background (Gpuio.Background.solid (Gpuio.Color.rgb_exn color))
      ]
  in
  let style = B.Expert.Var.create (styled 0x123456) in
  let d =
    create (fun graph ->
      W.component
        (B.Expert.Var.value data)
        ~config:(B.return (config ()))
        ~key:caller_key
        ~style:(B.Expert.Var.value style)
        ~render_cell:text_cell
        graph)
  in
  let r = R.create (Gpuio_protocol.Window_id.create ~slot:1L ~generation:1L |> ok) in
  let prepare () =
    let output = result d in
    let root = root output in
    assert (Option.value_exn root.key |> Gpuio.Key.equal caller_key);
    assert (
      Gpuio.Style.equal
        root.style
        (View.Expert.describe (View.column ~style:(B.Expert.Var.get style) [])).style);
    let pending =
      R.prepare r ~theme:Gpuio.Theme.default (Some (W.Output.view output)) |> ok
    in
    let operations =
      match R.message pending with
      | Some (Wire.Message.Apply transaction) -> transaction.operations
      | _ -> []
    in
    R.accept r pending |> ok;
    operations
  in
  let mounts operations =
    List.filter_map operations ~f:(function
      | Wire.Op.Create (node, Virtual_list, _, _) -> Some node
      | _ -> None)
  in
  let first = prepare () in
  assert (List.length (mounts first) = 1);
  assert (
    not
      (List.exists first ~f:(function
         | Wire.Op.Create (_, Container, _, _) -> true
         | _ -> false)));
  B.Expert.Var.set style (styled 0xabcdef);
  let changed = prepare () in
  assert (List.is_empty (mounts changed));
  assert (
    List.exists changed ~f:(function
      | Wire.Op.Set_style _ -> true
      | _ -> false));
  B.Expert.Var.set data (source 3);
  let replaced = prepare () in
  let old = List.hd_exn (mounts first) in
  let fresh = List.hd_exn (mounts replaced) in
  assert (not (Gpuio_protocol.Node_id.equal old fresh));
  print_s [%sexp "one styled root; style update retains mount; fresh lineage replaces it"];
  [%expect {| "one styled root; style update retains mount; fresh lineage replaces it" |}]
;;

let%expect_test
    "tables sharing a source retain independent caller keys through sibling reorder"
  =
  let data = B.return (source 1) in
  let reverse = B.Expert.Var.create false in
  let d =
    create (fun graph ->
      let first =
        W.component
          data
          ~config:(B.return (config ()))
          ~key:(Gpuio.Key.of_string_exn "first")
          ~render_cell:text_cell
          graph
      in
      let second =
        W.component
          data
          ~config:(B.return (config ()))
          ~key:(Gpuio.Key.of_string_exn "second")
          ~render_cell:text_cell
          graph
      in
      let open B.Let_syntax in
      let%arr first = first
      and second = second
      and reverse = B.Expert.Var.value reverse in
      let views = [ W.Output.view (ok first); W.Output.view (ok second) ] in
      Ok (View.column (if reverse then List.rev views else views)))
  in
  let r = R.create (Gpuio_protocol.Window_id.create ~slot:1L ~generation:1L |> ok) in
  let prepare () =
    let pending = R.prepare r ~theme:Gpuio.Theme.default (Some (result d)) |> ok in
    let operations =
      match R.message pending with
      | Some (Gpuio_protocol.Wire.Message.Apply transaction) -> transaction.operations
      | _ -> []
    in
    R.accept r pending |> ok;
    operations
  in
  ignore (prepare () : Gpuio_protocol.Wire.Op.t list);
  B.Expert.Var.set reverse true;
  let reordered = prepare () in
  assert (
    List.exists reordered ~f:(function
      | Splice _ -> true
      | _ -> false));
  assert (
    not
      (List.exists reordered ~f:(function
         | Create _ | Remove _ -> true
         | _ -> false)));
  print_s
    [%sexp "same source, distinct caller keys; reorder preserves both native tables"];
  [%expect
    {| "same source, distinct caller keys; reorder preserves both native tables" |}]
;;
