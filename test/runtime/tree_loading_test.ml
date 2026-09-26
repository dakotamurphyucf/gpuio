open Core
module T = Gpuio.Tree
module P = Gpuio_eio.Tree_loading
module Scope = Gpuio_eio.Scope
module Inbox = Gpuio_eio.Inbox

let ok = Or_error.ok_exn
let id text = T.Id.of_string text |> ok

let forest count =
  let nodes =
    List.init count ~f:(fun i ->
      let name = Int.to_string i in
      ( id name
      , T.Node.create ~label:name ~children:(Branch { ids = []; next = More None }) ()
        |> ok ))
  in
  T.create ~roots:(List.map nodes ~f:fst) nodes |> ok
;;

let page name =
  { P.Page.roots = [ id name ]
  ; nodes = [ id name, T.Node.create ~label:name ~children:Leaf () |> ok ]
  ; next = End
  }
;;

let drain inbox = List.iter (Inbox.take_turn inbox) ~f:(fun f -> f ())

let settle inbox =
  for _ = 1 to 10 do
    Eio.Fiber.yield ();
    drain inbox
  done
;;

let tree t = P.Snapshot.tree (P.snapshot t)
let status t name = P.Snapshot.status (P.snapshot t) (id name)
let request t name = P.request t (id name) |> ok

let with_scope ?(capacity = 16) ?(max_tasks = 16) f =
  Eio_mock.Backend.run (fun () ->
    Eio.Switch.run (fun sw ->
      let inbox = Inbox.create ~capacity () in
      let scope = Scope.Expert.create ~sw ~inbox ~max_tasks in
      f scope inbox;
      Scope.cancel scope;
      Inbox.close inbox))
;;

let%expect_test "UI loading controls reject stale tokens and backpressure without raising"
  =
  with_scope (fun scope inbox ->
    let started = ref 0 in
    let t =
      P.create ~scope (forest 70) ~load:(fun _ ->
        Int.incr started;
        Eio.Fiber.await_cancel ())
      |> ok
    in
    let module Controls = Gpuio_bonsai.Tree_rows.Loading in
    let errors = ref [] in
    let controls =
      P.controls
        ~on_error:(fun error ->
          Bonsai.Effect.of_thunk (fun () -> errors := error :: !errors))
        t
    in
    let run = Bonsai.Effect.Expert.handle in
    let target i = P.Snapshot.target (P.snapshot t) (id (Int.to_string i)) |> ok in
    let old = target 0 in
    let foreign = Gpuio.Tree_loading.create (forest 70) |> Gpuio.Tree_loading.snapshot in
    run
      (Controls.request
         controls
         (Gpuio.Tree_loading.Snapshot.target foreign (id "0") |> ok));
    settle inbox;
    assert (!started = 0);
    List.iter (List.range 0 70) ~f:(fun i -> run (Controls.request controls (target i)));
    settle inbox;
    assert (!started = 4);
    assert (P.Snapshot.queued_count (P.snapshot t) = 64);
    assert (List.is_empty !errors);
    let old_lease = P.Snapshot.lease (P.snapshot t) in
    P.reset t (forest 1) |> ok;
    run (Controls.request controls old);
    run
      (Controls.cancel_hidden
         controls
         old_lease
         (Gpuio.Tree_state.create (tree t) () |> ok));
    settle inbox;
    assert (P.Snapshot.queued_count (P.snapshot t) = 0);
    assert (!started = 4);
    run (Controls.request controls (target 0));
    settle inbox;
    assert (!started = 5);
    let current = target 0 in
    P.close t;
    run (Controls.request controls current);
    run (Controls.retry controls current);
    run (Controls.cancel controls current);
    settle inbox;
    assert (!started = 5 && List.is_empty !errors));
  print_endline
    "four workers and 64 queued; foreign/reset/closed controls ignored; no overflow error";
  [%expect
    {| four workers and 64 queued; foreign/reset/closed controls ignored; no overflow error |}]
;;

let%expect_test "page completion retires a UI target while payload updates preserve it" =
  with_scope (fun scope inbox ->
    let started = ref 0 in
    let t =
      P.create ~scope (forest 1) ~load:(fun _ ->
        Int.incr started;
        Ok { P.Page.roots = []; nodes = []; next = More (Some (Int.to_string !started)) })
      |> ok
    in
    let controls = P.controls t in
    let module Controls = Gpuio_bonsai.Tree_rows.Loading in
    let run = Bonsai.Effect.Expert.handle in
    let original = P.Snapshot.target (P.snapshot t) (id "0") |> ok in
    P.update t (T.set_data (tree t) ~id:(id "0") () |> ok) |> ok;
    assert (P.Snapshot.is_current (P.snapshot t) original);
    run (Controls.request controls original);
    settle inbox;
    assert (!started = 1);
    assert (not (P.Snapshot.is_current (P.snapshot t) original));
    run (Controls.request controls original);
    settle inbox;
    assert (!started = 1);
    run (Controls.request controls (P.Snapshot.target (P.snapshot t) (id "0") |> ok));
    settle inbox;
    assert (!started = 2);
    P.close t);
  [%expect {| |}]
;;

let%expect_test "tree results and failures publish on the UI turn with explicit retry" =
  with_scope (fun scope inbox ->
    let changes = ref 0 in
    let failed = ref true in
    let t =
      P.create
        ~scope
        (forest 1)
        ~on_change:(fun _ -> Bonsai.Effect.of_thunk (fun () -> Int.incr changes))
        ~load:(fun _ ->
          if !failed then Or_error.error_string "offline" else Ok (page "loaded"))
      |> ok
    in
    request t "0";
    Eio.Fiber.yield ();
    assert (T.length (tree t) = 1);
    print_s [%sexp (status t "0" : P.Status.t option), (!changes : int)];
    settle inbox;
    print_s [%sexp (status t "0" : P.Status.t option), (!changes : int)];
    request t "0";
    P.cancel_hidden t (Gpuio.Tree_state.create (tree t) () |> ok);
    assert (!changes = 2);
    failed := false;
    P.retry t (id "0") |> ok;
    settle inbox;
    print_s
      [%sexp
        (status t "0" : P.Status.t option)
      , (T.preorder (tree t) : T.Id.t list)
      , (!changes : int)];
    P.close t);
  [%expect
    {|
    ((Loading) 1)
    (((Failed offline)) 2)
    ((End) (0 loaded) 4)
    |}]
;;

let%expect_test
    "reset suppresses a completed producer's result already queued in the UI inbox"
  =
  with_scope (fun scope inbox ->
    let changes = ref 0 in
    let t =
      P.create
        ~scope
        (forest 1)
        ~load:(fun _ -> Ok (page "obsolete"))
        ~on_change:(fun _ -> Bonsai.Effect.of_thunk (fun () -> Int.incr changes))
      |> ok
    in
    request t "0";
    Eio.Fiber.yield ();
    P.reset t (forest 1) |> ok;
    settle inbox;
    assert (Option.is_none (T.find (tree t) (id "obsolete")));
    print_s
      [%sexp
        (T.length (tree t) : int)
      , (P.Snapshot.generation (P.snapshot t) : int64)
      , (!changes : int)];
    request t "0";
    settle inbox;
    assert (Option.is_some (T.find (tree t) (id "obsolete")));
    P.close t);
  [%expect {| (1 1 2) |}]
;;

let%expect_test "cancelled producers release worker slots before queued work starts" =
  with_scope (fun scope inbox ->
    let active = ref 0 in
    let peak = ref 0 in
    let cancelled = ref 0 in
    let conversation_alive = ref true in
    ignore
      (Scope.start
         scope
         ~f:(fun () ->
           Exn.protect ~f:Eio.Fiber.await_cancel ~finally:(fun () ->
             conversation_alive := false))
         ~on_result:(fun (_ : unit Or_error.t) -> Bonsai.Effect.Ignore)
       |> ok
       : Scope.Task.t);
    let t =
      P.create ~scope (forest 6) ~load:(fun _ ->
        Int.incr active;
        peak := Int.max !peak !active;
        Exn.protect ~f:Eio.Fiber.await_cancel ~finally:(fun () ->
          Int.decr active;
          Int.incr cancelled))
      |> ok
    in
    for i = 0 to 5 do
      request t (Int.to_string i)
    done;
    Eio.Fiber.yield ();
    assert (!active = 4 && !peak = 4);
    assert (P.Snapshot.queued_count (P.snapshot t) = 2);
    P.cancel t (id "0");
    request t "0";
    assert (P.Snapshot.queued_count (P.snapshot t) = 3);
    settle inbox;
    assert (!active = 4 && !peak = 4 && !cancelled = 1);
    print_s
      [%sexp
        (!active : int)
      , (!peak : int)
      , (!cancelled : int)
      , (P.Snapshot.queued_count (P.snapshot t) : int)];
    P.close t;
    P.close t;
    settle inbox;
    assert !conversation_alive;
    print_s
      [%sexp
        (!active : int)
      , (!peak : int)
      , (!cancelled : int)
      , (Result.is_error (P.request t (id "0")) : bool)];
    Scope.cancel scope;
    Eio.Fiber.yield ();
    assert (not !conversation_alive));
  [%expect
    {|
    (4 4 1 2)
    (0 4 5 true)
    |}]
;;

let%expect_test "scope shutdown cancels workers blocked by bounded inbox capacity" =
  with_scope ~capacity:1 (fun scope inbox ->
    let calls = ref 0 in
    let changes = ref 0 in
    let t =
      P.create
        ~scope
        (forest 4)
        ~on_change:(fun _ -> Bonsai.Effect.of_thunk (fun () -> Int.incr changes))
        ~load:(fun request ->
          Int.incr calls;
          Ok (page ("loaded-" ^ T.Id.to_string (P.Request.parent request))))
      |> ok
    in
    for i = 0 to 3 do
      request t (Int.to_string i)
    done;
    Eio.Fiber.yield ();
    assert (!calls = 4);
    Scope.cancel scope;
    settle inbox;
    assert (T.length (tree t) = 4);
    print_s
      [%sexp
        (!calls : int)
      , (!changes : int)
      , (P.Snapshot.running_count (P.snapshot t) : int)
      , (P.Snapshot.queued_count (P.snapshot t) : int)];
    assert (Result.is_error (P.request t (id "0"))));
  [%expect {| (4 4 0 0) |}]
;;

let%expect_test "worker admission failure becomes retryable failure, not a stranded queue"
  =
  with_scope ~max_tasks:1 (fun scope inbox ->
    let t = P.create ~scope (forest 2) ~load:(fun _ -> Eio.Fiber.await_cancel ()) |> ok in
    request t "0";
    request t "1";
    print_s
      [%sexp
        (status t "1" : P.Status.t option), (P.Snapshot.queued_count (P.snapshot t) : int)];
    P.cancel t (id "0");
    settle inbox;
    P.retry t (id "1") |> ok;
    Eio.Fiber.yield ();
    print_s [%sexp (status t "1" : P.Status.t option)];
    P.close t;
    settle inbox);
  [%expect
    {|
    (((Failed "task limit reached")) 0)
    (Loading)
    |}]
;;
