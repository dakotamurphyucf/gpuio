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
