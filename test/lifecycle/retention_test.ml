open Core
module B = Bonsai.Cont
module E = Bonsai.Effect

let probe ~optimize ~custom_reset =
  let keys = B.Expert.Var.create (Int.Map.singleton 1 ()) in
  let component graph =
    B.assoc
      (module Int)
      (B.Expert.Var.value keys)
      ~f:(fun _ _ graph ->
        let row, reset =
          B.with_model_resetter
            ~f:(fun graph ->
              let count, bump =
                B.state_machine0
                  ~default_model:0
                  ~reset:(fun _ model -> if custom_reset then model else 0)
                  ~apply_action:(fun _ model () -> model + 1)
                  graph
              in
              let open B.Let_syntax in
              let%arr count = count
              and bump = bump in
              count, bump)
            graph
        in
        B.Edge.lifecycle ~on_deactivate:reset graph;
        row)
      graph
  in
  let clock = Bonsai.Time_source.create ~start:Time_ns.epoch in
  let driver = Bonsai_driver.create ~optimize ~clock component in
  let cycle () =
    Bonsai_driver.flush driver;
    Bonsai_driver.trigger_lifecycles driver;
    Bonsai_driver.flush driver;
    Bonsai_driver.result driver
  in
  let _, bump = Map.find_exn (cycle ()) 1 in
  Bonsai_driver.schedule_event driver (bump ());
  let count, _ = Map.find_exn (cycle ()) 1 in
  assert (count = 1);
  B.Expert.Var.set keys Int.Map.empty;
  ignore (cycle () : (int * (unit -> unit E.t)) Int.Map.t);
  let after_reset = Bonsai_driver.Expert.sexp_of_model driver in
  Bonsai_driver.schedule_event driver (bump ());
  ignore (cycle () : (int * (unit -> unit E.t)) Int.Map.t);
  let after_late_action = Bonsai_driver.Expert.sexp_of_model driver in
  B.Expert.Var.set keys (Int.Map.singleton 1 ());
  let revisited, _ = Map.find_exn (cycle ()) 1 in
  print_s
    [%sexp
      (optimize : bool)
    , (custom_reset : bool)
    , (after_reset : Sexp.t)
    , (after_late_action : Sexp.t)
    , (revisited : int)];
  Bonsai_driver.Expert.invalidate_observers driver
;;

let%expect_test "reset and late effects are separate retention policies" =
  List.iter [ false; true ] ~f:(fun optimize ->
    List.iter [ false; true ] ~f:(fun custom_reset -> probe ~optimize ~custom_reset));
  [%expect
    {|
    (false false () ((1 <opaque>)) 1)
    (false true ((1 <opaque>)) ((1 <opaque>)) 2)
    (true false () ((1 <opaque>)) 1)
    (true true ((1 <opaque>)) ((1 <opaque>)) 2)
    |}]
;;
