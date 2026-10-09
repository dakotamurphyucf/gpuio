open Core
module C = Gpuio.List_collection
module B = Bonsai.Cont
module E = Bonsai.Effect

module Status = struct
  type t =
    | Ready
    | Debouncing
    | Loading
    | Failed of Error.t
    | Cancelled
    | Closed
  [@@deriving sexp_of]
end

module Request = struct
  type ('key, 'data, 'cmp) t =
    { items : ('key, 'data, 'cmp) C.t
    ; query : string
    ; epoch : Gpuio.Key.t
    }

  let items t = t.items
  let query t = t.query
  let epoch t = t.epoch
end

module Page = struct
  type ('key, 'data) t =
    { upsert : ('key * 'data) list
    ; visible : 'key list
    }
end

module Snapshot = struct
  type ('key, 'data, 'cmp) t =
    { items : ('key, 'data, 'cmp) C.t
    ; query : string
    ; epoch : Gpuio.Key.t
    ; status : Status.t
    ; visible : 'key list
    ; stale : bool
    }

  let items t = t.items
  let source_id t = C.Identity.source_id (C.identity t.items)
  let query t = t.query
  let epoch t = t.epoch
  let status t = t.status
  let visible t = t.visible
  let is_stale t = t.stale

  let is_busy t =
    match t.status with
    | Debouncing | Loading -> true
    | Ready | Failed _ | Cancelled | Closed -> false
  ;;
end

type ('key, 'data, 'cmp) pending =
  { request : ('key, 'data, 'cmp) Request.t
  ; task : Scope.Task.t
  }

type ('key, 'data, 'cmp) t =
  { scope : Scope.t
  ; sleep : float -> unit
  ; debounce : float
  ; max_loaded : int
  ; search : ('key, 'data, 'cmp) Request.t -> ('key, 'data) Page.t Or_error.t
  ; on_change : ('key, 'data, 'cmp) Snapshot.t -> unit E.t
  ; value : ('key, 'data, 'cmp) Snapshot.t B.Expert.Var.t
  ; mutable snapshot : ('key, 'data, 'cmp) Snapshot.t
  ; mutable pending : ('key, 'data, 'cmp) pending option
  ; mutable serial : int64
  ; mutable closed : bool
  ; mutable unregister : unit -> unit
  }

let check t = Scope.Expert.check t.scope

let snapshot t =
  check t;
  t.snapshot
;;

let value t =
  check t;
  B.Expert.Var.value t.value
;;

let active t = (not t.closed) && Scope.is_active t.scope

let notify t =
  B.Expert.Var.set t.value t.snapshot;
  if active t then E.Expert.handle (t.on_change t.snapshot)
;;

let cancel_pending t =
  let pending = t.pending in
  t.pending <- None;
  Option.iter pending ~f:(fun p -> Scope.Task.cancel p.task)
;;

let epoch items serial =
  Gpuio.Key.of_string_exn
    (Gpuio.Key.to_string (C.Source_id.to_key (C.Identity.source_id (C.identity items)))
     ^ ":query:"
     ^ Int64.to_string serial)
;;

let advance t =
  if Int64.equal t.serial Int64.max_value then failwith "list search epoch exhausted";
  t.serial <- Int64.succ t.serial;
  { t.snapshot with Snapshot.epoch = epoch t.snapshot.items t.serial }
;;

let close t =
  check t;
  if not t.closed
  then (
    t.closed <- true;
    cancel_pending t;
    t.unregister ();
    t.unregister <- Fn.id;
    t.snapshot <- { (advance t) with status = Closed; stale = true };
    notify t)
;;

let create
      ~scope
      ~clock
      ?(debounce = Time_ns.Span.of_ms 150.)
      ?(max_loaded = 100_000)
      ?(on_change = fun _ -> E.Ignore)
      items
      ~search
  =
  Scope.Expert.check scope;
  let debounce = Time_ns.Span.to_sec debounce in
  if not (Scope.is_active scope)
  then Or_error.error_string "list search scope closed"
  else if Float.(debounce < 0. || debounce > 60.)
  then Or_error.error_string "search debounce must be between zero and sixty seconds"
  else if max_loaded < 1 || max_loaded > 1_000_000 || C.length items > max_loaded
  then Or_error.error_string "list search loaded-record limit exceeded or invalid"
  else (
    let snapshot =
      { Snapshot.items
      ; query = ""
      ; epoch = epoch items 0L
      ; status = Ready
      ; visible = C.keys items
      ; stale = false
      }
    in
    let t =
      { scope
      ; sleep = Eio.Time.Mono.sleep clock
      ; debounce
      ; max_loaded
      ; search
      ; on_change
      ; value = B.Expert.Var.create snapshot
      ; snapshot
      ; pending = None
      ; serial = 0L
      ; closed = false
      ; unregister = Fn.id
      }
    in
    let%map.Or_error unregister = Scope.on_cancel scope (fun () -> close t) in
    t.unregister <- unregister;
    t)
;;

let current t request =
  active t && Option.exists t.pending ~f:(fun p -> phys_equal p.request request)
;;

let merge t request (page : (_, _) Page.t) =
  if List.length page.upsert > t.max_loaded || List.length page.visible > t.max_loaded
  then Or_error.error_string "search result metadata exceeds loaded-record limit"
  else (
    let items = t.snapshot.items in
    let comparator = C.Identity.comparator (C.identity items) in
    let open Or_error.Let_syntax in
    let%bind updates = Map.Using_comparator.of_alist_or_error ~comparator page.upsert in
    let added =
      Map.fold updates ~init:0 ~f:(fun ~key ~data:_ n ->
        if Option.is_none (C.find items key) then n + 1 else n)
    in
    if C.length items + added > t.max_loaded
    then Or_error.error_string "search result would exceed loaded-record limit"
    else (
      (* Only explicit non-search-affecting point updates can occur without
         cancelling this request. Preserve those newer application values. *)
      let changed =
        C.fold_changed_values
          items
          ~previous:request.Request.items
          ~init:(Set.Using_comparator.empty ~comparator)
          ~f:(fun keys key -> Set.add keys key)
      in
      let items, appended =
        List.fold page.upsert ~init:(items, []) ~f:(fun (items, appended) (key, data) ->
          match C.find items key with
          | None -> items, (key, data) :: appended
          | Some _ when Set.mem changed key -> items, appended
          | Some _ -> C.set items ~key ~data |> Or_error.ok_exn, appended)
      in
      let%bind items =
        if List.is_empty appended
        then Ok items
        else C.splice items ~at:(C.length items) ~remove:0 (List.rev appended)
      in
      let%map (_ : (_, _) Gpuio.List_selection.Catalog.t) =
        Gpuio.List_selection.Catalog.create (C.identity items) ~visible:page.visible ()
      in
      items, page.visible))
;;

let complete t request result =
  if current t request
  then (
    let result = Or_error.join result |> Or_error.bind ~f:(merge t request) in
    t.pending <- None;
    t.snapshot
    <- (match result with
        | Error error -> { t.snapshot with status = Failed error; stale = true }
        | Ok (items, visible) ->
          { t.snapshot with items; visible; status = Ready; stale = false });
    notify t)
;;

let start t =
  cancel_pending t;
  t.snapshot
  <- { (advance t) with
       status = (if Float.(t.debounce > 0.) then Debouncing else Loading)
     ; stale = true
     };
  let request =
    { Request.items = t.snapshot.items
    ; query = t.snapshot.query
    ; epoch = t.snapshot.epoch
    }
  in
  match
    Scope.start
      t.scope
      ~f:(fun () ->
        if Float.(t.debounce > 0.)
        then (
          t.sleep t.debounce;
          Scope.Expert.enqueue t.scope (fun () ->
            if current t request
            then (
              t.snapshot <- { t.snapshot with status = Loading };
              notify t)));
        t.search request)
      ~on_result:(fun result -> E.of_thunk (fun () -> complete t request result))
  with
  | Error error ->
    t.snapshot <- { t.snapshot with status = Failed error };
    notify t;
    Error error
  | Ok task ->
    t.pending <- Some { request; task };
    notify t;
    Ok ()
;;

let refresh t =
  check t;
  if not (active t)
  then Or_error.error_string "list search controller closed"
  else start t
;;

let set_query t ~source query =
  check t;
  if not (active t)
  then Or_error.error_string "list search controller closed"
  else if not (C.Source_id.equal source (Snapshot.source_id t.snapshot))
  then Ok ()
  else if
    String.length query > 4096
    || String.contains query '\000'
    || not (Stdlib.String.is_valid_utf_8 query)
  then Or_error.error_string "search query must be UTF-8 without NUL, at most 4096 bytes"
  else if String.equal query t.snapshot.query
  then Ok ()
  else (
    t.snapshot <- { t.snapshot with query };
    start t)
;;

let retry t ~epoch =
  check t;
  if not (active t)
  then Or_error.error_string "list search controller closed"
  else if not (Gpuio.Key.equal epoch t.snapshot.epoch)
  then Ok ()
  else (
    match t.snapshot.status with
    | Failed _ | Cancelled -> start t
    | Ready | Debouncing | Loading | Closed -> Ok ())
;;

let cancel t ~epoch =
  check t;
  if active t && Gpuio.Key.equal epoch t.snapshot.epoch && Snapshot.is_busy t.snapshot
  then (
    cancel_pending t;
    t.snapshot <- { (advance t) with status = Cancelled; stale = true };
    notify t)
;;

let update_source t ?(refresh = true) items =
  check t;
  if not (active t)
  then Or_error.error_string "list search controller closed"
  else if C.length items > t.max_loaded
  then Or_error.error_string "list search loaded-record limit exceeded"
  else if phys_equal items t.snapshot.items
  then Ok ()
  else (
    let previous = t.snapshot.items in
    let same_identity = phys_equal (C.identity previous) (C.identity items) in
    let visible =
      if same_identity
      then t.snapshot.visible
      else if not (C.Identity.same_source (C.identity previous) (C.identity items))
      then []
      else
        List.filter t.snapshot.visible ~f:(fun key ->
          Option.exists (C.item_ref previous key) ~f:(C.contains_ref items))
    in
    t.snapshot <- { t.snapshot with items; visible };
    if refresh || not same_identity
    then start t
    else (
      notify t;
      Ok ()))
;;
