open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module P = Gpuio.Pagination
module N = Gpuio.Number_input
module C = Pagination_component
module W = Gpuio_protocol.Wire
module NW = Gpuio_protocol.Number_input_wire
module Driver = Gpuio_runtime_core.Window_driver

let ok = Or_error.ok_exn
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok
let overlay = Gpuio.Overlay.Config.create ~label:"Choose page" () |> ok
let pages = P.create ~total_pages:P.max_pages ~current:500_000_000 () |> ok

type request =
  { expected : N.Snapshot.t
  ; complete : (N.Snapshot.t, N.Command_error.t) Result.t -> unit
  }

type harness =
  { initial : W.Op.t list
  ; driver : Driver.t
  ; pages : P.t B.Expert.Var.t
  ; layout : Gpuio.Navigation.Pagination_layout.t B.Expert.Var.t
  ; active : bool B.Expert.Var.t
  ; callback : int B.Expert.Var.t
  ; latest : C.t option ref
  ; edits : request Queue.t
  ; navigations : (int * P.Request.t) Queue.t
  ; gap : Gpuio_protocol.Node_id.t * Gpuio_protocol.Handler_id.t
  }

let accept driver =
  match Driver.next_message driver with
  | Some (W.Message.Apply tx) ->
    Driver.submitted driver;
    Driver.acknowledge driver ~revision:tx.revision |> ok;
    tx.operations
  | None -> []
  | Some _ -> assert false
;;

let cycle t =
  Driver.cycle t.driver ~now:Time_ns.epoch |> ok;
  accept t.driver
;;

let current t = Option.value_exn !(t.latest)

let run t action =
  Driver.schedule t.driver action;
  cycle t
;;

let button operations text =
  List.find_map_exn operations ~f:(function
    | W.Op.Create (node, Button, label, Some handler) when String.equal label text ->
      Some (node, handler)
    | _ -> None)
;;

let create () =
  let pages = B.Expert.Var.create pages in
  let layout = B.Expert.Var.create Gpuio.Navigation.Pagination_layout.Full in
  let active = B.Expert.Var.create true
  and callback = B.Expert.Var.create 0 in
  let latest = ref None
  and edits = Queue.create ()
  and navigations = Queue.create () in
  let command expected command =
    assert (N.Command.equal command Commit);
    E.Expert.of_fun ~f:(fun ~callback ->
      Queue.enqueue edits { expected; complete = callback })
  in
  let component graph =
    let open B.Let_syntax in
    match%sub B.Expert.Var.value active with
    | false -> B.return (Gpuio_bonsai.View.column [])
    | true ->
      let on_request =
        B.map (B.Expert.Var.value callback) ~f:(fun version request ->
          E.of_thunk (fun () -> Queue.enqueue navigations (version, request)))
      in
      let pager =
        C.create
          command
          ~model:(B.Expert.Var.value pages)
          ~layout:(B.Expert.Var.value layout)
          ~on_request
          graph
      in
      let%arr pager = pager in
      latest := Some pager;
      C.view ~overlay pager |> ok
  in
  let driver =
    Driver.create window ~start:Time_ns.epoch ~theme:Gpuio.Theme.default component
  in
  Driver.cycle driver ~now:Time_ns.epoch |> ok;
  let initial = accept driver in
  let gap = button initial "…" in
  { initial; driver; pages; layout; active; callback; latest; edits; navigations; gap }
;;

let click t (node, handler) =
  Driver.dispatch
    t.driver
    (W.Event.Press (window, node, handler, Driver.revision t.driver));
  cycle t
;;

let open_gap t =
  let operations = click t t.gap in
  let node, config =
    List.find_map_exn operations ~f:(function
      | W.Op.Set_number_input (node, config, _) -> Some (node, config)
      | _ -> None)
  in
  let handler =
    List.find_map_exn operations ~f:(function
      | W.Op.Create (id, Number_input, _, Some handler)
        when Gpuio_protocol.Node_id.equal id node -> Some handler
      | _ -> None)
  in
  assert (
    List.count operations ~f:(function
      | W.Op.Create _ -> true
      | _ -> false)
    < 30);
  node, handler, config
;;

let wire config ?(composition = None) revision draft committed : NW.Snapshot.t =
  { revision
  ; domain = config.NW.Config.domain
  ; draft
  ; committed
  ; selection = { anchor = 0L; head = 0L }
  ; composition
  ; focused = true
  }
;;

let observe t (node, handler, config) ?composition revision draft committed =
  let snapshot = wire config ?composition revision draft committed in
  Driver.dispatch
    t.driver
    (W.Event.Number_input_event
       ( window
       , node
       , handler
       , Driver.revision t.driver
       , if Int64.equal revision 0L then NW.Event.Observed snapshot else Changed snapshot
       ));
  ignore (cycle t : W.Op.t list)
;;

let finish t request (node, _, config) ?composition revision draft committed =
  request.complete
    (Ok
       (N.Expert.snapshot_of_wire
          ~window
          ~node
          (wire config ?composition revision draft committed)
        |> ok));
  ignore (cycle t : W.Op.t list)
;;

let prepare () =
  let t = create () in
  let field = open_gap t in
  observe t field 0L "2" (Number 2.);
  t, field
;;

let%expect_test "bounded chooser coalesces confirmation and uses current callback" =
  let t, field = prepare () in
  let captured = current t in
  ignore (run t (E.Many [ C.confirm captured; C.confirm captured ]) : W.Op.t list);
  assert (Queue.length t.edits = 1 && C.is_confirming (current t));
  B.Expert.Var.set t.callback 7;
  ignore (cycle t : W.Op.t list);
  let request = Queue.dequeue_exn t.edits in
  finish t request field 1L "123456789" (Number 123456789.);
  assert (not (C.is_open (current t)));
  print_s [%sexp (Queue.to_list t.navigations : (int * P.Request.t) list)];
  ignore (run t (C.confirm captured) : W.Op.t list);
  assert (Queue.is_empty t.edits);
  [%expect {| ((7 (Page 123456789))) |}]
;;

let%expect_test "old dismissal and commit cannot affect a reopened chooser" =
  let t, first = prepare () in
  let old = current t in
  ignore (run t (C.confirm old) : W.Op.t list);
  let request = Queue.dequeue_exn t.edits in
  ignore (run t (C.cancel old) : W.Op.t list);
  let second = open_gap t in
  let a, _, _ = first
  and b, _, _ = second in
  assert (not (Gpuio_protocol.Node_id.equal a b));
  ignore (run t (C.cancel old) : W.Op.t list);
  finish t request first 1L "42" (Number 42.);
  assert (C.is_open (current t) && Queue.is_empty t.navigations);
  print_endline "fresh editor lifetime; old dismissal and completion ignored";
  [%expect {| fresh editor lifetime; old dismissal and completion ignored |}]
;;

let%expect_test "model, policy, layout and deactivation retire a pending opening" =
  List.iter [ "count"; "current"; "disabled"; "compact"; "inactive" ] ~f:(fun change ->
    let t, field = prepare () in
    ignore (run t (C.confirm (current t)) : W.Op.t list);
    let request = Queue.dequeue_exn t.edits in
    (match change with
     | "count" -> B.Expert.Var.set t.pages (P.with_total_pages pages 3 |> ok)
     | "current" -> B.Expert.Var.set t.pages (P.select pages ~page:5 |> ok)
     | "disabled" -> B.Expert.Var.set t.pages (P.with_disabled pages true)
     | "compact" -> B.Expert.Var.set t.layout Compact
     | "inactive" -> B.Expert.Var.set t.active false
     | _ -> assert false);
    ignore (cycle t : W.Op.t list);
    finish t request field 1L "42" (Number 42.);
    assert (Queue.is_empty t.navigations);
    B.Expert.Var.set t.active true;
    ignore (cycle t : W.Op.t list);
    assert (not (C.is_open (current t))));
  print_endline "all five transitions suppress late navigation; reactivation stays closed";
  [%expect {| all five transitions suppress late navigation; reactivation stays closed |}]
;;

let%expect_test "composition, command failure and edits after reply preserve draft" =
  let t, field = prepare () in
  ignore (run t (C.confirm (current t)) : W.Op.t list);
  let first = Queue.dequeue_exn t.edits in
  first.complete (Error Composing);
  ignore (cycle t : W.Op.t list);
  assert (Option.equal N.Command_error.equal (C.error (current t)) (Some Composing));
  observe t field 2L "15" (Number 2.);
  ignore (run t (C.confirm (current t)) : W.Op.t list);
  let second = Queue.dequeue_exn t.edits in
  observe t field 4L "16" (Number 15.);
  finish t second field 3L "15" (Number 15.);
  assert (Option.equal N.Command_error.equal (C.error (current t)) (Some Stale_revision));
  assert (C.is_open (current t) && Queue.is_empty t.navigations);
  ignore (run t (C.confirm (current t)) : W.Op.t list);
  let third = Queue.dequeue_exn t.edits in
  finish t third field 5L "16" (Number 16.);
  print_s [%sexp (Queue.to_list t.navigations : (int * P.Request.t) list)];
  [%expect {| ((0 (Page 16))) |}]
;;

let%expect_test "public pagination transaction sequence" =
  let t = create () in
  let opened = click t t.gap in
  let closed = run t (C.cancel (current t)) in
  let gaps =
    List.filter_map t.initial ~f:(function
      | W.Op.Create (node, Button, "…", Some handler) -> Some (node, handler)
      | _ -> None)
  in
  let second_opened = click t (List.nth_exn gaps 1) in
  let old = current t in
  let first_reopened = click t t.gap in
  let old_cancel = run t (C.cancel old) in
  assert (List.is_empty old_cancel && C.is_open (current t));
  let final_closed = run t (C.cancel (current t)) in
  List.iteri
    [ t.initial; opened; closed; second_opened; first_reopened; final_closed ]
    ~f:(fun index operations ->
      let base = Int64.of_int index in
      let message =
        W.Message.Apply { window; base; revision = Int64.succ base; operations }
      in
      let bytes =
        Bin_prot.Utils.bin_dump W.Message.bin_writer_t message |> Bigstring.to_string
      in
      let hex =
        String.to_list bytes
        |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
        |> String.concat
      in
      let fixture =
        Eio_main.run (fun env ->
          Eio.Path.load
            Eio.Path.(Eio.Stdenv.fs env / sprintf "pagination-view-%d.hex" index)
          |> String.strip)
      in
      assert (String.equal hex fixture));
  [%expect {| |}]
;;

let%expect_test "shortcut selection wins over an outstanding numeric confirmation" =
  let t, field = prepare () in
  ignore (run t (C.confirm (current t)) : W.Op.t list);
  let request = Queue.dequeue_exn t.edits in
  let rec find_shortcut view =
    let d = Gpuio.View.Expert.describe view in
    if String.equal d.text "2"
    then d.on_click
    else List.find_map d.children ~f:find_shortcut
  in
  let activate = C.view ~overlay (current t) |> ok |> find_shortcut |> Option.value_exn in
  ignore (run t (activate ()) : W.Op.t list);
  finish t request field 1L "42" (Number 42.);
  assert (not (C.is_open (current t)));
  print_s [%sexp (Queue.to_list t.navigations : (int * P.Request.t) list)];
  [%expect {| ((0 (Page 2))) |}]
;;
