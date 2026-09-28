open Core
module D = Gpuio_runtime_core.Notification_delivery
module W = Gpuio_protocol.Notification_wire
module E = Bonsai.Effect
module N = Gpuio.Notification

type harness =
  { delivery : D.t
  ; jobs : (unit -> unit) Queue.t
  ; requests : (W.Response.t -> unit) Queue.t
  ; events : N.Event.t Queue.t
  }

let create ?(handler = fun _ -> E.Ignore) () =
  let jobs = Queue.create ()
  and requests = Queue.create ()
  and events = Queue.create () in
  let delivery =
    D.create
      ~schedule:(Queue.enqueue jobs)
      ~take:(fun () ->
        E.Expert.of_fun ~f:(fun ~callback -> Queue.enqueue requests callback))
      ~on_event:(fun event ->
        Queue.enqueue events event;
        handler event)
  in
  { delivery; jobs; requests; events }
;;

let drain t =
  let budget = ref 1000 in
  while not (Queue.is_empty t.jobs) do
    decr budget;
    assert (!budget >= 0);
    Queue.dequeue_exn t.jobs ()
  done
;;

let event id = W.Event.Activated { id; tag = "same-tag" }
let respond t value = Queue.dequeue_exn t.requests value

let%expect_test "readiness and hints coalesce, preserving original receipt identity" =
  let t = create () in
  for _ = 1 to 1000 do
    D.available t.delivery
  done;
  drain t;
  assert (Queue.is_empty t.requests);
  D.ready t.delivery;
  D.ready t.delivery;
  drain t;
  assert (Queue.length t.requests = 1);
  for _ = 1 to 1000 do
    D.available t.delivery
  done;
  drain t;
  assert (Queue.length t.requests = 1);
  respond t (Events [ event 1L; event 2L ]);
  drain t;
  assert (Queue.length t.events = 2 && Queue.length t.requests = 1);
  let receipts =
    Queue.to_list t.events
    |> List.filter_map ~f:(function
      | N.Event.Activated r -> Some r
      | Action _ | Closed _ | Failed _ -> None)
  in
  assert (List.length receipts = 2);
  assert (not (N.Receipt.equal (List.nth_exn receipts 0) (List.nth_exn receipts 1)));
  respond t (Events []);
  drain t;
  D.ready t.delivery;
  drain t;
  assert (Queue.is_empty t.requests && Queue.is_empty t.jobs);
  [%expect {| |}]
;;

let%expect_test "slow effects backpressure intake and close suppresses queued delivery" =
  let completions = Queue.create () in
  let t =
    create
      ~handler:(fun _ ->
        E.Expert.of_fun ~f:(fun ~callback -> Queue.enqueue completions callback))
      ()
  in
  D.ready t.delivery;
  drain t;
  respond t (Events (List.init W.max_live ~f:(fun i -> event (Int64.of_int (i + 1)))));
  drain t;
  assert (Queue.length t.events = 1);
  D.available t.delivery;
  drain t;
  assert (Queue.is_empty t.requests);
  Queue.dequeue_exn completions ();
  drain t;
  assert (Queue.length t.events = 2);
  D.close t.delivery;
  Queue.dequeue_exn completions ();
  D.retry t.delivery;
  drain t;
  assert (Queue.length t.events = 2 && Queue.is_empty t.requests);
  assert (D.is_closed t.delivery);
  [%expect {| |}]
;;

let%expect_test
    "malformed batch is atomic, errors need explicit retry, late response is ignored"
  =
  let t = create () in
  D.ready t.delivery;
  drain t;
  respond t (Events [ event 1L; event 0L ]);
  drain t;
  print_s [%sexp (Queue.to_list t.events : N.Event.t list)];
  assert (Queue.is_empty t.requests);
  D.retry t.delivery;
  drain t;
  respond t (Failed Unavailable);
  drain t;
  print_s [%sexp (Queue.to_list t.events : N.Event.t list)];
  assert (Queue.is_empty t.requests);
  D.retry t.delivery;
  drain t;
  D.close t.delivery;
  respond t (Events [ event 3L ]);
  drain t;
  assert (Queue.length t.events = 2 && Queue.is_empty t.requests);
  [%expect
    {|
    ((Failed Native_failure))
    ((Failed Native_failure) (Failed Unavailable))
    |}]
;;
