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
