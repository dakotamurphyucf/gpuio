open Core

type 'a t =
  { scope : Scope.t
  ; capacity : int
  ; on_batch : 'a list -> unit Bonsai.Effect.t
  ; mutable pending : 'a list
  ; mutable count : int
  ; mutable scheduled : bool
  ; mutable closed : bool
  ; mutable unregister : unit -> unit
  }

let close t =
  Scope.Expert.check t.scope;
  t.closed <- true;
  t.pending <- [];
  t.count <- 0;
  t.unregister ();
  t.unregister <- Fn.id
;;

let create ~scope ~capacity ~on_batch =
  Scope.Expert.check scope;
  if capacity < 1 || capacity > 4096
  then Or_error.error_string "stream capacity must be in 1..4096"
  else if not (Scope.is_active scope)
  then Or_error.error_string "stream scope closed"
  else (
    let t =
      { scope
      ; capacity
      ; on_batch
      ; pending = []
      ; count = 0
      ; scheduled = false
      ; closed = false
      ; unregister = Fn.id
      }
    in
    let open Or_error.Let_syntax in
    let%map unregister = Scope.Expert.on_cancel scope (fun () -> close t) in
    t.unregister <- unregister;
    t)
;;

let push t value =
  Scope.Expert.check t.scope;
  if t.closed || not (Scope.is_active t.scope)
  then (
    close t;
    Or_error.error_string "stream closed")
  else if t.count >= t.capacity
  then Or_error.error_string "stream batch capacity reached"
  else (
    let scheduled =
      t.scheduled
      || Scope.Expert.try_enqueue t.scope (fun () ->
        let batch = List.rev t.pending in
        t.pending <- [];
        t.count <- 0;
        t.scheduled <- false;
        if not t.closed then Bonsai.Effect.Expert.handle (t.on_batch batch))
    in
    if not scheduled
    then Or_error.error_string "scheduler queue capacity reached"
    else (
      t.scheduled <- true;
      t.pending <- value :: t.pending;
      t.count <- t.count + 1;
      Ok ()))
;;
