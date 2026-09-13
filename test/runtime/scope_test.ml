open Core
module Scope = Gpuio_eio.Scope
module Inbox = Gpuio_eio.Inbox
module Stream = Gpuio_eio.Stream
module E = Bonsai.Effect

let drain inbox = List.iter (Inbox.take_turn inbox) ~f:(fun f -> f ())

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
