open Core
module Source = Gpuio.Text_source
module Wire = Gpuio_protocol.Wire.Document
module Id = Gpuio_protocol.Resource_id

type stage =
  | Begin
  | Chunks of int
  | Publish

type upload =
  { source : Source.t
  ; update : Wire.Update.t
  ; mutable stage : stage
  }

type t =
  { scope : Scope.t
  ; owner : Source.Expert.Owner.t
  ; wake : unit -> unit
  ; mutable entries : registration Int.Map.t
  ; mutable next : int
  ; mutable cursor : int
  ; mutable bytes : int
  ; mutable pending : (registration * Wire.Request.t) option
  ; mutable closed : bool
  }

and registration =
  { registry : t
  ; key : int
  ; mutable id : Id.t option
  ; mutable handle : Source.Handle.t option
  ; mutable desired : Source.t option
  ; mutable accepted : Source.t option
  ; mutable revision : int64
  ; mutable generation : int64
  ; mutable upload : upload option
  ; mutable released : bool
  ; mutable error : Wire.Error.t option
  ; mutable notify : ((registration, Wire.Error.t) Result.t -> unit) option
  ; mutable unregister : (unit -> unit) option
  }

let limit = 64 * 1024 * 1024
let check t = Scope.Expert.check t.scope

let held entry =
  let sources =
    List.filter_opt
      [ entry.desired; entry.accepted; Option.map entry.upload ~f:(fun u -> u.source) ]
  in
  List.fold sources ~init:[] ~f:(fun acc source ->
    if List.exists acc ~f:(Source.equal source) then acc else source :: acc)
  |> List.sum (module Int) ~f:Source.byte_length
;;

let change entry f =
  let before = held entry in
  f ();
  entry.registry.bytes <- entry.registry.bytes + held entry - before
;;

let unregister entry =
  let callback = entry.unregister in
  entry.unregister <- None;
  Option.iter callback ~f:(fun f -> f ())
;;

let forget entry =
  unregister entry;
  change entry (fun () ->
    entry.desired <- None;
    entry.accepted <- None;
    entry.upload <- None);
  entry.registry.entries <- Map.remove entry.registry.entries entry.key
;;

let release entry =
  check entry.registry;
  entry.released <- true;
  entry.notify <- None;
  unregister entry;
  change entry (fun () ->
    entry.desired <- None;
    entry.accepted <- None;
    entry.upload <- None);
  entry.registry.wake ()
;;

let fail entry error =
  let notify = entry.notify in
  entry.error <- Some error;
  release entry;
  Option.iter notify ~f:(fun f -> f (Error error))
;;

module Registration = struct
  type t = registration

  let handle t =
    check t.registry;
    Option.value_exn t.handle
  ;;

  let is_published t =
    check t.registry;
    (not t.released)
    && Option.exists t.desired ~f:(fun desired ->
      Option.exists t.accepted ~f:(Source.equal desired))
  ;;

  let source t =
    check t.registry;
    t.desired
  ;;

  let error t =
    check t.registry;
    t.error
  ;;

  let release = release

  let is_released t =
    check t.registry;
    t.released
  ;;

  let set t source =
    check t.registry;
    match t.desired with
    | None -> Error Wire.Error.Closed
    | Some previous ->
      if not (Source.Expert.same_source source previous)
      then Error Wire.Error.Invalid_revision
      else (
        let before = held t in
        t.desired <- Some source;
        let after = held t in
        if after - before > limit - t.registry.bytes
        then (
          t.desired <- Some previous;
          Error Wire.Error.Resource_limit)
        else (
          t.registry.bytes <- t.registry.bytes + after - before;
          t.registry.wake ();
          Ok ()))
  ;;
end

let create ~scope ~wake =
  Scope.Expert.check scope;
  { scope
  ; owner = Source.Expert.Owner.create ()
  ; wake
  ; entries = Int.Map.empty
  ; next = 0
  ; cursor = -1
  ; bytes = 0
  ; pending = None
  ; closed = false
  }
;;

let register t ~scope source ~on_result =
  check t;
  if t.closed || not (Scope.is_active scope)
  then on_result (Error Wire.Error.Closed)
  else if not (Scope.Expert.same_tree t.scope scope)
  then on_result (Error Wire.Error.Not_ready)
  else if
    Map.length t.entries >= 1024
    || Source.byte_length source > limit - t.bytes
    || t.next = Int.max_value
  then on_result (Error Wire.Error.Resource_limit)
  else (
    let entry =
      { registry = t
      ; key = t.next
      ; id = None
      ; handle = None
      ; desired = Some source
      ; accepted = None
      ; revision = 0L
      ; generation = 1L
      ; upload = None
      ; released = false
      ; error = None
      ; notify = Some on_result
      ; unregister = None
      }
    in
    match Scope.Expert.on_cancel scope (fun () -> release entry) with
    | Error _ -> on_result (Error Wire.Error.Resource_limit)
    | Ok unregister ->
      entry.unregister <- Some unregister;
      t.next <- t.next + 1;
      t.bytes <- t.bytes + held entry;
      t.entries <- Map.set t.entries ~key:entry.key ~data:entry;
      t.wake ())
;;

let wire_status source : Wire.Status.t =
  match Source.status source with
  | Streaming -> Streaming
  | Complete -> Complete
  | Cancelled -> Cancelled
;;

let request entry =
  let open Wire.Request in
  if entry.released
  then (
    match entry.id with
    | None ->
      forget entry;
      None
    | Some id -> Some (Release id))
  else (
    match entry.id with
    | None -> Some Create
    | Some id ->
      (match entry.upload, entry.desired with
       | None, Some source
         when not (Option.exists entry.accepted ~f:(Source.equal source)) ->
         let reset =
           Option.exists entry.accepted ~f:(fun previous ->
             not (Source.Expert.same_generation source previous))
         in
         if
           Int64.equal entry.revision Int64.max_value
           || (reset && Int64.equal entry.generation Int64.max_value)
         then fail entry Invalid_revision
         else (
           let from =
             Option.value_map entry.accepted ~default:0 ~f:(fun previous ->
               Source.Expert.changed_from source ~previous)
           in
           let generation =
             if reset then Int64.succ entry.generation else entry.generation
           in
           let update : Wire.Update.t =
             { id
             ; base = entry.revision
             ; revision = Int64.succ entry.revision
             ; generation
             ; from_byte = Int64.of_int from
             ; suffix_bytes = Int64.of_int (Source.byte_length source - from)
             ; status = wire_status source
             }
           in
           change entry (fun () -> entry.upload <- Some { source; update; stage = Begin }))
       | Some _, _ | None, _ -> ());
      Option.map entry.upload ~f:(fun upload ->
        match upload.stage with
        | Begin -> Begin upload.update
        | Publish -> Publish (id, upload.update.revision)
        | Chunks offset ->
          let first = Int64.to_int_exn upload.update.from_byte + offset in
          let last =
            Int.min (first + Wire.max_chunk_bytes) (Source.byte_length upload.source)
          in
          (* Public source ranges require UTF-8 boundaries, so move a partial
             scalar into the next transport chunk. At most three retries. *)
          let rec slice last =
            match Source.slice upload.source ~first ~last with
            | Ok bytes -> bytes
            | Error _ -> slice (last - 1)
          in
          Chunk (id, upload.update.revision, Int64.of_int offset, slice last)))
;;

let next_request t =
  check t;
  if t.closed || Option.is_some t.pending
  then None
  else (
    let entries = Map.data t.entries in
    let after, before = List.partition_tf entries ~f:(fun e -> e.key > t.cursor) in
    let cleanup, active = List.partition_tf (after @ before) ~f:(fun e -> e.released) in
    List.find_map (cleanup @ active) ~f:(fun entry ->
      Option.map (request entry) ~f:(fun request ->
        t.cursor <- entry.key;
        t.pending <- Some (entry, request);
        request)))
;;

let complete t response =
  check t;
  match t.pending with
  | None -> ()
  | Some (entry, request) ->
    t.pending <- None;
    if entry.released
    then (
      (match response with
       | Wire.Response.Created id -> entry.id <- Some id
       | Ack | Failed _ -> ());
      match request, entry.id with
      | Release _, _ | _, None -> forget entry
      | _, Some _ -> ())
    else (
      match response, request with
      | Wire.Response.Failed error, _ -> fail entry error
      | Created id, Create -> entry.id <- Some id
      | Ack, (Begin _ | Chunk _ | Publish _) ->
        (match entry.upload with
         | None -> fail entry Native_failure
         | Some upload ->
           (match request with
            | Begin _ ->
              upload.stage
              <- (if Int64.equal upload.update.suffix_bytes 0L then Publish else Chunks 0)
            | Chunk (_, _, offset, bytes) ->
              let next = Int64.to_int_exn offset + String.length bytes in
              upload.stage
              <- (if next = Int64.to_int_exn upload.update.suffix_bytes
                  then Publish
                  else Chunks next)
            | Publish _ ->
              change entry (fun () ->
                entry.accepted <- Some upload.source;
                entry.upload <- None);
              entry.revision <- upload.update.revision;
              entry.generation <- upload.update.generation;
              entry.handle <- Some (Source.Expert.handle ~owner:t.owner upload.update.id);
              let notify = entry.notify in
              entry.notify <- None;
              Option.iter notify ~f:(fun f -> f (Ok entry))
            | Create | Abort _ | Release _ -> assert false))
      | Ack, Release _ -> forget entry
      | Created _, _ | Ack, _ -> fail entry Native_failure);
    t.wake ()
;;

let close t =
  check t;
  t.closed <- true;
  t.pending <- None;
  Map.iter t.entries ~f:(fun entry ->
    release entry;
    forget entry);
  t.entries <- Int.Map.empty
;;

module Expert = struct
  let owner t = t.owner

  let counts t =
    check t;
    Map.length t.entries, t.bytes
  ;;
end

let accepts_navigation t id ~generation =
  check t;
  (not t.closed)
  && Map.exists t.entries ~f:(fun entry ->
    (not entry.released)
    && Option.exists entry.id ~f:(Id.equal id)
    && Option.exists entry.desired ~f:(fun desired ->
      (Int64.equal generation entry.generation
       && Option.exists entry.accepted ~f:(Source.Expert.same_generation desired))
      || Option.exists entry.upload ~f:(fun upload ->
        Int64.equal generation upload.update.generation
        && Source.Expert.same_generation desired upload.source)))
;;
