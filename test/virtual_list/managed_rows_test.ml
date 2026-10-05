open Core
module B = Bonsai.Cont
module E = Bonsai.Effect
module Rows = Gpuio_bonsai.Managed_rows

let run ~optimize =
  let visible = B.Expert.Var.create (Int.Map.singleton 1 ()) in
  let activations = ref 0 in
  let deactivations = ref 0 in
  let component graph =
    Rows.assoc
      (module Int)
      (B.Expert.Var.value visible)
      ~f:(fun _ _ lifetime graph ->
        let count, bump =
          B.state_machine0
            ~default_model:0
            ~sexp_of_model:Int.sexp_of_t
            ~apply_action:(fun _ count () -> count + 1)
            graph
        in
        B.Edge.lifecycle
          ~on_activate:(B.return (E.of_thunk (fun () -> Int.incr activations)))
          ~on_deactivate:(B.return (E.of_thunk (fun () -> Int.incr deactivations)))
          graph;
        let open B.Let_syntax in
        let%arr count = count
        and bump = bump
        and lifetime = lifetime in
        count, Rows.Lifetime.guard lifetime (bump ()))
      graph
  in
  let clock = Bonsai.Time_source.create ~start:Time_ns.epoch in
  let driver = Bonsai_driver.create ~clock ~optimize component in
  let cycle () =
    Bonsai_driver.flush driver;
    Bonsai_driver.trigger_lifecycles driver;
    Bonsai_driver.flush driver;
    Bonsai_driver.result driver
  in
  let model_is_empty () =
    Sexp.equal (Bonsai_driver.Expert.sexp_of_model driver) (Sexp.List [])
  in
  let _, old_bump = Map.find_exn (cycle ()) 1 in
  Bonsai_driver.schedule_event driver old_bump;
  let count, _ = Map.find_exn (cycle ()) 1 in
  assert (count = 1);
  B.Expert.Var.set visible Int.Map.empty;
  assert (Map.is_empty (cycle ()));
  assert (model_is_empty ());
  Bonsai_driver.schedule_event driver old_bump;
  ignore (cycle () : (int * unit E.t) Int.Map.t);
  assert (model_is_empty ());
  B.Expert.Var.set visible (Int.Map.singleton 1 ());
  let count, new_bump = Map.find_exn (cycle ()) 1 in
  assert (count = 0);
  Bonsai_driver.schedule_event driver old_bump;
  let count, _ = Map.find_exn (cycle ()) 1 in
  assert (count = 0);
  Bonsai_driver.schedule_event driver new_bump;
  let count, _ = Map.find_exn (cycle ()) 1 in
  assert (count = 1);
  for key = 2 to 1000 do
    B.Expert.Var.set visible (Int.Map.singleton key ());
    let count, bump = Map.find_exn (cycle ()) key in
    assert (count = 0);
    Bonsai_driver.schedule_event driver bump;
    let count, _ = Map.find_exn (cycle ()) key in
    assert (count = 1)
  done;
  B.Expert.Var.set visible Int.Map.empty;
  ignore (cycle () : (int * unit E.t) Int.Map.t);
  assert (model_is_empty ());
  assert (!activations = !deactivations);
  print_s
    [%sexp
      (optimize : bool)
    , (!activations : int)
    , (!deactivations : int)
    , (model_is_empty () : bool)];
  Bonsai_driver.Expert.invalidate_observers driver
;;

let%expect_test "eviction resets models and guarded effects cannot revive old visits" =
  run ~optimize:false;
  run ~optimize:true;
  [%expect
    {|
    (false 1001 1001 true)
    (true 1001 1001 true)
    |}]
;;

let%expect_test "row activation and deactivation effects compose with the reset" =
  List.iter [ false; true ] ~f:(fun optimize ->
    let visible = B.Expert.Var.create (Int.Map.singleton 1 ()) in
    let component graph =
      Rows.assoc
        (module Int)
        (B.Expert.Var.value visible)
        ~f:(fun _ _ lifetime graph ->
          let count, bump =
            B.state_machine0
              ~default_model:0
              ~sexp_of_model:Int.sexp_of_t
              ~apply_action:(fun _ count () -> count + 1)
              graph
          in
          let open B.Let_syntax in
          let on_activate =
            let%arr lifetime = lifetime
            and bump = bump in
            Rows.Lifetime.guard lifetime (bump ())
          in
          let on_deactivate =
            let%arr bump = bump in
            bump ()
          in
          B.Edge.lifecycle ~on_activate ~on_deactivate graph;
          count)
        graph
    in
    let clock = Bonsai.Time_source.create ~start:Time_ns.epoch in
    let driver = Bonsai_driver.create ~clock ~optimize component in
    Bonsai_driver.trigger_lifecycles driver;
    Bonsai_driver.flush driver;
    let activated = Map.find_exn (Bonsai_driver.result driver) 1 in
    B.Expert.Var.set visible Int.Map.empty;
    Bonsai_driver.flush driver;
    Bonsai_driver.trigger_lifecycles driver;
    Bonsai_driver.flush driver;
    print_s
      [%sexp
        (optimize : bool)
      , (activated : int)
      , (Bonsai_driver.Expert.sexp_of_model driver : Sexp.t)];
    Bonsai_driver.Expert.invalidate_observers driver);
  [%expect
    {|
    (false 1 ())
    (true 1 ())
    |}]
;;

let%expect_test "nested lifetimes survive data updates but never revive after remount" =
  List.iter [ false; true ] ~f:(fun optimize ->
    let members = B.Expert.Var.create (Int.Map.of_alist_exn [ 1, 0; 2, 0 ]) in
    let columns = B.Expert.Var.create (Int.Map.singleton 0 ()) in
    let calls = ref [] in
    let component graph =
      Rows.assoc
        (module Int)
        (B.Expert.Var.value members)
        ~f:(fun row data _ graph ->
          Rows.assoc
            (module Int)
            (B.Expert.Var.value columns)
            ~f:(fun _ _ lifetime _graph ->
              let open B.Let_syntax in
              let%arr row = row
              and data = data
              and lifetime = lifetime in
              ( lifetime
              , Rows.Lifetime.guard
                  lifetime
                  (E.of_thunk (fun () -> calls := (row, data) :: !calls)) ))
            graph)
        graph
    in
    let driver =
      Bonsai_driver.create
        ~optimize
        ~clock:(Bonsai.Time_source.create ~start:Time_ns.epoch)
        component
    in
    let cycle () =
      Bonsai_driver.flush driver;
      Bonsai_driver.trigger_lifecycles driver;
      Bonsai_driver.flush driver;
      Bonsai_driver.result driver
    in
    let cell rows row = Map.find_exn (Map.find_exn rows row) 0 in
    let fire (_, action) =
      Bonsai_driver.schedule_event driver action;
      ignore (cycle () : _ Int.Map.t)
    in
    let initial = cycle () in
    let first = cell initial 1
    and second = cell initial 2 in
    assert (not (phys_equal (fst first) (fst second)));
    B.Expert.Var.set members (Int.Map.of_alist_exn [ 1, 10; 2, 0 ]);
    let updated = cycle () in
    assert (phys_equal (fst first) (fst (cell updated 1)));
    assert (phys_equal (fst second) (fst (cell updated 2)));
    fire (cell updated 1);
    B.Expert.Var.set members (Int.Map.singleton 2 0);
    let retained = cycle () in
    assert (phys_equal (fst second) (fst (cell retained 2)));
    fire first;
    fire second;
    B.Expert.Var.set members (Int.Map.of_alist_exn [ 1, 20; 2, 0 ]);
    let revisited = cycle () in
    let fresh = cell revisited 1 in
    assert (not (phys_equal (fst first) (fst fresh)));
    fire first;
    fire fresh;
    B.Expert.Var.set columns Int.Map.empty;
    ignore (cycle () : _ Int.Map.t);
    fire fresh;
    fire second;
    B.Expert.Var.set columns (Int.Map.singleton 0 ());
    let reinserted = cycle () in
    assert (not (phys_equal (fst fresh) (fst (cell reinserted 1))));
    assert (not (phys_equal (fst second) (fst (cell reinserted 2))));
    fire fresh;
    fire second;
    fire (cell reinserted 1);
    fire (cell reinserted 2);
    print_s [%sexp (optimize : bool), (List.rev !calls : (int * int) list)];
    B.Expert.Var.set members Int.Map.empty;
    ignore (cycle () : _ Int.Map.t);
    Bonsai_driver.Expert.invalidate_observers driver);
  [%expect
    {|
    (false ((1 10) (2 0) (1 20) (1 20) (2 0)))
    (true ((1 10) (2 0) (1 20) (1 20) (2 0)))
    |}]
;;
