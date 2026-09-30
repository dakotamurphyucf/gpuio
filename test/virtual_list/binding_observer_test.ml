open Core
module B = Bonsai.Cont
module Binding = Gpuio.Command_binding
module View = Gpuio_bonsai.View

let ok = Or_error.ok_exn
let command = Gpuio.Command.Id.of_string "run" |> ok
let here = Binding.Config.create ~context:Binding.Context.here [ Command command ] |> ok
let focused = Binding.Config.create [ Command command ] |> ok

let sample config epoch =
  Binding.Expert.of_wire
    config
    { Gpuio_protocol.Command_binding_wire.Observation.epoch
    ; state = Ready [ Missing_command ]
    }
  |> Option.value_exn
;;

let run ~optimize =
  let config = B.Expert.Var.create here in
  let active = B.Expert.Var.create true in
  let component graph =
    let open B.Let_syntax in
    match%sub B.Expert.Var.value active with
    | false -> B.return (View.text "inactive")
    | true ->
      Gpuio_bonsai.Command_binding.component
        ~config:(B.Expert.Var.value config)
        ~f:(fun observation graph ->
          let count, set_count = B.state 0 ~equal:Int.equal graph in
          let%arr observation = observation
          and count = count
          and set_count = set_count in
          [ View.text
              (Option.value_map observation ~default:"pending" ~f:(fun observation ->
                 Binding.Observation.epoch observation |> Int64.to_string))
          ; View.button ~on_click:(set_count (count + 1)) (Int.to_string count)
          ])
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
  let status view =
    let children = (Gpuio.View.Expert.describe view).children in
    List.map children ~f:(fun child -> (Gpuio.View.Expert.describe child).text)
  in
  let callback view =
    (Gpuio.View.Expert.describe view).command_binding_scope
    |> Option.value_exn
    |> fun (scope : _ Gpuio.View.Expert.command_binding_scope) -> scope.on_update
  in
  let initial = cycle () in
  assert (List.equal String.equal (status initial) [ "pending"; "0" ]);
  let old_here = callback initial in
  let button = List.nth_exn (Gpuio.View.Expert.describe initial).children 1 in
  let click = (Gpuio.View.Expert.describe button).on_click |> Option.value_exn in
  Bonsai_driver.schedule_event driver (click ());
  Bonsai_driver.schedule_event driver (old_here (sample here 1L));
  assert (List.equal String.equal (status (cycle ())) [ "1"; "1" ]);
  B.Expert.Var.set config focused;
  let next = cycle () in
  assert (List.equal String.equal (status next) [ "pending"; "1" ]);
  Bonsai_driver.schedule_event driver (old_here (sample here 2L));
  assert (List.equal String.equal (status (cycle ())) [ "pending"; "1" ]);
  let old_focused = callback next in
  Bonsai_driver.schedule_event driver (old_focused (sample focused 3L));
  assert (List.equal String.equal (status (cycle ())) [ "3"; "1" ]);
  B.Expert.Var.set config here;
  let returned = cycle () in
  assert (List.equal String.equal (status returned) [ "pending"; "1" ]);
  Bonsai_driver.schedule_event driver (old_here (sample here 4L));
  Bonsai_driver.schedule_event driver (old_focused (sample focused 4L));
  assert (List.equal String.equal (status (cycle ())) [ "pending"; "1" ]);
  let fresh = callback returned in
  Bonsai_driver.schedule_event driver (fresh (sample here 5L));
  assert (List.equal String.equal (status (cycle ())) [ "5"; "1" ]);
  B.Expert.Var.set active false;
  ignore (cycle () : View.t);
  Bonsai_driver.schedule_event driver (fresh (sample here 6L));
  ignore (cycle () : View.t);
  B.Expert.Var.set active true;
  assert (List.equal String.equal (status (cycle ())) [ "pending"; "1" ]);
  Bonsai_driver.schedule_event driver (fresh (sample here 7L));
  assert (List.equal String.equal (status (cycle ())) [ "pending"; "1" ]);
  Bonsai_driver.Expert.invalidate_observers driver;
  print_s
    [%sexp
      (optimize : bool)
    , ("pending per visit; stale effects fenced; child state retained" : string)]
;;

let%expect_test "binding observation belongs to the active configuration visit" =
  run ~optimize:false;
  run ~optimize:true;
  [%expect
    {|
    (false "pending per visit; stale effects fenced; child state retained")
    (true "pending per visit; stale effects fenced; child state retained")
    |}]
;;
