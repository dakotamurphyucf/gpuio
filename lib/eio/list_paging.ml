open Core
module P = Gpuio.List_paging
module Direction = P.Direction
module Boundary = P.Boundary
module Request = P.Request
module Status = P.Status

module Page = struct
  type ('key, 'data) t =
    { rows : ('key * 'data) list
    ; next : Boundary.t
    }
end

module Snapshot = struct
  type ('key, 'data, 'cmp) t =
    { items : ('key, 'data, 'cmp) Gpuio.List_collection.t
    ; before : Status.t
    ; after : Status.t
    }
end

type pending =
  { request : Request.t
  ; task : Scope.Task.t
  }

type ('key, 'data, 'cmp) t =
  { scope : Scope.t
  ; state : ('key, 'data, 'cmp) P.t
  ; load : Request.t -> ('key, 'data) Page.t Or_error.t
  ; on_change : ('key, 'data, 'cmp) Snapshot.t -> unit Bonsai.Effect.t
  ; mutable before : pending option
  ; mutable after : pending option
  ; mutable closed : bool
  ; mutable unregister : unit -> unit
  }

let check t = Scope.Expert.check t.scope

let pending t = function
  | Direction.Before -> t.before
  | After -> t.after
;;

let set_pending t direction pending =
  match direction with
  | Direction.Before -> t.before <- pending
  | After -> t.after <- pending
;;

let cancel_pending t direction =
  match pending t direction with
  | None -> false
  | Some pending ->
    set_pending t direction None;
    Scope.Task.cancel pending.task;
    ignore (P.cancel t.state pending.request : P.Completion.t);
    true
;;

let close t =
  check t;
  if not t.closed
  then (
    t.closed <- true;
    ignore (cancel_pending t Before : bool);
    ignore (cancel_pending t After : bool);
    t.unregister ();
    t.unregister <- Fn.id)
;;

let create ~scope items ~before ~after ~load ~on_change =
  Scope.Expert.check scope;
  if not (Scope.is_active scope)
  then Or_error.error_string "list paging scope closed"
  else (
    let t =
      { scope
      ; state = P.create items ~before ~after
      ; load
      ; on_change
      ; before = None
      ; after = None
      ; closed = false
      ; unregister = Fn.id
      }
    in
    let open Or_error.Let_syntax in
    let%map unregister = Scope.Expert.on_cancel scope (fun () -> close t) in
    t.unregister <- unregister;
    t)
;;

let items t =
  check t;
  P.items t.state
;;

let status t direction =
  check t;
  P.status t.state direction
;;

let snapshot t =
  check t;
  { Snapshot.items = P.items t.state
  ; before = P.status t.state Before
  ; after = P.status t.state After
  }
;;

let notify t =
  if (not t.closed) && Scope.is_active t.scope
  then Bonsai.Effect.Expert.handle (t.on_change (snapshot t))
;;

let start t direction ~retry =
  check t;
  if t.closed || not (Scope.is_active t.scope)
  then Or_error.error_string "list paging controller closed"
  else
    let open Or_error.Let_syntax in
    let%bind request = (if retry then P.retry else P.request) t.state direction in
    match request with
    | None -> Ok ()
    | Some request ->
      let task =
        Scope.start
          t.scope
          ~f:(fun () -> t.load request)
          ~on_result:(fun result ->
            Bonsai.Effect.of_thunk (fun () ->
              let result = Or_error.join result in
              let completion =
                match result with
                | Error error -> P.fail t.state request error
                | Ok { Page.rows; next } ->
                  (match P.complete t.state request ~rows ~next with
                   | Ok completion -> completion
                   | Error _ -> P.Completion.Applied)
              in
              match completion with
              | Obsolete -> ()
              | Applied ->
                set_pending t direction None;
                notify t))
      in
      (match task with
       | Error error ->
         ignore (P.fail t.state request error : P.Completion.t);
         notify t;
         Error error
       | Ok task ->
         set_pending t direction (Some { request; task });
         notify t;
         Ok ())
;;

let request t direction = start t direction ~retry:false
let retry t direction = start t direction ~retry:true

let cancel t direction =
  check t;
  if cancel_pending t direction then notify t
;;

let reset t items ~before ~after =
  check t;
  if t.closed || not (Scope.is_active t.scope)
  then Or_error.error_string "list paging controller closed"
  else
    let open Or_error.Let_syntax in
    let%map () = P.reset t.state items ~before ~after in
    ignore (cancel_pending t Before : bool);
    ignore (cancel_pending t After : bool);
    notify t
;;

let set t ~key ~data =
  check t;
  if t.closed || not (Scope.is_active t.scope)
  then Or_error.error_string "list paging controller closed"
  else
    let open Or_error.Let_syntax in
    let%map () = P.set t.state ~key ~data in
    notify t
;;
