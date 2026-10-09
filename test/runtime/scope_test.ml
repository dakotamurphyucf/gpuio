open Core
module Scope = Gpuio_eio.Scope
module Inbox = Gpuio_eio.Inbox
module Stream = Gpuio_eio.Stream
module E = Bonsai.Effect

let drain inbox = List.iter (Inbox.take_turn inbox) ~f:(fun f -> f ())

let%expect_test
    "diagnostics distinguish live tasks, queued results and cancellation cleanup"
  =
  Eio_mock.Backend.run (fun () ->
    Eio.Switch.run (fun sw ->
      let inbox = Inbox.create ~capacity:8 () in
      let root = Scope.Expert.create ~sw ~inbox ~max_tasks:8 in
      let child = Scope.child root ~name:"observed" |> Or_error.ok_exn in
      let print () =
        print_s [%sexp (Scope.stats root : Scope.Stats.t), (Inbox.length inbox : int)]
      in
      let unregister = Scope.on_cancel child Fn.id |> Or_error.ok_exn in
      ignore
        (Scope.start
           child
           ~f:Eio.Fiber.await_cancel
           ~on_result:(fun (_ : unit Or_error.t) ->
             failwith "cancelled producer delivered")
         |> Or_error.ok_exn
         : Scope.Task.t);
      ignore
        (Scope.start
           child
           ~f:(fun () -> ())
           ~on_result:(fun _ -> failwith "cancelled completion delivered")
         |> Or_error.ok_exn
         : Scope.Task.t);
      Eio.Fiber.yield ();
      print ();
      Scope.cancel child;
      unregister ();
      Eio.Fiber.yield ();
      print ();
      drain inbox;
      print ();
      Scope.cancel root;
      print ();
      Inbox.close inbox));
  [%expect
    {|
    (((scopes 2) (tasks 1) (cleanups 1)) 1)
    (((scopes 1) (tasks 0) (cleanups 0)) 1)
    (((scopes 1) (tasks 0) (cleanups 0)) 0)
    (((scopes 0) (tasks 0) (cleanups 0)) 0)
    |}]
;;

let%expect_test "scope cancellation is selective; queued task completion is suppressed" =
  Eio_mock.Backend.run (fun () ->
    Eio.Switch.run (fun sw ->
      let inbox = Inbox.create ~capacity:8 () in
      let root = Scope.Expert.create ~sw ~inbox ~max_tasks:8 in
      let window = Scope.child root ~name:"window" |> Or_error.ok_exn in
      let conversation = Scope.child root ~name:"conversation" |> Or_error.ok_exn in
      let finished = ref [] in
      let task scope name =
        Scope.start
          scope
          ~f:(fun () -> name)
          ~on_result:(fun result ->
            E.of_thunk (fun () -> finished := Or_error.ok_exn result :: !finished))
        |> Or_error.ok_exn
      in
      let cancelled = task window "window" in
      ignore (task conversation "conversation" : Scope.Task.t);
      Eio.Fiber.yield ();
      Scope.cancel window;
      Scope.Task.cancel cancelled;
      drain inbox;
      assert (List.equal String.equal !finished [ "conversation" ]);
      let cancelled = ref false in
      ignore
        (Scope.start
           conversation
           ~f:(fun () ->
             Exn.protect ~f:Eio.Fiber.await_cancel ~finally:(fun () -> cancelled := true))
           ~on_result:(fun (_ : unit Or_error.t) ->
             failwith "cancellation was converted into a result")
         |> Or_error.ok_exn
         : Scope.Task.t);
      Eio.Fiber.yield ();
      Scope.cancel root;
      Eio.Fiber.yield ();
      assert !cancelled;
      assert (Result.is_error (Scope.child root ~name:"late"));
      Inbox.close inbox;
      print_s [%sexp (!finished : string list)]));
  [%expect {| (conversation) |}]
;;

let%expect_test "streams batch in order and enforce capacity" =
  Eio_mock.Backend.run (fun () ->
    Eio.Switch.run (fun sw ->
      let inbox = Inbox.create ~capacity:8 () in
      let root = Scope.Expert.create ~sw ~inbox ~max_tasks:8 in
      let batches = ref [] in
      let stream =
        Stream.create ~scope:root ~capacity:3 ~on_batch:(fun values ->
          E.of_thunk (fun () -> batches := values :: !batches))
        |> Or_error.ok_exn
      in
      List.iter [ 1; 2; 3 ] ~f:(fun value -> Stream.push stream value |> Or_error.ok_exn);
      assert (Result.is_error (Stream.push stream 4));
      drain inbox;
      Stream.push stream 4 |> Or_error.ok_exn;
      Stream.close stream;
      drain inbox;
      Scope.cancel root;
      Inbox.close inbox;
      print_s [%sexp (List.rev !batches : int list list)]));
  [%expect {| ((1 2 3)) |}]
;;

let%expect_test
    "a completion wakes a scheduler with no periodic timer; errors remain results"
  =
  Eio_mock.Backend.run (fun () ->
    Eio.Switch.run (fun sw ->
      let inbox = Inbox.create ~capacity:1 () in
      let root = Scope.Expert.create ~sw ~inbox ~max_tasks:2 in
      let completion = ref None in
      ignore
        (Scope.start
           root
           ~f:(fun () ->
             Eio.Fiber.yield ();
             failwith "task failed")
           ~on_result:(fun result ->
             E.of_thunk (fun () -> completion := Some (Result.is_error result)))
         |> Or_error.ok_exn
         : Scope.Task.t);
      while Option.is_none !completion do
        Inbox.await inbox;
        drain inbox
      done;
      assert (Option.value_exn !completion);
      Scope.cancel root;
      Inbox.close inbox;
      print_endline "EVENT_WAKE_PASS"));
  [%expect {| EVENT_WAKE_PASS |}]
;;

let%expect_test "pending producers obey task bounds and external cancellation propagates" =
  Eio_mock.Backend.run (fun () ->
    Eio.Switch.run (fun sw ->
      let inbox = Inbox.create ~capacity:1 () in
      let root = Scope.Expert.create ~sw ~inbox ~max_tasks:1 in
      let task =
        Scope.start
          root
          ~f:Eio.Fiber.await_cancel
          ~on_result:(fun (_ : unit Or_error.t) -> assert false)
        |> Or_error.ok_exn
      in
      assert (
        Result.is_error
          (Scope.start root ~f:(fun () -> ()) ~on_result:(fun _ -> E.Ignore)));
      Eio.Fiber.yield ();
      Scope.Task.cancel task;
      Eio.Fiber.yield ();
      assert (Scope.Task.is_finished task);
      Scope.cancel root;
      Inbox.close inbox));
  let propagated = ref false in
  Eio_mock.Backend.run (fun () ->
    try
      Eio.Cancel.sub (fun context ->
        Eio.Switch.run (fun sw ->
          let inbox = Inbox.create ~capacity:1 () in
          let root = Scope.Expert.create ~sw ~inbox ~max_tasks:1 in
          ignore
            (Scope.start
               root
               ~f:Eio.Fiber.await_cancel
               ~on_result:(fun (_ : unit Or_error.t) -> assert false)
             |> Or_error.ok_exn
             : Scope.Task.t);
          Eio.Cancel.cancel context Exit;
          Eio.Fiber.yield ()))
    with
    | Eio.Cancel.Cancelled _ -> propagated := true);
  assert !propagated;
  print_endline "CANCELLATION_PASS";
  [%expect {| CANCELLATION_PASS |}]
;;

let%expect_test "full scheduler rejects a stream value without stranding its next batch" =
  Eio_mock.Backend.run (fun () ->
    Eio.Switch.run (fun sw ->
      let inbox = Inbox.create ~capacity:1 () in
      let root = Scope.Expert.create ~sw ~inbox ~max_tasks:1 in
      let received = ref [] in
      let stream =
        Stream.create ~scope:root ~capacity:4 ~on_batch:(fun values ->
          E.of_thunk (fun () -> received := values))
        |> Or_error.ok_exn
      in
      assert (Inbox.try_push inbox Fn.id);
      assert (Result.is_error (Stream.push stream 1));
      drain inbox;
      Stream.push stream 2 |> Or_error.ok_exn;
      drain inbox;
      print_s [%sexp (!received : int list)];
      Scope.cancel root;
      Inbox.close inbox));
  [%expect {| (2) |}]
;;

let%expect_test "raising cleanup cannot strand sibling scopes or their producers" =
  Eio_mock.Backend.run (fun () ->
    Eio.Switch.run (fun sw ->
      let inbox = Inbox.create ~capacity:8 () in
      let root = Scope.Expert.create ~sw ~inbox ~max_tasks:8 in
      let first = Scope.child root ~name:"first" |> Or_error.ok_exn in
      let nested = Scope.child first ~name:"nested" |> Or_error.ok_exn in
      let sibling = Scope.child root ~name:"sibling" |> Or_error.ok_exn in
      let calls = ref [] in
      let register scope name ~raises =
        Scope.on_cancel scope (fun () ->
          calls := name :: !calls;
          if raises then failwith name)
        |> Or_error.ok_exn
      in
      let unregister = register nested "nested-first" ~raises:true in
      ignore (register nested "nested-second" ~raises:false : unit -> unit);
      ignore (register first "parent" ~raises:false : unit -> unit);
      ignore (register sibling "sibling" ~raises:true : unit -> unit);
      ignore (register root "root" ~raises:false : unit -> unit);
      let task =
        Scope.start
          sibling
          ~f:Eio.Fiber.await_cancel
          ~on_result:(fun (_ : unit Or_error.t) -> failwith "cancelled task delivered")
        |> Or_error.ok_exn
      in
      Eio.Fiber.yield ();
      let failure =
        try
          Scope.cancel root;
          "none"
        with
        | Failure message -> message
      in
      Eio.Fiber.yield ();
      print_s [%sexp (failure : string), (List.rev !calls : string list)];
      print_s
        [%sexp
          (List.map [ root; first; nested; sibling ] ~f:Scope.is_active : bool list)
        , (Scope.Task.is_finished task : bool)
        , (Scope.stats root : Scope.Stats.t)];
      Scope.cancel root;
      unregister ();
      print_s [%sexp (List.length !calls : int), (Scope.stats root : Scope.Stats.t)];
      (* Keep the deliberately failing pre-repair test from stranding its fiber. *)
      Scope.Task.cancel task;
      Eio.Fiber.yield ();
      Inbox.close inbox));
  [%expect
    {|
    (nested-first (nested-first nested-second parent sibling root))
    ((false false false false) true ((scopes 0) (tasks 0) (cleanups 0)))
    (5 ((scopes 0) (tasks 0) (cleanups 0)))
    |}]
;;

let%expect_test
    "reentrant failing cancellation suppresses queued streams and spares peers"
  =
  Eio_mock.Backend.run (fun () ->
    Eio.Switch.run (fun sw ->
      let inbox = Inbox.create ~capacity:8 () in
      let root = Scope.Expert.create ~sw ~inbox ~max_tasks:8 in
      let branch = Scope.child root ~name:"closing" |> Or_error.ok_exn in
      let peer = Scope.child root ~name:"peer" |> Or_error.ok_exn in
      let calls = ref [] in
      let unregister =
        Scope.on_cancel branch (fun () ->
          Scope.cancel branch;
          calls := "cleanup" :: !calls;
          assert (Result.is_error (Scope.child branch ~name:"late"));
          failwith "cleanup")
        |> Or_error.ok_exn
      in
      let stream =
        Stream.create ~scope:branch ~capacity:4 ~on_batch:(fun (_ : int list) ->
          failwith "retired stream delivered")
        |> Or_error.ok_exn
      in
      Stream.push stream 1 |> Or_error.ok_exn;
      ignore
        (Scope.start
           peer
           ~f:(fun () -> ())
           ~on_result:(fun _ -> E.of_thunk (fun () -> calls := "peer" :: !calls))
         |> Or_error.ok_exn
         : Scope.Task.t);
      Eio.Fiber.yield ();
      (try Scope.cancel branch with
       | Failure message -> print_endline message);
      unregister ();
      unregister ();
      Scope.cancel branch;
      drain inbox;
      print_s
        [%sexp
          (List.rev !calls : string list)
        , (Result.is_error (Stream.push stream 2) : bool)
        , (Scope.stats root : Scope.Stats.t)];
      let replacement = Scope.child root ~name:"replacement" |> Or_error.ok_exn in
      Scope.cancel replacement;
      Scope.cancel root;
      print_s [%sexp (Scope.stats root : Scope.Stats.t)];
      Inbox.close inbox));
  [%expect
    {|
    cleanup
    ((cleanup peer) true ((scopes 2) (tasks 0) (cleanups 0)))
    ((scopes 0) (tasks 0) (cleanups 0))
    |}]
;;

let%expect_test "a descendant can unregister a pending ancestor cleanup during cancel" =
  Eio_mock.Backend.run (fun () ->
    Eio.Switch.run (fun sw ->
      let inbox = Inbox.create ~capacity:8 () in
      let root = Scope.Expert.create ~sw ~inbox ~max_tasks:8 in
      let child = Scope.child root ~name:"child" |> Or_error.ok_exn in
      let calls = ref [] in
      let unregister =
        Scope.on_cancel root (fun () -> calls := "unregistered" :: !calls)
        |> Or_error.ok_exn
      in
      ignore
        (Scope.on_cancel root (fun () -> calls := "root" :: !calls) |> Or_error.ok_exn
         : unit -> unit);
      ignore
        (Scope.on_cancel child (fun () ->
           unregister ();
           calls := "child" :: !calls)
         |> Or_error.ok_exn
         : unit -> unit);
      Scope.cancel root;
      print_s [%sexp (List.rev !calls : string list), (Scope.stats root : Scope.Stats.t)];
      Inbox.close inbox));
  [%expect {| ((child root) ((scopes 0) (tasks 0) (cleanups 0))) |}]
;;
