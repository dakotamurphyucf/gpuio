open Core
module C = Gpuio.List_collection
module P = Gpuio_eio.List_search
module Scope = Gpuio_eio.Scope
module Inbox = Gpuio_eio.Inbox

let ok = Or_error.ok_exn
let source () = C.of_alist (module Int) [ 1, "one"; 2, "two" ] |> ok
let drain inbox = List.iter (Inbox.take_turn inbox) ~f:(fun f -> f ())

let settle inbox =
  for _ = 1 to 5 do
    Eio.Fiber.yield ();
    drain inbox
  done
;;

let with_scope ?(max_tasks = 8) ?(capacity = 16) f =
  Eio_mock.Backend.run (fun () ->
    Eio.Switch.run (fun sw ->
      let inbox = Inbox.create ~capacity () in
      let scope = Scope.Expert.create ~sw ~inbox ~max_tasks in
      let clock = Eio_mock.Clock.Mono.make () in
      f scope inbox clock;
      Scope.cancel scope;
      Inbox.close inbox))
;;

let snap = P.snapshot
let items t = P.Snapshot.items (snap t)
let epoch t = P.Snapshot.epoch (snap t)
let query t text = P.set_query t ~source:(P.Snapshot.source_id (snap t)) text |> ok

let ready t =
  match P.Snapshot.status (snap t) with
  | Ready -> true
  | _ -> false
;;

let failed t =
  match P.Snapshot.status (snap t) with
  | Failed _ -> true
  | _ -> false
;;

let immediate ~scope ~clock ?max_loaded ?on_change items ~search =
  P.create ~scope ~clock ~debounce:Time_ns.Span.zero ?max_loaded ?on_change items ~search
  |> ok
;;

let%expect_test
    "debounce uses monotonic Eio deadlines, coalesces typing and publishes on UI loop"
  =
  with_scope (fun scope inbox clock ->
    let searched = ref []
    and statuses = ref [] in
    let t =
      P.create
        ~scope
        ~clock
        (source ())
        ~search:(fun request ->
          searched := P.Request.query request :: !searched;
          Ok { P.Page.upsert = []; visible = [ 1 ] })
        ~on_change:(fun snapshot ->
          Bonsai.Effect.of_thunk (fun () ->
            statuses := P.Snapshot.status snapshot :: !statuses))
      |> ok
    in
    assert ((Scope.stats scope).tasks = 0);
    List.iter [ "o"; "on"; "one" ] ~f:(query t);
    Eio.Fiber.yield ();
    Eio_mock.Clock.Mono.set_time clock (Mtime.of_uint64_ns 149_000_000L);
    Eio.Fiber.yield ();
    assert (List.is_empty !searched);
    assert (P.Snapshot.is_busy (snap t) && P.Snapshot.is_stale (snap t));
    Eio_mock.Clock.Mono.set_time clock (Mtime.of_uint64_ns 150_000_000L);
    Eio.Fiber.yield ();
    assert (not (ready t));
    settle inbox;
    assert (List.equal String.equal !searched [ "one" ]);
    assert (ready t && not (P.Snapshot.is_stale (snap t)));
    assert (List.equal Int.equal (P.Snapshot.visible (snap t)) [ 1 ]);
    assert (C.length (items t) = 2);
    assert ((Scope.stats scope).tasks = 0);
    print_s [%sexp (List.rev !statuses : P.Status.t list)];
    P.close t);
  [%expect
    {|
    +mock time is now 0.149
    +mock time is now 0.15
    (Debouncing Debouncing Debouncing Loading Ready)
    |}]
;;

let%expect_test
    "queued old query and source completions cannot publish or resurrect memberships"
  =
  with_scope (fun scope inbox clock ->
    let calls = ref 0 in
    let t =
      immediate ~scope ~clock (source ()) ~search:(fun request ->
        Int.incr calls;
        Ok { P.Page.upsert = [ 3, P.Request.query request ]; visible = [ 3 ] })
    in
    let old_source = P.Snapshot.source_id (snap t) in
    query t "old";
    Eio.Fiber.yield ();
    assert (Inbox.length inbox = 1);
    query t "new";
    settle inbox;
    assert (String.equal (C.find (items t) 3 |> Option.value_exn) "new");
    let old_target = C.item_ref (items t) 3 |> Option.value_exn in
    query t "old-source";
    Eio.Fiber.yield ();
    let fresh = C.of_alist (module Int) [ 1, "new source" ] |> ok in
    P.update_source t fresh |> ok;
    let new_epoch = epoch t in
    P.set_query t ~source:old_source "obsolete editor" |> ok;
    assert (Gpuio.Key.equal new_epoch (epoch t));
    settle inbox;
    assert (not (C.contains_ref (items t) old_target));
    assert (String.equal (P.Snapshot.query (snap t)) "old-source");
    assert (!calls = 4);
    P.close t);
  [%expect {| |}]
;;

let%expect_test "remote merges preserve hidden membership and newer streamed values" =
  with_scope (fun scope inbox clock ->
    let initial = source () in
    let original = C.item_ref initial 1 |> Option.value_exn in
    let hidden = C.item_ref initial 2 |> Option.value_exn in
    let t =
      immediate ~scope ~clock initial ~search:(fun request ->
        assert (phys_equal (P.Request.items request) initial);
        Ok { P.Page.upsert = [ 1, "old remote copy"; 3, "fetched" ]; visible = [ 3; 1 ] })
    in
    query t "remote";
    Eio.Fiber.yield ();
    let token = epoch t in
    P.update_source t ~refresh:false (C.set (items t) ~key:1 ~data:"new stream" |> ok)
    |> ok;
    assert (Gpuio.Key.equal token (epoch t));
    settle inbox;
    assert (C.contains_ref (items t) original && C.contains_ref (items t) hidden);
    assert (String.equal (C.find (items t) 1 |> Option.value_exn) "new stream");
    assert (String.equal (C.find (items t) 3 |> Option.value_exn) "fetched");
    assert (List.equal Int.equal (C.keys (items t)) [ 1; 2; 3 ]);
    assert (List.equal Int.equal (P.Snapshot.visible (snap t)) [ 3; 1 ]);
    assert (ready t);
    P.close t);
  [%expect {| |}]
;;

let%expect_test
    "invalid result pages fail atomically and explicit retry validates new data"
  =
  with_scope (fun scope inbox clock ->
    let page = ref { P.Page.upsert = []; visible = [] } in
    let t =
      immediate ~scope ~clock ~max_loaded:3 (source ()) ~search:(fun _ -> Ok !page)
    in
    List.iter
      [ { P.Page.upsert = [ 1, "overwrite"; 1, "duplicate" ]; visible = [ 1 ] }
      ; { upsert = [ 1, "overwrite"; 3, "fetched" ]; visible = [ 3; 99 ] }
      ; { upsert = [ 3, "fetched" ]; visible = [ 3; 3 ] }
      ; { upsert = [ 3, "three"; 4, "four" ]; visible = [ 3; 4 ] }
      ]
      ~f:(fun invalid ->
        page := invalid;
        let before = items t in
        P.refresh t |> ok;
        settle inbox;
        assert (failed t && phys_equal before (items t)));
    let failed_epoch = epoch t in
    page := { P.Page.upsert = [ 3, "third" ]; visible = [ 3 ] };
    query t "";
    assert (failed t);
    P.retry t ~epoch:failed_epoch |> ok;
    let retry_epoch = epoch t in
    P.cancel t ~epoch:failed_epoch;
    assert (Gpuio.Key.equal retry_epoch (epoch t));
    settle inbox;
    assert (ready t && C.length (items t) = 3);
    let before = epoch t in
    P.retry t ~epoch:failed_epoch |> ok;
    assert (Gpuio.Key.equal before (epoch t));
    P.close t);
  [%expect {| |}]
;;

let%expect_test "cancel, parent close and task admission failures do not strand producers"
  =
  with_scope ~max_tasks:1 (fun scope inbox clock ->
    let unwound = ref 0 in
    let t =
      immediate ~scope ~clock (source ()) ~search:(fun _ ->
        Exn.protect ~f:Eio.Fiber.await_cancel ~finally:(fun () -> Int.incr unwound))
    in
    query t "first";
    Eio.Fiber.yield ();
    let first = epoch t in
    (* Cancelled fibers are charged until they unwind. No unbounded replacement
       queue is allocated when the shared scope limit is reached. *)
    assert (
      Result.is_error (P.set_query t ~source:(P.Snapshot.source_id (snap t)) "second"));
    assert (failed t);
    settle inbox;
    assert (!unwound = 1 && (Scope.stats scope).tasks = 0);
    P.retry t ~epoch:(epoch t) |> ok;
    Eio.Fiber.yield ();
    P.cancel t ~epoch:first;
    assert (P.Snapshot.is_busy (snap t));
    P.cancel t ~epoch:(epoch t);
    settle inbox;
    assert (!unwound = 2 && (Scope.stats scope).tasks = 0);
    (match P.Snapshot.status (snap t) with
     | Cancelled -> ()
     | _ -> assert false);
    P.retry t ~epoch:(epoch t) |> ok;
    Eio.Fiber.yield ();
    Scope.cancel scope;
    settle inbox;
    assert (!unwound = 3);
    (match P.Snapshot.status (snap t) with
     | Closed -> ()
     | _ -> assert false);
    assert ((Scope.stats scope).tasks = 0 && (Scope.stats scope).cleanups = 0);
    assert (Result.is_error (P.refresh t));
    P.close t);
  [%expect {| |}]
;;

let%expect_test
    "structural edits fence requests even when refresh is false and a key is reused"
  =
  with_scope (fun scope inbox clock ->
    let t =
      immediate ~scope ~clock (source ()) ~search:(fun _ ->
        Ok { P.Page.upsert = []; visible = [ 1 ] })
    in
    query t "one";
    settle inbox;
    let old_target = C.item_ref (items t) 1 |> Option.value_exn in
    let token = epoch t in
    let next = C.splice (items t) ~at:0 ~remove:1 [] |> ok in
    let next = C.splice next ~at:0 ~remove:0 [ 1, "new one" ] |> ok in
    P.update_source t ~refresh:false next |> ok;
    assert (not (Gpuio.Key.equal token (epoch t)));
    assert (List.is_empty (P.Snapshot.visible (snap t)));
    assert (not (C.contains_ref (items t) old_target));
    settle inbox;
    assert (List.equal Int.equal (P.Snapshot.visible (snap t)) [ 1 ]);
    P.close t);
  [%expect {| |}]
;;

let%expect_test "configuration and query errors preserve the accepted snapshot" =
  with_scope (fun scope _inbox clock ->
    let search _ = Ok { P.Page.upsert = []; visible = [] } in
    List.iter [ -1; 0; 1; 1_000_001 ] ~f:(fun max_loaded ->
      assert (Result.is_error (P.create ~scope ~clock ~max_loaded (source ()) ~search)));
    List.iter [ -1.; 61. ] ~f:(fun seconds ->
      assert (
        Result.is_error
          (P.create
             ~scope
             ~clock
             ~debounce:(Time_ns.Span.of_sec seconds)
             (source ())
             ~search)));
    let t = immediate ~scope ~clock (source ()) ~search in
    let before = snap t in
    List.iter
      [ "bad\000query"; "\255"; String.make 4097 'a' ]
      ~f:(fun text ->
        assert (Result.is_error (P.set_query t ~source:(P.Snapshot.source_id before) text));
        assert (phys_equal before (snap t)));
    assert ((Scope.stats scope).tasks = 0);
    P.close t);
  [%expect {| |}]
;;

let%expect_test
    "closing while the UI inbox is full unwinds superseded producers without publishing"
  =
  with_scope ~capacity:1 (fun scope inbox clock ->
    let changes = ref 0 in
    let t =
      immediate
        ~scope
        ~clock
        (source ())
        ~on_change:(fun _ -> Bonsai.Effect.of_thunk (fun () -> Int.incr changes))
        ~search:(fun _ ->
          Ok { P.Page.upsert = [ 3, "must not arrive" ]; visible = [ 3 ] })
    in
    Inbox.push inbox Fn.id;
    query t "first";
    Eio.Fiber.yield ();
    assert ((Scope.stats scope).tasks = 1 && Inbox.length inbox = 1);
    query t "second";
    Eio.Fiber.yield ();
    P.close t;
    settle inbox;
    assert ((Scope.stats scope).tasks = 0 && Inbox.length inbox = 0);
    assert (Option.is_none (C.find (items t) 3));
    assert (!changes = 2);
    match P.Snapshot.status (snap t) with
    | Closed -> ()
    | _ -> assert false);
  [%expect {| |}]
;;
