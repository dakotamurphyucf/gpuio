open Core
module P = Gpuio.Table_paging
module Direction = P.Direction
module Boundary = P.Boundary
module Request = P.Request
module Status = P.Status
module Snapshot = P.Snapshot

module Page = struct
  type 'data t =
    { rows : (Gpuio.Table_data.Id.t * 'data) list
    ; next : Boundary.t
    }
end

exception Obsolete_request

type 'query worker =
  { inbox : 'query Request.t Eio.Stream.t
  ; mutable request : 'query Request.t option
  ; mutable context : Eio.Cancel.t option
  ; mutable task : Scope.Task.t option
  }

type ('query, 'data) t =
  { scope : Scope.t
  ; state : ('query, 'data) P.t
  ; load : 'query Request.t -> 'data Page.t Or_error.t
  ; on_change : ('query, 'data) Snapshot.t -> unit Bonsai.Effect.t
  ; value : ('query, 'data) Snapshot.t Bonsai.Cont.Expert.Var.t
  ; mutable before : 'query Request.t option
  ; mutable after : 'query Request.t option
  ; mutable workers : 'query worker list
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

let queued t = function
  | Direction.Before -> t.before
  | After -> t.after
;;

let set_queued t direction request =
  match direction with
  | Direction.Before -> t.before <- request
  | After -> t.after <- request
;;

let notify t =
  if (not t.closed) && Scope.is_active t.scope
  then (
    let snapshot = P.snapshot t.state in
    Bonsai.Cont.Expert.Var.set t.value snapshot;
    Bonsai.Effect.Expert.handle (t.on_change snapshot))
;;

let cancel_direction t direction =
  Option.iter (queued t direction) ~f:(fun request ->
    ignore (P.cancel t.state request : P.Completion.t));
  set_queued t direction None;
  List.iter t.workers ~f:(fun worker ->
    Option.iter worker.request ~f:(fun request ->
      if Direction.equal (Request.direction request) direction
      then ignore (P.cancel t.state request : P.Completion.t)))
;;

let close t =
  check t;
  if not t.closed
  then (
    t.closed <- true;
    cancel_direction t Before;
    cancel_direction t After;
    List.iter t.workers ~f:(fun worker -> Option.iter worker.task ~f:Scope.Task.cancel);
    t.workers <- [];
    Bonsai.Cont.Expert.Var.set t.value (P.snapshot t.state);
    t.unregister ();
    t.unregister <- Fn.id)
;;

let synchronize t =
  List.iter t.workers ~f:(fun worker ->
    match worker.request, worker.context with
    | Some request, Some context when not (P.Expert.is_current t.state request) ->
      Eio.Cancel.cancel context Obsolete_request
    | None, _ | Some _, None | Some _, Some _ -> ())
;;

let take_queued t =
  List.find_map [ Direction.Before; After ] ~f:(fun direction ->
    match queued t direction with
    | None -> None
    | Some request ->
      set_queued t direction None;
      Option.some_if (P.Expert.is_current t.state request) request)
;;

let rec work t worker =
  let request = Eio.Stream.take worker.inbox in
  let result =
    try
      Eio.Cancel.sub (fun context ->
        worker.context <- Some context;
        if t.closed || not (P.Expert.is_current t.state request)
        then None
        else Some (t.load request))
    with
    | Eio.Cancel.Cancelled _ as exn ->
      if P.Expert.is_current t.state request && not t.closed then raise exn else None
    | exn -> Some (Error (Error.of_exn exn))
  in
  worker.context <- None;
  (* Outside the producer's canceled context, but still inside the scoped worker.
     Returning the slot through the bounded inbox prevents cancellation races
     from launching more producers than the two worker slots. *)
  Scope.Expert.enqueue t.scope (fun () ->
    if not t.closed
    then (
      Option.iter result ~f:(function
        | Error error -> ignore (P.fail t.state request error : P.Completion.t)
        | Ok { Page.rows; next } ->
          ignore (P.complete t.state request ~rows ~next : P.Completion.t Or_error.t));
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
              | Ok () -> Error.of_string "table paging worker stopped unexpectedly"
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
    if Option.is_some idle || List.length t.workers < 2
    then (
      match take_queued t with
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

let create
      ?(on_change = fun _ -> Bonsai.Effect.Ignore)
      ~scope
      ~query
      data
      ~before
      ~after
      ~load
  =
  Scope.Expert.check scope;
  if not (Scope.is_active scope)
  then Or_error.error_string "table paging scope closed"
  else (
    let%bind.Or_error state = P.create ~query data ~before ~after in
    let t =
      { scope
      ; state
      ; load
      ; on_change
      ; value = Bonsai.Cont.Expert.Var.create (P.snapshot state)
      ; before = None
      ; after = None
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
  then Or_error.error_string "table paging controller closed"
  else Ok ()
;;

let start t direction ~retry =
  let%bind.Or_error () = open_controller t in
  let%map.Or_error request = (if retry then P.retry else P.request) t.state direction in
  Option.iter request ~f:(fun request ->
    set_queued t direction (Some request);
    pump t;
    notify t)
;;

let request t direction = start t direction ~retry:false
let retry t direction = start t direction ~retry:true

let cancel t direction =
  check t;
  if not t.closed
  then (
    cancel_direction t direction;
    pump t;
    notify t)
;;

let reset t ~query data ~before ~after =
  let%bind.Or_error () = open_controller t in
  let%map.Or_error () = P.reset t.state ~query data ~before ~after in
  t.before <- None;
  t.after <- None;
  pump t;
  notify t
;;

let set t ~key ~data =
  let%bind.Or_error () = open_controller t in
  let%map.Or_error () = P.set t.state ~key ~data in
  notify t
;;

let append t rows =
  let%bind.Or_error () = open_controller t in
  let%map.Or_error () = P.append t.state rows in
  notify t
;;

let controls t =
  check t;
  let current ~generation =
    (not t.closed)
    && Scope.is_active t.scope
    && Int64.equal generation (P.generation t.state)
  in
  let run operation ~generation direction =
    Bonsai.Effect.of_thunk (fun () ->
      check t;
      if current ~generation
      then (
        match operation t direction with
        | Ok () -> ()
        | Error error ->
          (match P.status t.state direction with
           | Failed _ -> ()
           | Ready | Loading | End -> Error.raise error)))
  in
  Gpuio_bonsai.Table.Paging.create
    ~request:(run request)
    ~retry:(run retry)
    ~cancel:(fun ~generation direction ->
      Bonsai.Effect.of_thunk (fun () ->
        check t;
        if current ~generation then cancel t direction))
;;
