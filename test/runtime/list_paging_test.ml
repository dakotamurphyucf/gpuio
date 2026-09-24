open Core
module C = Gpuio.List_collection
module P = Gpuio_eio.List_paging
module Scope = Gpuio_eio.Scope
module Inbox = Gpuio_eio.Inbox

let drain inbox = List.iter (Inbox.take_turn inbox) ~f:(fun f -> f ())
let empty () = C.empty (module Int)

let with_scope f =
  Eio_mock.Backend.run (fun () ->
    Eio.Switch.run (fun sw ->
      let inbox = Inbox.create ~capacity:16 () in
      let scope = Scope.Expert.create ~sw ~inbox ~max_tasks:8 in
      f scope inbox;
      Scope.cancel scope;
      Inbox.close inbox))
;;

let%expect_test "paging producer results and failures publish on the UI turn" =
  with_scope (fun scope inbox ->
    let changes = ref 0 in
    let fail = ref true in
    let t =
      P.create
        ~scope
        (empty ())
        ~before:End
        ~after:(More None)
        ~load:(fun _ ->
          if !fail
          then Or_error.error_string "network unavailable"
          else Ok { P.Page.rows = [ 1, "loaded" ]; next = End })
        ~on_change:(fun () -> Bonsai.Effect.of_thunk (fun () -> Int.incr changes))
      |> Or_error.ok_exn
    in
    P.request t After |> Or_error.ok_exn;
    print_s [%sexp (P.status t After : P.Status.t), (!changes : int)];
    Eio.Fiber.yield ();
    assert (C.is_empty (P.items t));
    drain inbox;
    print_s [%sexp (P.status t After : P.Status.t), (!changes : int)];
    fail := false;
    P.request t After |> Or_error.ok_exn;
    assert (!changes = 2);
    P.retry t After |> Or_error.ok_exn;
    Eio.Fiber.yield ();
    drain inbox;
    print_s
      [%sexp
        (P.status t After : P.Status.t)
      , (C.to_alist (P.items t) : (int * string) list)
      , (!changes : int)];
    P.close t);
  [%expect
    {|
    (Loading 1)
    ((Failed "network unavailable") 2)
    (End ((1 loaded)) 4)
    |}]
;;

let%expect_test "conversation reset suppresses already queued results" =
  with_scope (fun scope inbox ->
    let changes = ref 0 in
    let t =
      P.create
        ~scope
        (empty ())
        ~before:(More None)
        ~after:(More None)
        ~load:(fun request ->
          let key =
            match P.Request.direction request with
            | Before -> 1
            | After -> 2
          in
          Ok { P.Page.rows = [ key, "old conversation" ]; next = End })
        ~on_change:(fun () -> Bonsai.Effect.of_thunk (fun () -> Int.incr changes))
      |> Or_error.ok_exn
    in
    P.request t Before |> Or_error.ok_exn;
    P.request t After |> Or_error.ok_exn;
    Eio.Fiber.yield ();
    P.reset t (empty ()) ~before:End ~after:End |> Or_error.ok_exn;
    drain inbox;
    print_s
      [%sexp
        (C.length (P.items t) : int), (!changes : int), (P.status t After : P.Status.t)];
    P.close t);
  [%expect {| (0 3 End) |}]
;;

let%expect_test "cancellation stops producers without cancelling conversation work" =
  with_scope (fun scope inbox ->
    let cancelled = ref 0 in
    let conversation_alive = ref true in
    ignore
      (Scope.start
         scope
         ~f:(fun () ->
           Exn.protect ~f:Eio.Fiber.await_cancel ~finally:(fun () ->
             conversation_alive := false))
         ~on_result:(fun (_ : unit Or_error.t) -> Bonsai.Effect.Ignore)
       |> Or_error.ok_exn
       : Scope.Task.t);
    let t =
      P.create
        ~scope
        (empty ())
        ~before:(More None)
        ~after:(More None)
        ~load:(fun _ ->
          Exn.protect ~f:Eio.Fiber.await_cancel ~finally:(fun () -> Int.incr cancelled))
        ~on_change:(fun () -> Bonsai.Effect.Ignore)
      |> Or_error.ok_exn
    in
    P.request t Before |> Or_error.ok_exn;
    P.request t After |> Or_error.ok_exn;
    Eio.Fiber.yield ();
    P.cancel t Before;
    Eio.Fiber.yield ();
    print_s [%sexp (!cancelled : int), (P.status t Before : P.Status.t)];
    P.close t;
    P.close t;
    Eio.Fiber.yield ();
    drain inbox;
    print_s
      [%sexp
        (!cancelled : int)
      , (!conversation_alive : bool)
      , (Or_error.is_error (P.request t After) : bool)];
    Scope.cancel scope;
    Eio.Fiber.yield ();
    assert (not !conversation_alive));
  [%expect
    {|
    (1 Ready)
    (2 true true)
    |}]
;;

let%expect_test "scope shutdown disposes both outstanding loads" =
  with_scope (fun scope inbox ->
    let cancelled = ref 0 in
    let changes = ref 0 in
    let t =
      P.create
        ~scope
        (empty ())
        ~before:(More None)
        ~after:(More None)
        ~load:(fun _ ->
          Exn.protect ~f:Eio.Fiber.await_cancel ~finally:(fun () -> Int.incr cancelled))
        ~on_change:(fun () -> Bonsai.Effect.of_thunk (fun () -> Int.incr changes))
      |> Or_error.ok_exn
    in
    P.request t Before |> Or_error.ok_exn;
    P.request t After |> Or_error.ok_exn;
    Eio.Fiber.yield ();
    Scope.cancel scope;
    Eio.Fiber.yield ();
    drain inbox;
    print_s [%sexp (!cancelled : int), (!changes : int)];
    assert (Or_error.is_error (P.request t After)));
  [%expect {| (2 2) |}]
;;
