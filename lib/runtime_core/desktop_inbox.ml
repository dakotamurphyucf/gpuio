open Core

module Admission = struct
  type t =
    | Queued
    | Too_large
    | Full
    | Closed
  [@@deriving equal, sexp_of]
end

type phase =
  | Waiting
  | Ready
  | Closed

type t =
  { queue : string Queue.t
  ; mutable bytes : int
  ; mutable phase : phase
  }

let max_entries = Gpuio_protocol.Desktop_wire.max_links
let max_bytes = Gpuio_protocol.Desktop_wire.max_link_batch_bytes
let create () = { queue = Queue.create (); bytes = 0; phase = Waiting }

let push t value =
  match t.phase with
  | Closed -> Admission.Closed
  | Waiting | Ready ->
    let bytes = String.length value in
    if bytes > Gpuio.Deep_link.max_bytes
    then Admission.Too_large
    else if Queue.length t.queue >= max_entries || bytes > max_bytes - t.bytes
    then Admission.Full
    else (
      Queue.enqueue t.queue value;
      t.bytes <- t.bytes + bytes;
      Admission.Queued)
;;

let ready t =
  match t.phase with
  | Closed | Ready -> ()
  | Waiting -> t.phase <- Ready
;;

let pop t =
  match t.phase with
  | Closed | Waiting -> None
  | Ready ->
    let value = Queue.dequeue t.queue in
    Option.iter value ~f:(fun value -> t.bytes <- t.bytes - String.length value);
    value
;;

let close t =
  t.phase <- Closed;
  Queue.clear t.queue;
  t.bytes <- 0
;;

let length t = Queue.length t.queue
let bytes t = t.bytes
