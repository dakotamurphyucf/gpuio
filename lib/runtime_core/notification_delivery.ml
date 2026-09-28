open Core
module E = Bonsai.Effect
module Wire = Gpuio_protocol.Notification_wire
module N = Gpuio.Notification

type t =
  { guard : Domain_guard.t
  ; events : N.Event.t Queue.t
  ; mutable schedule : (unit -> unit) -> unit
  ; mutable take : unit -> Wire.Response.t E.t
  ; mutable on_event : N.Event.t -> unit E.t
  ; mutable ready : bool
  ; mutable closed : bool
  ; mutable scheduled : bool
  ; mutable taking : bool
  ; mutable delivering : bool
  ; mutable pending : bool
  }

let check t = Domain_guard.check t.guard

let create ~schedule ~take ~on_event =
  { guard = Domain_guard.create ()
  ; events = Queue.create ()
  ; schedule
  ; take
  ; on_event
  ; ready = false
  ; closed = false
  ; scheduled = false
  ; taking = false
  ; delivering = false
  ; pending = false
  }
;;

let rec wake t =
  if t.ready && (not t.closed) && not t.scheduled
  then (
    t.scheduled <- true;
    t.schedule (fun () ->
      check t;
      t.scheduled <- false;
      if not t.closed then pump t))

and received t response =
  check t;
  t.taking <- false;
  if not t.closed
  then (
    (match response with
     | Wire.Response.Events events when Wire.Response.valid response ->
       List.iter events ~f:(fun event ->
         Queue.enqueue t.events (N.Expert.event_of_wire event |> Or_error.ok_exn))
     | Failed error -> Queue.enqueue t.events (Failed (N.Expert.error_of_wire error))
     | Events _
     | Capabilities _
     | Authorization _
     | Posted _
     | Replaced
     | Dismiss_requested
     | Closed -> Queue.enqueue t.events (Failed Native_failure));
    wake t)

and pump t =
  if (not t.taking) && not t.delivering
  then (
    match Queue.dequeue t.events with
    | Some event ->
      t.delivering <- true;
      E.Expert.handle
        (E.map (t.on_event event) ~f:(fun () ->
           check t;
           t.delivering <- false;
           wake t))
    | None ->
      if t.pending
      then (
        t.pending <- false;
        t.taking <- true;
        E.Expert.handle (E.map (t.take ()) ~f:(received t))))
;;

let available t =
  check t;
  if not t.closed
  then (
    t.pending <- true;
    wake t)
;;

let ready t =
  check t;
  if (not t.ready) && not t.closed
  then (
    t.ready <- true;
    available t)
;;

let retry = available

let close t =
  check t;
  t.closed <- true;
  Queue.clear t.events;
  t.pending <- false;
  t.schedule <- (fun _ -> ());
  t.take <- (fun () -> E.return (Wire.Response.Failed Closed));
  t.on_event <- (fun _ -> E.Ignore)
;;

let is_closed t =
  check t;
  t.closed
;;
