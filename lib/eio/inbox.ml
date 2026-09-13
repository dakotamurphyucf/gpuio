open Core
module Guard = Gpuio_runtime_core.Domain_guard

type t =
  { guard : Guard.t
  ; capacity : int
  ; jobs : (unit -> unit) Queue.t
  ; changed : Eio.Condition.t
  ; space : Eio.Condition.t
  ; mutable notified : bool
  ; mutable closed : bool
  }

let create ~capacity () =
  if capacity < 1 then invalid_arg "inbox capacity";
  { guard = Guard.create ()
  ; capacity
  ; jobs = Queue.create ()
  ; changed = Eio.Condition.create ()
  ; space = Eio.Condition.create ()
  ; notified = false
  ; closed = false
  }
;;

let wake t =
  Guard.check t.guard;
  t.notified <- true;
  Eio.Condition.broadcast t.changed
;;

let try_push t job =
  Guard.check t.guard;
  if t.closed || Queue.length t.jobs >= t.capacity
  then false
  else (
    Queue.enqueue t.jobs job;
    wake t;
    true)
;;

let push t job =
  Guard.check t.guard;
  while (not t.closed) && Queue.length t.jobs >= t.capacity do
    Eio.Condition.await_no_mutex t.space
  done;
  if not t.closed
  then (
    Queue.enqueue t.jobs job;
    wake t)
;;

let take_turn t =
  Guard.check t.guard;
  t.notified <- false;
  let jobs = Queue.to_list t.jobs in
  Queue.clear t.jobs;
  Eio.Condition.broadcast t.space;
  jobs
;;

let await t =
  Guard.check t.guard;
  if (not t.notified) && Queue.is_empty t.jobs && not t.closed
  then Eio.Condition.await_no_mutex t.changed
;;

let close t =
  Guard.check t.guard;
  t.closed <- true;
  Queue.clear t.jobs;
  wake t;
  Eio.Condition.broadcast t.space
;;
