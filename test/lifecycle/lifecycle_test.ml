open Core
module B = Bonsai.Cont
module E = Bonsai.Effect

let run ~optimize () =
  let events = ref [] in
  let record name key = E.of_thunk (fun () -> events := !events @ [ name, key ]) in
  let keys = B.Expert.Var.create (Int.Map.of_alist_exn [ 1, (); 2, () ]) in
  let component graph =
    let open B.Let_syntax in
    let rows =
      B.assoc
        (module Int)
        (B.Expert.Var.value keys)
        ~f:(fun key _ graph ->
          let count, bump =
            B.state_machine0
              ~default_model:0
              ~apply_action:(fun _ model () -> model + 1)
              graph
          in
          let activated, set_activated = B.state false graph in
          let on_activate =
            let%arr key = key
            and set_activated = set_activated in
            E.Many [ record "activate" key; set_activated true ]
          in
          let on_deactivate =
            let%arr key = key in
            record "deactivate" key
          in
          let after_display =
            let%arr key = key in
            record "display" key
          in
          B.Edge.lifecycle ~on_activate ~on_deactivate ~after_display graph;
          let%arr count = count
          and bump = bump
          and activated = activated in
          count, bump, activated)
        graph
    in
    (* The visible output is deliberately unchanged by membership or state. *)
    let%arr rows = rows in
    "unchanged native view", rows
  in
  let clock = Bonsai.Time_source.create ~start:Time_ns.epoch in
  let driver = Bonsai_driver.create ~optimize ~clock component in
  let cycle () =
    Bonsai_driver.flush driver;
    let view, rows = Bonsai_driver.result driver in
    assert (String.equal view "unchanged native view");
    Bonsai_driver.trigger_lifecycles driver;
    rows
  in
  let count_event name = List.count !events ~f:(fun (n, _) -> String.equal n name) in
  let rows = cycle () in
  assert (count_event "activate" = 2);
  assert (count_event "display" = 2);
  let _, _, activated = Map.find_exn rows 1 in
  assert (not activated);
  let rows = cycle () in
  let _, bump, activated = Map.find_exn rows 1 in
  assert activated;
  assert (count_event "activate" = 2);
  Bonsai_driver.schedule_event driver (bump ());
  let rows = cycle () in
  let count, _, _ = Map.find_exn rows 1 in
  assert (count = 1);
  B.Expert.Var.set keys (Int.Map.singleton 2 ());
  ignore (cycle ());
  assert (count_event "deactivate" = 1);
  B.Expert.Var.set keys (Int.Map.of_alist_exn [ 2, (); 1, () ]);
  let rows = cycle () in
  let count, _, _ = Map.find_exn rows 1 in
  assert (count = 1);
  assert (count_event "activate" = 3);
  events := [];
  B.Expert.Var.set keys (Int.Map.singleton 3 ());
  ignore (cycle ());
  assert (
    List.equal
      (fun (a, b) (c, d) -> String.equal a c && Int.equal b d)
      !events
      [ "deactivate", 1; "deactivate", 2; "activate", 3; "display", 3 ]);
  for key = 4 to 23 do
    B.Expert.Var.set keys (Int.Map.singleton key ());
    ignore (cycle ());
    Gc.full_major ()
  done;
  B.Expert.Var.set keys Int.Map.empty;
  assert (Map.is_empty (cycle ()));
  Bonsai_driver.Expert.invalidate_observers driver;
  printf
    "LIFECYCLE_PASS optimize=%b state-retention reactivation unchanged-view effect-flush \
     ordering cleanup gc\n\
     %!"
    optimize
;;

let%expect_test "keyed lifecycle scheduling with and without optimization" =
  run ~optimize:true ();
  run ~optimize:false ();
  [%expect
    {| 
    LIFECYCLE_PASS optimize=true state-retention reactivation unchanged-view effect-flush ordering cleanup gc
    LIFECYCLE_PASS optimize=false state-retention reactivation unchanged-view effect-flush ordering cleanup gc
  |}]
;;

let%expect_test "driver can be owned by a dedicated OCaml domain" =
  Domain.join (Domain.spawn (fun () -> run ~optimize:true ()));
  [%expect
    {| LIFECYCLE_PASS optimize=true state-retention reactivation unchanged-view effect-flush ordering cleanup gc |}]
;;
