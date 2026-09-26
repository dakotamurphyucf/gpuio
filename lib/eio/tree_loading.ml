open Core
module P = Gpuio.Tree_loading
module Request = P.Request
module Status = P.Status
module Page = P.Page
module Snapshot = P.Snapshot

exception Obsolete_request

type worker =
  { inbox : Request.t Eio.Stream.t
  ; mutable request : Request.t option
  ; mutable context : Eio.Cancel.t option
  ; mutable task : Scope.Task.t option
  }

type 'data t =
  { scope : Scope.t
  ; state : 'data P.t
  ; load : Request.t -> 'data Page.t Or_error.t
  ; on_change : 'data Snapshot.t -> unit Bonsai.Effect.t
  ; value : 'data Snapshot.t Bonsai.Cont.Expert.Var.t
  ; mutable published : 'data Snapshot.t
  ; mutable workers : worker list
  ; mutable closed : bool
  ; mutable unregister : unit -> unit
  }

let check t = Scope.Expert.check t.scope

let snapshot t =
  check t;
  P.snapshot t.state
;;

let value t =
  check t;
  Bonsai.Cont.Expert.Var.value t.value
;;

let notify t =
  if (not t.closed) && Scope.is_active t.scope
  then (
    let snapshot = P.snapshot t.state in
    if not (phys_equal snapshot t.published)
    then (
      t.published <- snapshot;
      Bonsai.Cont.Expert.Var.set t.value snapshot;
      Bonsai.Effect.Expert.handle (t.on_change snapshot)))
;;

let close t =
  check t;
  if not t.closed
  then (
    t.closed <- true;
    P.close t.state;
    t.published <- P.snapshot t.state;
    Bonsai.Cont.Expert.Var.set t.value t.published;
    List.iter t.workers ~f:(fun worker -> Option.iter worker.task ~f:Scope.Task.cancel);
    t.workers <- [];
    t.unregister ();
    t.unregister <- Fn.id)
;;

let synchronize t =
  List.iter t.workers ~f:(fun worker ->
    match worker.request, worker.context with
    | Some request, Some context when not (P.is_current t.state request) ->
      Eio.Cancel.cancel context Obsolete_request
    | None, _ | Some _, None | Some _, Some _ -> ())
;;

let rec work t worker =
  let request = Eio.Stream.take worker.inbox in
  let result =
    try
      Eio.Cancel.sub (fun context ->
        worker.context <- Some context;
        if t.closed || not (P.is_current t.state request)
        then None
        else Some (t.load request))
    with
    | Eio.Cancel.Cancelled _ as exn ->
      if P.is_current t.state request && not t.closed then raise exn else None
    | exn -> Some (Error (Error.of_exn exn))
  in
  worker.context <- None;
  (* Outside the request's cancellation context: cancelled producers still return
     their worker slot through the bounded UI inbox. The outer scoped task remains
     cancellable during backpressure or window/application shutdown. *)
  Scope.Expert.enqueue t.scope (fun () ->
    if not t.closed
    then (
      Option.iter result ~f:(function
        | Error error -> ignore (P.fail t.state request error : P.Completion.t)
        | Ok page -> ignore (P.complete t.state request page : P.Completion.t Or_error.t));
      worker.request <- None;
      pump t;
      notify t));
  work t worker

and allocate_worker t =
  let worker =
    { inbox = Eio.Stream.create 1; request = None; context = None; task = None }
  in
  let%map.Or_error task =
    Scope.start
      t.scope
      ~f:(fun () -> work t worker)
      ~on_result:(fun result ->
        Bonsai.Effect.of_thunk (fun () ->
          if not t.closed
          then (
            let error =
              match result with
              | Error error -> error
              | Ok () -> Error.of_string "tree loader worker stopped unexpectedly"
            in
            Option.iter worker.request ~f:(fun request ->
              ignore (P.fail t.state request error : P.Completion.t));
            t.workers
            <- List.filter t.workers ~f:(fun current -> not (phys_equal current worker));
            pump t;
            notify t)))
  in
  worker.task <- Some task;
  t.workers <- worker :: t.workers;
  worker

and pump t =
  if (not t.closed) && Scope.is_active t.scope
  then (
    synchronize t;
    let idle = List.find t.workers ~f:(fun worker -> Option.is_none worker.request) in
    if Option.is_some idle || List.length t.workers < P.max_running
    then (
      match P.take t.state with
      | None -> ()
      | Some request ->
        let worker =
          match idle with
          | Some worker -> Ok worker
          | None -> allocate_worker t
        in
        (match worker with
         | Error error -> ignore (P.fail t.state request error : P.Completion.t)
         | Ok worker ->
           worker.request <- Some request;
           Eio.Stream.add worker.inbox request);
        pump t))
;;

let create ?(on_change = fun _ -> Bonsai.Effect.Ignore) ~scope tree ~load =
  Scope.Expert.check scope;
  if not (Scope.is_active scope)
  then Or_error.error_string "tree loading scope closed"
  else (
    let state = P.create tree in
    let published = P.snapshot state in
    let t =
      { scope
      ; state
      ; load
      ; on_change
      ; value = Bonsai.Cont.Expert.Var.create published
      ; published
      ; workers = []
      ; closed = false
      ; unregister = Fn.id
      }
    in
    let%map.Or_error unregister = Scope.Expert.on_cancel scope (fun () -> close t) in
    t.unregister <- unregister;
    t)
;;

let open_controller t =
  check t;
  if t.closed || not (Scope.is_active t.scope)
  then Or_error.error_string "tree loader closed"
  else Ok ()
;;

let enqueue t id ~retry =
  let open Or_error.Let_syntax in
  let%bind () = open_controller t in
  let%map changed = (if retry then P.retry else P.request) t.state id in
  if changed
  then (
    pump t;
    notify t)
;;

let request t id = enqueue t id ~retry:false
let retry t id = enqueue t id ~retry:true

let change t f =
  check t;
  if not t.closed
  then (
    f t.state;
    pump t;
    notify t)
;;

let cancel t id = change t (fun state -> P.cancel state id)
let cancel_subtree t id = change t (fun state -> P.cancel_subtree state id)
let cancel_hidden t state = change t (fun model -> P.cancel_hidden model state)
let invalidate t id = change t (fun state -> P.invalidate state id)

let replace t tree f =
  let open Or_error.Let_syntax in
  let%bind () = open_controller t in
  let%map () = f t.state tree in
  pump t;
  notify t
;;

let update t tree = replace t tree P.update
let reset t tree = replace t tree P.reset
