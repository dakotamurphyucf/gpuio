open Core
module D = Gpuio.Table_data
module P = Gpuio_eio.Table_paging
module Scope = Gpuio_eio.Scope
module Inbox = Gpuio_eio.Inbox

let ok = Or_error.ok_exn
let id text = D.Id.of_string text |> ok
let empty () = D.create [] |> ok
let drain inbox = List.iter (Inbox.take_turn inbox) ~f:(fun f -> f ())

let settle inbox =
  for _ = 1 to 20 do
    Eio.Fiber.yield ();
    drain inbox
  done
;;

let with_scope ?(capacity = 16) ?(max_tasks = 8) f =
  Eio_mock.Backend.run (fun () ->
    Eio.Switch.run (fun sw ->
      let inbox = Inbox.create ~capacity () in
      let scope = Scope.Expert.create ~sw ~inbox ~max_tasks in
      Exn.protect
        ~f:(fun () -> f scope inbox)
        ~finally:(fun () ->
          Scope.cancel scope;
          Inbox.close inbox)))
;;

let status t = (P.snapshot t).after
let names t = D.keys (P.snapshot t).data |> List.map ~f:D.Id.to_string
let request t direction = P.request t direction |> ok

let%expect_test "query snapshots load off the UI loop and failures require explicit retry"
  =
  with_scope (fun scope inbox ->
    let changes = ref 0 in
    let calls = ref 0 in
    let fail = ref true in
    let t =
      P.create
        ~scope
        ~query:"name ascending"
        (empty ())
        ~before:End
        ~after:(More None)
        ~load:(fun request ->
          assert (String.equal (P.Request.query request) "name ascending");
          incr calls;
          if !fail
          then Or_error.error_string "offline"
          else Ok { P.Page.rows = [ id "row", "日本語" ]; next = End })
        ~on_change:(fun _ -> Bonsai.Effect.of_thunk (fun () -> incr changes))
      |> ok
    in
    request t After;
    assert (!calls = 0);
    Eio.Fiber.yield ();
    assert (D.is_empty (P.snapshot t).data);
    assert (!changes = 1);
    settle inbox;
    print_s [%sexp (status t : P.Status.t), (!calls : int)];
    request t After;
    settle inbox;
    assert (!calls = 1);
    fail := false;
    P.retry t After |> ok;
    settle inbox;
    print_s [%sexp (names t : string list), (status t : P.Status.t), (!calls : int)];
    P.close t;
    settle inbox);
  [%expect
    {|
    ((Failed offline) 1)
    ((row) End 2)
    |}]
;;

let%expect_test
    "rapid resets cannot exceed two producers while cancellation cleanup waits"
  =
  with_scope (fun scope inbox ->
    let cleanup_ready, resolver = Eio.Promise.create () in
    let released = ref false in
    let release () =
      if not !released
      then (
        released := true;
        Eio.Promise.resolve resolver ())
    in
    let active = ref 0 in
    let peak = ref 0 in
    let started = ref [] in
    let finished = ref 0 in
    let t =
      P.create
        ~scope
        ~query:0
        (empty ())
        ~before:(More None)
        ~after:(More None)
        ~load:(fun request ->
          let query = P.Request.query request in
          started := query :: !started;
          incr active;
          peak := Int.max !peak !active;
          Exn.protect
            ~f:(fun () ->
              if query = 0 then Eio.Fiber.await_cancel ();
              let direction =
                match P.Request.direction request with
                | Before -> "before"
                | After -> "after"
              in
              Ok
                { P.Page.rows = [ id (sprintf "%d-%s" query direction), query ]
                ; next = End
                })
            ~finally:(fun () ->
              if query = 0
              then Eio.Cancel.protect (fun () -> Eio.Promise.await cleanup_ready);
              decr active;
              incr finished))
      |> ok
    in
    Exn.protect ~finally:release ~f:(fun () ->
      request t Before;
      request t After;
      settle inbox;
      assert (!active = 2);
      for query = 1 to 100 do
        P.reset t ~query (empty ()) ~before:(More None) ~after:(More None) |> ok;
        request t Before;
        request t After
      done;
      settle inbox;
      assert (!active = 2);
      assert (List.length !started = 2);
      release ();
      settle inbox;
      print_s
        [%sexp
          (List.sort !started ~compare:Int.compare : int list)
        , (!peak : int)
        , (!finished : int)
        , (!active : int)];
      print_s [%sexp (names t : string list)];
      P.close t;
      settle inbox));
  [%expect
    {|
    ((0 0 100 100) 2 4 0)
    (100-before 100-after)
    |}]
;;

let%expect_test "a saturated UI inbox cannot publish pages from the previous sort" =
  with_scope ~capacity:1 (fun scope inbox ->
    let published = ref [] in
    let t =
      P.create
        ~scope
        ~query:"old"
        (empty ())
        ~before:(More None)
        ~after:(More None)
        ~load:(fun request ->
          let direction =
            match P.Request.direction request with
            | Before -> "before"
            | After -> "after"
          in
          Ok
            { P.Page.rows = [ id (P.Request.query request ^ "-" ^ direction), () ]
            ; next = End
            })
        ~on_change:(fun snapshot ->
          Bonsai.Effect.of_thunk (fun () ->
            published := List.map (D.keys snapshot.data) ~f:D.Id.to_string :: !published))
      |> ok
    in
    request t Before;
    request t After;
    for _ = 1 to 4 do
      Eio.Fiber.yield ()
    done;
    assert (D.is_empty (P.snapshot t).data);
    P.reset t ~query:"new" (empty ()) ~before:(More None) ~after:(More None) |> ok;
    request t Before;
    request t After;
    settle inbox;
    assert (
      not (List.exists !published ~f:(List.exists ~f:(String.is_prefix ~prefix:"old"))));
    print_s [%sexp (names t : string list)];
    P.close t;
    settle inbox);
  [%expect {| (new-before new-after) |}]
;;

let%expect_test "invalid reset preserves a producer; close affects only its own workers" =
  with_scope (fun scope inbox ->
    let cancelled = ref 0 in
    let other_alive = ref false in
    let other =
      Scope.start
        scope
        ~f:(fun () ->
          other_alive := true;
          Exn.protect ~f:Eio.Fiber.await_cancel ~finally:(fun () -> other_alive := false))
        ~on_result:(fun (_ : unit Or_error.t) -> Bonsai.Effect.Ignore)
      |> ok
    in
    let t =
      P.create
        ~scope
        ~query:0
        (empty ())
        ~before:(More None)
        ~after:(More None)
        ~load:(fun _ ->
          Exn.protect ~f:Eio.Fiber.await_cancel ~finally:(fun () -> incr cancelled))
      |> ok
    in
    request t Before;
    request t After;
    settle inbox;
    assert (
      Result.is_error
        (P.reset
           t
           ~query:1
           (empty ())
           ~before:End
           ~after:(More (Some (String.make 4097 'x')))));
    settle inbox;
    assert (!cancelled = 0);
    assert ((P.snapshot t).query = 0);
    P.cancel t Before;
    settle inbox;
    assert (!cancelled = 1);
    P.close t;
    P.close t;
    settle inbox;
    print_s
      [%sexp
        (!cancelled : int)
      , (!other_alive : bool)
      , (Result.is_error (P.request t After) : bool)];
    assert (not (Scope.Task.is_finished other));
    Scope.cancel scope;
    settle inbox;
    assert (not !other_alive);
    assert (Scope.Task.is_finished other));
  [%expect {| (2 true true) |}]
;;

let%expect_test
    "worker admission failures recover through retry and scope shutdown disposes loads"
  =
  with_scope ~max_tasks:1 (fun scope inbox ->
    let cancelled = ref 0 in
    let t =
      P.create
        ~scope
        ~query:0
        (empty ())
        ~before:(More None)
        ~after:(More None)
        ~load:(fun _ ->
          Exn.protect ~f:Eio.Fiber.await_cancel ~finally:(fun () -> incr cancelled))
      |> ok
    in
    request t Before;
    request t After;
    print_s [%sexp (status t : P.Status.t)];
    settle inbox;
    P.cancel t Before;
    settle inbox;
    P.retry t After |> ok;
    settle inbox;
    print_s [%sexp (status t : P.Status.t), (!cancelled : int)];
    Scope.cancel scope;
    settle inbox;
    print_s [%sexp (!cancelled : int), (Result.is_error (P.request t Before) : bool)];
    assert (
      Result.is_error
        (P.create ~scope ~query:0 (empty ()) ~before:End ~after:End ~load:(fun _ ->
           assert false))));
  [%expect
    {|
    (Failed "task limit reached")
    (Loading 1)
    (2 true)
    |}]
;;
