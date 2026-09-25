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
        ~on_change:(fun _ -> Bonsai.Effect.of_thunk (fun () -> Int.incr changes))
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
        ~on_change:(fun _ -> Bonsai.Effect.of_thunk (fun () -> Int.incr changes))
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
        ~on_change:(fun _ -> Bonsai.Effect.Ignore)
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
        ~on_change:(fun _ -> Bonsai.Effect.of_thunk (fun () -> Int.incr changes))
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

let%expect_test
    "managed paging fills empty pages, waits for fresh layout and guards generations"
  =
  let module B = Bonsai.Cont in
  let module V = Gpuio_bonsai.Virtual_list in
  with_scope (fun scope inbox ->
    let calls = ref [] in
    let fail = ref false in
    let pager =
      P.create ~scope (empty ()) ~before:End ~after:(More None) ~load:(fun request ->
        calls := P.Request.cursor request :: !calls;
        if !fail
        then Or_error.error_string "offline"
        else (
          match P.Request.cursor request with
          | None -> Ok { P.Page.rows = []; next = More (Some "next") }
          | Some "next" -> Ok { P.Page.rows = [ 1, "first" ]; next = More (Some "last") }
          | Some _ -> Ok { P.Page.rows = [ 2, "last" ]; next = End }))
      |> Or_error.ok_exn
    in
    let controls = P.controls pager in
    let config =
      V.Config.create ~max_active:8 ~height:(Estimated 80.) () |> Or_error.ok_exn
    in
    let driver =
      Bonsai_driver.create
        ~action_history:Release_after_flush
        ~clock:(Bonsai.Time_source.create ~start:Time_ns.epoch)
        (fun graph ->
           V.paged
             (module Int)
             (P.value pager)
             ~paging:(B.return controls)
             ~row_key:Gpuio.Key.of_int
             ~config
             ~render_row:(fun ~key:_ ~data ~lifetime:_ _ -> B.map data ~f:Gpuio.View.text)
             graph)
    in
    let result () =
      Bonsai_driver.flush driver;
      Bonsai_driver.result driver |> Or_error.ok_exn
    in
    let display () =
      ignore (result () : int V.Output.t);
      Bonsai_driver.trigger_lifecycles driver;
      ignore (result () : int V.Output.t)
    in
    let report ~at_end =
      let output = result () in
      let list =
        (Gpuio.View.Expert.describe (V.Output.view output)).children
        |> List.hd_exn
        |> Gpuio.View.Expert.describe
      in
      let list = Option.value_exn list.virtual_list in
      let rows = C.keys (P.items pager) |> List.map ~f:Gpuio.Key.of_int in
      let viewport : Gpuio.Virtual_list.Viewport.t =
        { visible_first = 0
        ; visible_last = List.length rows
        ; requested = rows
        ; pinned = []
        ; anchor = None
        ; following_tail = false
        ; at_start = true
        ; at_end
        ; budget_exhausted = false
        }
      in
      Bonsai_driver.schedule_event driver (Option.value_exn list.on_viewport viewport)
    in
    display ();
    assert (List.is_empty !calls);
    report ~at_end:true;
    display ();
    Eio.Fiber.yield ();
    drain inbox;
    display ();
    (* The empty cursor-advancing page needs no new geometry; it can continue. *)
    Eio.Fiber.yield ();
    drain inbox;
    display ();
    assert (List.length !calls = 2 && C.length (P.items pager) = 1);
    assert (Option.is_none (V.Output.viewport (result ())));
    Eio.Fiber.yield ();
    drain inbox;
    display ();
    assert (List.length !calls = 2);
    (* A nonempty page waits for native layout; an old at-end flag must not
       greedily load the entire history before a new frame is painted. *)
    report ~at_end:false;
    display ();
    assert (List.length !calls = 2);
    fail := true;
    report ~at_end:true;
    display ();
    Eio.Fiber.yield ();
    drain inbox;
    display ();
    assert (List.length !calls = 3);
    for _ = 1 to 4 do
      report ~at_end:true;
      display ()
    done;
    Eio.Fiber.yield ();
    assert (List.length !calls = 3);
    fail := false;
    Bonsai_driver.schedule_event driver (V.Paging.retry controls ~generation:0L After);
    Eio.Fiber.yield ();
    drain inbox;
    display ();
    assert (C.length (P.items pager) = 2);
    assert (
      match P.status pager After with
      | End -> true
      | Ready | Loading | Failed _ -> false);
    P.reset pager (empty ()) ~before:End ~after:(More None) |> Or_error.ok_exn;
    display ();
    Bonsai_driver.schedule_event driver (V.Paging.request controls ~generation:0L After);
    Eio.Fiber.yield ();
    assert (List.length !calls = 4);
    P.close pager;
    Bonsai_driver.Expert.invalidate_observers driver);
  [%expect {| |}]
;;

let%expect_test "append during queued older history preserves scoped completion" =
  with_scope (fun scope inbox ->
    let t =
      P.create ~scope (empty ()) ~before:(More None) ~after:End ~load:(fun _ ->
        Ok { P.Page.rows = [ 1, "history" ]; next = End })
      |> Or_error.ok_exn
    in
    P.request t Before |> Or_error.ok_exn;
    Eio.Fiber.yield ();
    P.append t [ 2, "live" ] |> Or_error.ok_exn;
    drain inbox;
    print_s [%sexp (C.to_alist (P.items t) : (int * string) list)];
    P.close t;
    print_s [%sexp (Result.is_error (P.append t [ 3, "late" ]) : bool)]);
  [%expect
    {|
    ((1 history) (2 live))
    true
    |}]
;;
