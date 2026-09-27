open Core
module Job = Gpuio_agent_chat_runtime.Fixture_job
module Scope = Gpuio_eio.Scope
module Inbox = Gpuio_eio.Inbox
module E = Bonsai.Effect

let ok = Or_error.ok_exn
let drain inbox = List.iter (Inbox.take_turn inbox) ~f:(fun f -> f ())

let%expect_test "one running fixture and only the latest of 100 replacements" =
  Eio_mock.Backend.run (fun () ->
    Eio.Switch.run (fun sw ->
      let inbox = Inbox.create ~capacity:8 () in
      let scope = Scope.Expert.create ~sw ~inbox ~max_tasks:8 in
      let job = Job.create ~scope |> ok in
      let held, release = Eio.Promise.create () in
      let started = ref []
      and delivered = ref []
      and active = ref 0
      and peak = ref 0 in
      let submit number =
        Job.submit
          job
          ~f:(fun () ->
            started := number :: !started;
            incr active;
            peak := Int.max !peak !active;
            Exn.protect
              ~f:(fun () ->
                if number = 1 then Eio.Promise.await held;
                number)
              ~finally:(fun () -> decr active))
          ~on_result:(fun result ->
            E.of_thunk (fun () -> delivered := ok result :: !delivered))
        |> E.Expert.handle
      in
      submit 1;
      Eio.Fiber.yield ();
      for number = 2 to 101 do
        submit number
      done;
      Eio.Promise.resolve release ();
      while List.is_empty !delivered do
        Inbox.await inbox;
        drain inbox
      done;
      print_s
        [%sexp (List.rev !started : int list), (!delivered : int list), (!peak : int)];
      Scope.cancel scope;
      Inbox.close inbox));
  [%expect {| ((1 101) (101) 1) |}]
;;

let%expect_test "cancel retires delivery without admitting another concurrent build" =
  Eio_mock.Backend.run (fun () ->
    Eio.Switch.run (fun sw ->
      let inbox = Inbox.create ~capacity:8 () in
      let scope = Scope.Expert.create ~sw ~inbox ~max_tasks:8 in
      let job = Job.create ~scope |> ok in
      let held, release = Eio.Promise.create () in
      let delivered = ref [] in
      let done_ result = E.of_thunk (fun () -> delivered := ok result :: !delivered) in
      E.Expert.handle
        (Job.submit
           job
           ~f:(fun () ->
             Eio.Promise.await held;
             1)
           ~on_result:done_);
      Eio.Fiber.yield ();
      Job.cancel job;
      E.Expert.handle (Job.submit job ~f:(fun () -> 2) ~on_result:done_);
      Eio.Promise.resolve release ();
      while List.is_empty !delivered do
        Inbox.await inbox;
        drain inbox
      done;
      print_s [%sexp (!delivered : int list)];
      Scope.cancel scope;
      E.Expert.handle
        (Job.submit job ~f:(fun () -> failwith "closed fixture ran") ~on_result:done_);
      Eio.Fiber.yield ();
      drain inbox;
      print_s [%sexp (!delivered : int list)];
      Inbox.close inbox));
  [%expect
    {|
    (2)
    (2)
    |}]
;;

let%expect_test "closing a scope cancels the producer and discards queued replacement" =
  Eio_mock.Backend.run (fun () ->
    Eio.Switch.run (fun sw ->
      let inbox = Inbox.create ~capacity:8 () in
      let scope = Scope.Expert.create ~sw ~inbox ~max_tasks:8 in
      let job = Job.create ~scope |> ok in
      let held, _release = Eio.Promise.create () in
      let cancelled = ref false in
      let delivered = ref [] in
      let done_ result = E.of_thunk (fun () -> delivered := ok result :: !delivered) in
      E.Expert.handle
        (Job.submit
           job
           ~f:(fun () ->
             Exn.protect
               ~f:(fun () ->
                 Eio.Promise.await held;
                 1)
               ~finally:(fun () -> cancelled := true))
           ~on_result:done_);
      Eio.Fiber.yield ();
      E.Expert.handle
        (Job.submit job ~f:(fun () -> failwith "retired producer ran") ~on_result:done_);
      Scope.cancel scope;
      Eio.Fiber.yield ();
      drain inbox;
      print_s [%sexp (!cancelled : bool), (!delivered : int list)];
      Inbox.close inbox));
  [%expect {| (true ()) |}]
;;
