open Core
module E = Bonsai.Effect
module Wire = Gpuio_protocol.Desktop_wire

type t =
  { guard : Domain_guard.t
  ; schemes : Gpuio.Deep_link.Scheme.t list
  ; inbox : Desktop_inbox.t
  ; mutable schedule : (unit -> unit) -> unit
  ; mutable take : unit -> Wire.Response.t E.t
  ; mutable on_event : Gpuio.Desktop.Event.t -> unit E.t
  ; mutable ready : bool
  ; mutable closed : bool
  ; mutable scheduled : bool
  ; mutable taking : bool
  ; mutable delivering : bool
  ; mutable pending : bool
  ; mutable dropped : int64
  ; mutable error : Gpuio.Desktop.Error.t option
  }

let check t = Domain_guard.check t.guard

let create ~identity ~schedule ~take ~on_event =
  { guard = Domain_guard.create ()
  ; schemes = Gpuio.Desktop.Identity.schemes identity
  ; inbox = Desktop_inbox.create ()
  ; schedule
  ; take
  ; on_event
  ; ready = false
  ; closed = false
  ; scheduled = false
  ; taking = false
  ; delivering = false
  ; pending = false
  ; dropped = 0L
  ; error = None
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

and deliver t event =
  t.delivering <- true;
  E.Expert.handle
    (E.map (t.on_event event) ~f:(fun () ->
       check t;
       t.delivering <- false;
       wake t))

and received t response =
  check t;
  t.taking <- false;
  if not t.closed
  then (
    (match response with
     | Wire.Response.Links batch when Wire.Link_batch.valid batch ->
       (* Takes start only after the preceding batch has fully drained. Thus a
          valid native batch fits the same count/byte limits without dropping. *)
       List.iter batch.links ~f:(fun value ->
         match Desktop_inbox.push t.inbox value with
         | Queued -> ()
         | Too_large | Full | Closed -> failwith "desktop batch admission invariant");
       t.dropped <- batch.dropped
     | Failed error -> t.error <- Some (Gpuio.Desktop.Expert.error_of_wire error)
     | Links _ | Configured | Capabilities _ | Requested | Registered ->
       t.error <- Some Native_failure);
    wake t)

and pump t =
  if (not t.taking) && not t.delivering
  then (
    match Desktop_inbox.pop t.inbox with
    | Some input ->
      let event =
        match Gpuio.Deep_link.of_string ~schemes:t.schemes input with
        | Ok link -> Gpuio.Desktop.Event.Link link
        | Error reason -> Rejected_link { input; reason }
      in
      deliver t event
    | None ->
      if Int64.(t.dropped > 0L)
      then (
        let count = t.dropped in
        t.dropped <- 0L;
        deliver t (Overflow count))
      else (
        match t.error with
        | Some error ->
          t.error <- None;
          deliver t (Failed error)
        | None ->
          if t.pending
          then (
            t.pending <- false;
            t.taking <- true;
            E.Expert.handle (E.map (t.take ()) ~f:(received t)))))
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
    Desktop_inbox.ready t.inbox;
    available t)
;;

let retry = available

let close t =
  check t;
  t.closed <- true;
  Desktop_inbox.close t.inbox;
  t.pending <- false;
  t.dropped <- 0L;
  t.error <- None;
  t.schedule <- (fun _ -> ());
  t.take <- (fun () -> E.return (Wire.Response.Failed Closed));
  t.on_event <- (fun _ -> E.Ignore)
;;

let is_closed t =
  check t;
  t.closed
;;
