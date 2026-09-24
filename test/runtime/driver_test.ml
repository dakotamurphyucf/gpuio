open Core
open Gpuio_protocol
module B = Bonsai.Cont
module E = Bonsai.Effect
module Driver = Gpuio_runtime_core.Window_driver

let window = Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn
let now seconds = Time_ns.add Time_ns.epoch (Time_ns.Span.of_sec seconds)
let cycle t seconds = Driver.cycle t ~now:(now seconds) |> Or_error.ok_exn

let accept t =
  match Driver.next_message t with
  | None -> None
  | Some (Wire.Message.Apply tx) ->
    Driver.submitted t;
    Driver.acknowledge t ~revision:tx.revision |> Or_error.ok_exn;
    Some tx
  | Some _ -> assert false
;;

let create component =
  Driver.create window ~start:Time_ns.epoch ~theme:Gpuio.Theme.default component
;;

let%expect_test "controlled toggles preserve two activations before Bonsai stabilizes" =
  let driver =
    create (fun graph ->
      let checked, toggle = B.toggle ~default_model:false graph in
      let open B.Let_syntax in
      let%arr checked = checked
      and toggle = toggle in
      Gpuio.View.switch ~checked ~on_toggle:(fun () -> toggle) "Stream")
  in
  cycle driver 0.;
  let tx = accept driver |> Option.value_exn in
  let node, handler =
    List.find_map_exn tx.operations ~f:(function
      | Create (node, Switch, _, Some handler) -> Some (node, handler)
      | _ -> None)
  in
  let activate () = Driver.dispatch driver (Press (window, node, handler, 1L)) in
  activate ();
  activate ();
  cycle driver 0.;
  assert (Option.is_none (Driver.next_message driver));
  activate ();
  cycle driver 0.;
  let tx = accept driver |> Option.value_exn in
  print_s [%sexp (tx.operations : Wire.Op.t list)];
  Driver.close driver;
  [%expect {| ((Set_control ((slot 0) (generation 1)) (Switch true false))) |}]
;;

let%expect_test
    "native acceptance gates activation; activation actions settle and idle cycles emit \
     nothing"
  =
  let events = ref [] in
  let component graph =
    let state, set_state = B.state false graph in
    let open B.Let_syntax in
    let on_activate =
      let%arr set_state = set_state in
      E.Many [ E.of_thunk (fun () -> events := "activate" :: !events); set_state true ]
    in
    B.Edge.lifecycle
      ~on_activate
      ~on_deactivate:(B.return (E.of_thunk (fun () -> events := "deactivate" :: !events)))
      graph;
    let%arr state = state in
    Gpuio.View.text (if state then "ready" else "initial")
  in
  let driver = create component in
  cycle driver 0.;
  assert (List.is_empty !events);
  ignore (accept driver : Wire.Transaction.t option);
  assert (List.equal String.equal !events [ "activate" ]);
  cycle driver 0.;
  let tx = accept driver |> Option.value_exn in
  assert (
    List.exists tx.operations ~f:(function
      | Set_text (_, "ready") -> true
      | _ -> false));
  for i = 1 to 100 do
    cycle driver (Float.of_int i);
    assert (Option.is_none (Driver.next_message driver))
  done;
  Driver.close driver;
  Driver.close driver;
  print_s [%sexp (List.rev !events : string list)];
  [%expect {| (activate deactivate) |}]
;;

let%expect_test "Bonsai sleeps and incremental deadlines fire without a custom clock" =
  let events = ref [] in
  let component graph =
    let open B.Let_syntax in
    let sleep = B.Clock.sleep graph in
    let deadline = B.Clock.at (B.return (now 2.)) graph in
    let on_activate =
      let%arr sleep = sleep in
      let open E.Let_syntax in
      let%bind () = sleep (Time_ns.Span.of_sec 1.) in
      E.of_thunk (fun () -> events := "slept" :: !events)
    in
    B.Edge.lifecycle ~on_activate graph;
    let%arr deadline = deadline in
    Gpuio.View.text
      (match deadline with
       | Before -> "before"
       | After -> "after")
  in
  let driver = create component in
  cycle driver 0.;
  ignore (accept driver : Wire.Transaction.t option);
  cycle driver 0.5;
  assert (List.is_empty !events);
  assert (Option.is_none (Driver.next_message driver));
  cycle driver 1.1;
  assert (List.equal String.equal !events [ "slept" ]);
  cycle driver 2.1;
  let tx = accept driver |> Option.value_exn in
  assert (
    List.exists tx.operations ~f:(function
      | Set_text (_, "after") -> true
      | _ -> false));
  Driver.close driver;
  print_endline "CLOCK_PASS";
  [%expect {| CLOCK_PASS |}]
;;

let%expect_test
    "no-diff lifecycle transitions and current callbacks; stale domain access fails"
  =
  let events = ref [] in
  let selected = B.Expert.Var.create 0 in
  let component graph =
    let open B.Let_syntax in
    let selected = B.Expert.Var.value selected in
    let (_ : unit B.t) =
      match%sub selected with
      | 0 ->
        B.Edge.lifecycle
          ~on_activate:(B.return (E.of_thunk (fun () -> events := "a+" :: !events)))
          ~on_deactivate:(B.return (E.of_thunk (fun () -> events := "a-" :: !events)))
          graph;
        B.return ()
      | _ ->
        B.Edge.lifecycle
          ~on_activate:(B.return (E.of_thunk (fun () -> events := "b+" :: !events)))
          ~on_deactivate:(B.return (E.of_thunk (fun () -> events := "b-" :: !events)))
          graph;
        B.return ()
    in
    let%arr selected = selected in
    Gpuio.View.button
      ~on_click:(fun () ->
        E.of_thunk (fun () -> events := Int.to_string selected :: !events))
      "same"
  in
  let driver = create component in
  cycle driver 0.;
  let tx = accept driver |> Option.value_exn in
  let node, handler =
    List.find_map_exn tx.operations ~f:(function
      | Create (node, Button, _, Some handler) -> Some (node, handler)
      | _ -> None)
  in
  B.Expert.Var.set selected 1;
  cycle driver 0.;
  assert (Option.is_none (Driver.next_message driver));
  Driver.dispatch driver (Press (window, node, handler, 1L));
  let wrong_domain =
    Domain.spawn (fun () -> Result.try_with (fun () -> Driver.revision driver))
    |> Domain.join
  in
  assert (Result.is_error wrong_domain);
  Driver.close driver;
  print_s [%sexp (List.rev !events : string list)];
  [%expect {| (a+ a- b+ 1 b-) |}]
;;

let%expect_test
    "pending native commits retain their base and close prevents late dispatch"
  =
  let fired = ref false in
  let driver =
    create (fun _ ->
      B.return
        (Gpuio.View.button
           ~on_click:(fun () -> E.of_thunk (fun () -> fired := true))
           "same"))
  in
  cycle driver 0.;
  let tx =
    match Driver.next_message driver with
    | Some (Apply tx) -> tx
    | _ -> assert false
  in
  Driver.submitted driver;
  cycle driver 1.;
  assert (Option.is_none (Driver.next_message driver));
  assert (Result.is_error (Driver.acknowledge driver ~revision:2L));
  Driver.acknowledge driver ~revision:1L |> Or_error.ok_exn;
  assert (Result.is_error (Driver.acknowledge driver ~revision:1L));
  let node, handler =
    List.find_map_exn tx.operations ~f:(function
      | Create (node, Button, _, Some handler) -> Some (node, handler)
      | _ -> None)
  in
  Driver.close driver;
  Driver.dispatch driver (Press (window, node, handler, 1L));
  assert (not !fired);
  assert (Result.is_error (Driver.cycle driver ~now:(now 2.)));
  print_endline "COMMIT_LIFETIME_PASS";
  [%expect {| COMMIT_LIFETIME_PASS |}]
;;

let%expect_test "activation sleep starts from acknowledgment-time clock sample" =
  let fired = ref false in
  let driver =
    create (fun graph ->
      let open B.Let_syntax in
      let sleep = B.Clock.sleep graph in
      let on_activate =
        let%arr sleep = sleep in
        let open E.Let_syntax in
        let%bind () = sleep (Time_ns.Span.of_sec 1.) in
        E.of_thunk (fun () -> fired := true)
      in
      B.Edge.lifecycle ~on_activate graph;
      B.return (Gpuio.View.text "same"))
  in
  cycle driver 0.;
  Driver.advance_clock driver ~now:(now 10.);
  ignore (accept driver : Wire.Transaction.t option);
  cycle driver 10.5;
  assert (not !fired);
  cycle driver 11.1;
  assert !fired;
  Driver.close driver;
  print_endline "ACK_CLOCK_PASS";
  [%expect {| ACK_CLOCK_PASS |}]
;;

let%expect_test
    "native row retention retries without committing deactivation or resetting a model"
  =
  let module Rows = Gpuio_bonsai.Managed_rows in
  let desired = B.Expert.Var.create true in
  let activations = ref 0 in
  let deactivations = ref 0 in
  let key = Gpuio.Key.of_string_exn "row" in
  let order = Gpuio.Virtual_list.Order.create [ key ] |> Or_error.ok_exn in
  let config =
    Gpuio.Virtual_list.Config.create ~height:(Estimated 80.) () |> Or_error.ok_exn
  in
  let component graph =
    let open B.Let_syntax in
    let active =
      let%arr active = B.Expert.Var.value desired in
      if active then Int.Map.singleton 1 () else Int.Map.empty
    in
    let rows =
      Rows.assoc
        (module Int)
        active
        ~f:(fun _ _ _ graph ->
          let count, bump =
            B.state_machine0
              ~default_model:0
              ~apply_action:(fun _ count () -> count + 1)
              graph
          in
          B.Edge.lifecycle
            ~on_activate:(B.return (E.of_thunk (fun () -> Int.incr activations)))
            ~on_deactivate:(B.return (E.of_thunk (fun () -> Int.incr deactivations)))
            graph;
          let%arr count = count
          and bump = bump in
          Gpuio.View.button ~on_click:bump (Int.to_string count))
        graph
    in
    let%arr rows = rows in
    Gpuio.View.Expert.managed_virtual_list
      ~config
      ~order
      ~on_viewport:(fun _ -> E.Ignore)
      ~on_retain:(fun keys ->
        E.of_thunk (fun () ->
          assert (List.equal Gpuio.Key.equal keys [ key ]);
          B.Expert.Var.set desired true))
      (Map.data rows |> List.map ~f:(fun row -> key, row))
    |> Or_error.ok_exn
  in
  let driver = create component in
  cycle driver 0.;
  let first = accept driver |> Option.value_exn in
  let list_node =
    List.find_map_exn first.operations ~f:(function
      | Create (node, Virtual_list, _, _) -> Some node
      | _ -> None)
  in
  let button, handler =
    List.find_map_exn first.operations ~f:(function
      | Create (node, Button, _, Some handler) -> Some (node, handler)
      | _ -> None)
  in
  Driver.dispatch driver (Press (window, button, handler, first.revision));
  cycle driver 0.;
  ignore (accept driver : Wire.Transaction.t option);
  assert (!activations = 1 && !deactivations = 0);
  B.Expert.Var.set desired false;
  cycle driver 0.;
  let eviction =
    match Driver.next_message driver with
    | Some (Apply tx) -> tx
    | _ -> assert false
  in
  Driver.submitted driver;
  let pins = [ { Gpuio_protocol.List_wire.Retained.node = list_node; rows = [ 1L ] } ] in
  assert (
    Or_error.is_error
      (Driver.retry_list_rows driver ~revision:Int64.(eviction.revision + 1L) pins));
  Driver.retry_list_rows driver ~revision:eviction.revision pins |> Or_error.ok_exn;
  assert (!deactivations = 0);
  cycle driver 0.;
  assert (Option.is_none (Driver.next_message driver));
  assert (!activations = 1 && !deactivations = 0);
  Driver.dispatch driver (Press (window, button, handler, first.revision));
  cycle driver 0.;
  let updated = accept driver |> Option.value_exn in
  assert (
    List.exists updated.operations ~f:(function
      | Set_text (_, "2") -> true
      | _ -> false));
  B.Expert.Var.set desired false;
  cycle driver 0.;
  ignore (accept driver : Wire.Transaction.t option);
  assert (!deactivations = 1);
  Driver.close driver;
  [%expect {| |}]
;;

let%expect_test
    "an in-flight native commit freezes the lifecycle snapshot until acceptance"
  =
  let source = B.Expert.Var.create 0 in
  let events = ref [] in
  let component graph =
    let open B.Let_syntax in
    let source = B.Expert.Var.value source in
    let after_display =
      let%arr value = source in
      E.of_thunk (fun () -> events := value :: !events)
    in
    B.Edge.after_display after_display graph;
    let%arr value = source in
    Gpuio.View.text (Int.to_string value)
  in
  let driver = create component in
  cycle driver 0.;
  Driver.submitted driver;
  B.Expert.Var.set source 1;
  cycle driver 1.;
  let other = create (fun _ -> B.return (Gpuio.View.text "another window")) in
  cycle other 1.;
  Driver.close other;
  Driver.acknowledge driver ~revision:1L |> Or_error.ok_exn;
  assert (List.equal Int.equal !events [ 0 ]);
  cycle driver 0.;
  ignore (accept driver : Wire.Transaction.t option);
  assert (List.equal Int.equal !events [ 1; 0 ]);
  Driver.close driver;
  [%expect {| |}]
;;
