open Core
module Delivery = Gpuio_runtime_core.Desktop_delivery
module Wire = Gpuio_protocol.Desktop_wire
module E = Bonsai.Effect
module Event = Gpuio.Desktop.Event

let identity =
  Gpuio.Desktop.Identity.create
    ~identifier:"com.example"
    ~name:"Example"
    ~schemes:[ Gpuio.Deep_link.Scheme.of_string "example" |> Or_error.ok_exn ]
    ()
  |> Or_error.ok_exn
;;

type harness =
  { delivery : Delivery.t
  ; jobs : (unit -> unit) Queue.t
  ; requests : (Wire.Response.t -> unit) Queue.t
  ; events : Event.t Queue.t
  }

let create ?(handler = fun _ -> E.Ignore) () =
  let jobs = Queue.create () in
  let requests = Queue.create () in
  let events = Queue.create () in
  let delivery =
    Delivery.create
      ~identity
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
  let budget = ref 10_000 in
  while not (Queue.is_empty t.jobs) do
    decr budget;
    assert (!budget >= 0);
    Queue.dequeue_exn t.jobs ()
  done
;;

let respond t response = Queue.dequeue_exn t.requests response
let batch ?(dropped = 0L) links = Wire.Response.Links { links; dropped }

let names t =
  Queue.to_list t.events
  |> List.map ~f:(function
    | Event.Link link -> "link:" ^ Gpuio.Deep_link.route link
    | Rejected_link { reason; _ } ->
      "rejected:" ^ Sexp.to_string (Gpuio.Deep_link.Error.sexp_of_t reason)
    | Overflow count -> "overflow:" ^ Int64.to_string count
    | Failed error -> "failed:" ^ Sexp.to_string (Gpuio.Desktop.Error.sexp_of_t error))
;;

let%expect_test "readiness and repeated availability coalesce without idle polling" =
  let t = create () in
  for _ = 1 to 1000 do
    Delivery.available t.delivery
  done;
  drain t;
  assert (Queue.is_empty t.requests);
  Delivery.ready t.delivery;
  Delivery.ready t.delivery;
  drain t;
  assert (Queue.length t.requests = 1);
  for _ = 1 to 1000 do
    Delivery.available t.delivery
  done;
  drain t;
  assert (Queue.length t.requests = 1);
  respond t (batch ~dropped:2L [ "example://first"; "wrong://route"; "example://first" ]);
  drain t;
  print_s [%sexp (names t : string list)];
  (* One hint received during the take remains pending, but not 1000 requests. *)
  assert (Queue.length t.requests = 1);
  respond t (batch []);
  drain t;
  Delivery.ready t.delivery;
  drain t;
  assert (Queue.is_empty t.requests && Queue.is_empty t.jobs);
  [%expect {| (link:first rejected:Unsupported_scheme link:first overflow:2) |}]
;;

let%expect_test "slow application effects serialize delivery and bound retained work" =
  let completions = Queue.create () in
  let t =
    create
      ~handler:(fun _ ->
        E.Expert.of_fun ~f:(fun ~callback -> Queue.enqueue completions callback))
      ()
  in
  Delivery.ready t.delivery;
  drain t;
  respond t (batch [ "example://one"; "example://two" ]);
  drain t;
  assert (List.equal String.equal (names t) [ "link:one" ]);
  Delivery.available t.delivery;
  drain t;
  assert (Queue.is_empty t.requests);
  Queue.dequeue_exn completions ();
  drain t;
  assert (List.equal String.equal (names t) [ "link:one"; "link:two" ]);
  Delivery.close t.delivery;
  Queue.dequeue_exn completions ();
  Delivery.ready t.delivery;
  Delivery.retry t.delivery;
  drain t;
  assert (Queue.is_empty t.requests);
  assert (Delivery.is_closed t.delivery);
  [%expect {| |}]
;;

let%expect_test "late native responses and scheduled callbacks stop on close" =
  let t = create () in
  Delivery.ready t.delivery;
  drain t;
  Delivery.close t.delivery;
  respond t (batch [ "example://late" ]);
  drain t;
  assert (Queue.is_empty t.events);
  let t = create () in
  Delivery.ready t.delivery;
  drain t;
  respond t (batch [ "example://queued" ]);
  Delivery.close t.delivery;
  drain t;
  assert (Queue.is_empty t.events);
  [%expect {| |}]
;;

let%expect_test "failure is explicit and retry is requested by the application" =
  let t = create () in
  Delivery.ready t.delivery;
  drain t;
  respond t (Failed Busy);
  drain t;
  assert (Queue.is_empty t.requests);
  print_s [%sexp (names t : string list)];
  Delivery.retry t.delivery;
  Delivery.retry t.delivery;
  drain t;
  assert (Queue.length t.requests = 1);
  respond t (batch [ "example://recovered" ]);
  drain t;
  print_s [%sexp (names t : string list)];
  Delivery.retry t.delivery;
  drain t;
  respond t (batch [ String.make (Wire.max_link_bytes + 1) 'x' ]);
  drain t;
  print_s [%sexp (names t : string list)];
  [%expect
    {|
    (failed:Busy)
    (failed:Busy link:recovered)
    (failed:Busy link:recovered failed:Native_failure)
    |}]
;;
