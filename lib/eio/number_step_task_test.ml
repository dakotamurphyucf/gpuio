open Core
module N = Gpuio.Number_input
module E = Bonsai.Effect
module Task = Number_step_task

let ok = Or_error.ok_exn

let snapshot =
  N.Expert.snapshot_of_wire
    ~window:(Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok)
    ~node:(Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok)
    { revision = 1L
    ; domain =
        Gpuio.Numeric.Expert.to_wire
          (Gpuio.Numeric.Domain.create ~min:0. ~max:100. ~step:1. |> ok)
    ; draft = "12"
    ; committed = Number 12.
    ; selection = { anchor = 2L; head = 2L }
    ; composition = None
    ; focused = true
    }
  |> ok
;;

let proposal = N.Step_resolution.Apply (N.Value.of_float 12. |> ok)

type harness =
  { scope : Scope.t
  ; inbox : Inbox.t
  ; mutable declines : int
  ; commands :
      (N.Step_resolution.t * ((N.Snapshot.t, N.Command_error.t) Result.t -> unit)) Queue.t
  ; reports : (N.Snapshot.t, Task.Error.t) Result.t Queue.t
  }

let with_harness ?(max_tasks = 8) f =
  Eio_main.run (fun _ ->
    Eio.Switch.run (fun sw ->
      let inbox = Inbox.create ~capacity:1 () in
      let scope = Scope.Expert.create ~sw ~inbox ~max_tasks in
      let h =
        { scope
        ; inbox
        ; declines = 0
        ; commands = Queue.create ()
        ; reports = Queue.create ()
        }
      in
      Exn.protect
        ~f:(fun () -> f h)
        ~finally:(fun () ->
          Scope.cancel scope;
          Inbox.close inbox)))
;;

let pump h =
  Eio.Fiber.yield ();
  List.iter (Inbox.take_turn h.inbox) ~f:(fun f -> f ())
;;

let start ?on_result h f =
  Task.start
    ~scope:h.scope
    ~f
    ~decline:(fun () -> h.declines <- h.declines + 1)
    ~resolve:(fun decision ->
      E.Expert.of_fun ~f:(fun ~callback -> Queue.enqueue h.commands (decision, callback)))
    ~on_result:
      (Option.value on_result ~default:(fun result ->
         E.of_thunk (fun () -> Queue.enqueue h.reports result)))
;;

let clean h =
  Eio.Fiber.yield ();
  let stats = Scope.stats h.scope in
  assert (stats.scopes = 1 && stats.tasks = 0 && stats.cleanups = 0)
;;

let reply h result =
  let _, complete = Queue.dequeue_exn h.commands in
  complete result
;;

let%expect_test "normal completion releases its scope before publication" =
  with_harness (fun h ->
    let task = start h (fun () -> proposal) |> ok in
    pump h;
    assert (not (Task.is_finished task));
    assert (Queue.length h.commands = 1);
    reply h (Ok snapshot);
    assert (Task.is_finished task);
    assert (Queue.length h.reports = 1 && h.declines = 0);
    clean h;
    Task.cancel task;
    assert (h.declines = 0));
  print_endline "one dispatch, one completion, no retained child or cleanup";
  [%expect {| one dispatch, one completion, no retained child or cleanup |}]
;;

let%expect_test
    "cancellation fences suspended work, queued completion and late native reply"
  =
  with_harness (fun h ->
    let wait, _ = Eio.Promise.create () in
    let task = start h (fun () -> Eio.Promise.await wait) |> ok in
    Task.cancel task;
    Task.cancel task;
    pump h;
    assert (h.declines = 1 && Queue.is_empty h.commands && Queue.is_empty h.reports);
    assert (Task.is_finished task);
    clean h;
    let task = start h (fun () -> proposal) |> ok in
    Eio.Fiber.yield ();
    assert (Inbox.length h.inbox = 1);
    Task.cancel task;
    pump h;
    assert (h.declines = 2 && Queue.is_empty h.commands);
    clean h;
    let task = start h (fun () -> proposal) |> ok in
    pump h;
    assert (Queue.length h.commands = 1);
    Task.cancel task;
    reply h (Ok snapshot);
    assert (h.declines = 3 && Queue.is_empty h.reports);
    clean h);
  print_endline "cancel is idempotent; no late apply or completion after cancellation";
  [%expect {| cancel is idempotent; no late apply or completion after cancellation |}]
;;

let%expect_test "work exceptions and native refusal decline without leaking resources" =
  with_harness (fun h ->
    let task = start h (fun () -> failwith "work failed") |> ok in
    pump h;
    assert (Task.is_finished task && h.declines = 1 && Queue.is_empty h.commands);
    (match Queue.dequeue_exn h.reports with
     | Error (Work_failed _) -> ()
     | _ -> assert false);
    clean h;
    let task = start h (fun () -> proposal) |> ok in
    pump h;
    reply h (Error Busy);
    assert (Task.is_finished task && h.declines = 2);
    (match Queue.dequeue_exn h.reports with
     | Error (Resolution_failed Busy) -> ()
     | _ -> assert false);
    clean h;
    let task =
      start
        h
        (fun () -> proposal)
        ~on_result:(fun _ -> E.of_thunk (fun () -> failwith "callback failed"))
      |> ok
    in
    pump h;
    assert (Result.is_error (Or_error.try_with (fun () -> reply h (Ok snapshot))));
    assert (Task.is_finished task);
    clean h);
  print_endline
    "failures reported, cleanup precedes user callback, saturated resolution declines";
  [%expect
    {| failures reported, cleanup precedes user callback, saturated resolution declines |}]
;;

let%expect_test "task saturation and producer inbox backpressure remain cancellable" =
  with_harness ~max_tasks:1 (fun h ->
    let wait, _ = Eio.Promise.create () in
    let blocker =
      Scope.start
        h.scope
        ~f:(fun () -> Eio.Promise.await wait)
        ~on_result:(fun _ -> E.Ignore)
      |> ok
    in
    assert (Result.is_error (start h (fun () -> proposal)));
    assert (h.declines = 1);
    let stats = Scope.stats h.scope in
    assert (stats.scopes = 1 && stats.cleanups = 0);
    Scope.Task.cancel blocker;
    clean h;
    assert (Inbox.try_push h.inbox ignore);
    let task = start h (fun () -> proposal) |> ok in
    Eio.Fiber.yield ();
    Task.cancel task;
    pump h;
    assert (h.declines = 2 && Queue.is_empty h.commands && Queue.is_empty h.reports);
    clean h);
  print_endline
    "start failure declines; inbox-blocked completion cannot outlive cancellation";
  [%expect
    {| start failure declines; inbox-blocked completion cannot outlive cancellation |}]
;;

let%expect_test "parent cancellation retires the managed child without user callbacks" =
  with_harness (fun h ->
    let task = start h (fun () -> proposal) |> ok in
    Eio.Fiber.yield ();
    Scope.cancel h.scope;
    pump h;
    assert (Task.is_finished task && h.declines = 1);
    assert (Queue.is_empty h.commands && Queue.is_empty h.reports);
    let stats = Scope.stats h.scope in
    assert (stats.scopes = 0 && stats.tasks = 0 && stats.cleanups = 0));
  print_endline "parent cancellation declines once and suppresses buffered result";
  [%expect {| parent cancellation declines once and suppresses buffered result |}]
;;
