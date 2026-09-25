open Core
module Scene = Gpuio.Canvas_scene
module Wire = Gpuio_protocol.Wire.Canvas
module Id = Gpuio_protocol.Resource_id

module Error = struct
  type t =
    | Native of Wire.Error.t
    | Wrong_application
    | Wrong_scope
  [@@deriving equal, sexp_of]
end

(* Epoch identity records reset intent independently of the native generation.
   Only an acknowledged publication advances that generation. *)
type snapshot =
  { scene : Scene.t
  ; epoch : unit ref
  ; intent : unit ref
  }

type stage =
  | Begin
  | Chunks of int
  | Publish
  | Abort

type upload =
  { snapshot : snapshot
  ; bytes : string
  ; update : Wire.Update.t
  ; mutable stage : stage
  }

type t =
  { scope : Scope.t
  ; owner : Scene.Expert.Owner.t
  ; asset_owner : Gpuio.Asset.Expert.Owner.t
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
  ; mutable handle : Scene.Handle.t option
  ; mutable desired : snapshot option
  ; mutable accepted : snapshot option
  ; mutable revision : int64
  ; mutable generation : int64
  ; mutable upload : upload option
  ; mutable rejected : unit ref option
  ; mutable released : bool
  ; mutable error : Error.t option
  ; mutable notify : ((registration, Error.t) Result.t -> unit) option
  ; mutable unregister : (unit -> unit) option
  }

let limit = 64 * 1024 * 1024
let check t = Scope.Expert.check t.scope
let equal_snapshot a b = phys_equal a.epoch b.epoch && Scene.equal a.scene b.scene

let held entry =
  List.filter_opt
    [ entry.desired; entry.accepted; Option.map entry.upload ~f:(fun u -> u.snapshot) ]
  |> List.fold ~init:[] ~f:(fun acc snapshot ->
    if List.exists acc ~f:(Scene.equal snapshot.scene) then acc else snapshot.scene :: acc)
  |> List.sum (module Int) ~f:Scene.Expert.retained_bytes
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

let clear entry =
  change entry (fun () ->
    entry.desired <- None;
    entry.accepted <- None;
    entry.upload <- None);
  entry.rejected <- None
;;

let forget entry =
  unregister entry;
  clear entry;
  entry.registry.entries <- Map.remove entry.registry.entries entry.key
;;

let release entry =
  check entry.registry;
  entry.released <- true;
  entry.notify <- None;
  unregister entry;
  clear entry;
  entry.registry.wake ()
;;

let fail entry error =
  let notify = entry.notify in
  entry.error <- Some error;
  release entry;
  Option.iter notify ~f:(fun f -> f (Error error))
;;

let reject entry error =
  match entry.accepted, entry.upload with
  | None, _ | _, None -> fail entry (Native error)
  | Some _, Some upload ->
    entry.error <- Some (Native error);
    entry.rejected <- Some upload.snapshot.intent;
    (match upload.stage with
     | Begin -> change entry (fun () -> entry.upload <- None)
     | Chunks _ | Publish -> upload.stage <- Abort
     | Abort -> fail entry (Native error))
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
    && Option.is_none t.upload
    && Option.exists t.desired ~f:(fun desired ->
      Option.exists t.accepted ~f:(equal_snapshot desired))
  ;;

  let scene t =
    check t.registry;
    Option.map t.desired ~f:(fun snapshot -> snapshot.scene)
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

  let update t scene ~reset =
    check t.registry;
    match t.desired with
    | None -> Error (Error.Native Closed)
    | Some previous ->
      if not (Scene.Expert.assets_belong_to scene ~asset_owner:t.registry.asset_owner)
      then Error Error.Wrong_application
      else (
        let desired =
          { scene; epoch = (if reset then ref () else previous.epoch); intent = ref () }
        in
        let before = held t in
        t.desired <- Some desired;
        let after = held t in
        if after - before > limit - t.registry.bytes
        then (
          t.desired <- Some previous;
          Error (Error.Native Resource_limit))
        else (
          t.registry.bytes <- t.registry.bytes + after - before;
          t.registry.wake ();
          Ok ()))
  ;;

  let set t scene = update t scene ~reset:false
  let reset t scene = update t scene ~reset:true
end

let create ~scope ~asset_owner ~wake =
  Scope.Expert.check scope;
  { scope
  ; owner = Scene.Expert.Owner.create ()
  ; asset_owner
  ; wake
  ; entries = Int.Map.empty
  ; next = 0
  ; cursor = -1
  ; bytes = 0
  ; pending = None
  ; closed = false
  }
;;

let register t ~scope scene ~on_result =
  check t;
  if t.closed || not (Scope.is_active scope)
  then on_result (Error (Error.Native Closed))
  else if not (Scope.Expert.same_tree t.scope scope)
  then on_result (Error Error.Wrong_scope)
  else if not (Scene.Expert.assets_belong_to scene ~asset_owner:t.asset_owner)
  then on_result (Error Error.Wrong_application)
  else if
    Map.length t.entries >= 256
    || Scene.Expert.retained_bytes scene > limit - t.bytes
    || t.next = Int.max_value
  then on_result (Error (Error.Native Resource_limit))
  else (
    let entry =
      { registry = t
      ; key = t.next
      ; id = None
      ; handle = None
      ; desired = Some { scene; epoch = ref (); intent = ref () }
      ; accepted = None
      ; revision = 0L
      ; generation = 1L
      ; upload = None
      ; rejected = None
      ; released = false
      ; error = None
      ; notify = Some on_result
      ; unregister = None
      }
    in
    match Scope.Expert.on_cancel scope (fun () -> release entry) with
    | Error _ -> on_result (Error (Error.Native Resource_limit))
    | Ok unregister ->
      entry.unregister <- Some unregister;
      t.next <- t.next + 1;
      t.bytes <- t.bytes + held entry;
      t.entries <- Map.set t.entries ~key:entry.key ~data:entry;
      t.wake ())
;;

let prepare entry id snapshot =
  let reset =
    Option.exists entry.accepted ~f:(fun previous ->
      not (phys_equal snapshot.epoch previous.epoch))
  in
  if
    Int64.equal entry.revision Int64.max_value
    || (reset && Int64.equal entry.generation Int64.max_value)
  then fail entry (Native Invalid_revision)
  else (
    match Scene.Expert.encode snapshot.scene ~asset_owner:entry.registry.asset_owner with
    | Error _ -> fail entry Wrong_application
    | Ok bytes ->
      let update : Wire.Update.t =
        { id
        ; base = entry.revision
        ; revision = Int64.succ entry.revision
        ; generation = (if reset then Int64.succ entry.generation else entry.generation)
        ; bytes = Int64.of_int (String.length bytes)
        }
      in
      change entry (fun () ->
        entry.upload <- Some { snapshot; bytes; update; stage = Begin }))
;;

let request entry ~can_start =
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
       | None, Some snapshot
         when (not (Option.exists entry.accepted ~f:(equal_snapshot snapshot)))
              && (not (Option.exists entry.rejected ~f:(phys_equal snapshot.intent)))
              && can_start -> prepare entry id snapshot
       | Some _, _ | None, _ -> ());
      Option.map entry.upload ~f:(fun upload ->
        match upload.stage with
        | Begin -> Begin upload.update
        | Publish -> Publish (id, upload.update.revision)
        | Abort -> Abort (id, upload.update.revision)
        | Chunks offset ->
          let len = Int.min Wire.max_chunk_bytes (String.length upload.bytes - offset) in
          Chunk
            ( id
            , upload.update.revision
            , Int64.of_int offset
            , String.sub upload.bytes ~pos:offset ~len )))
;;

let next_request t =
  check t;
  if t.closed || Option.is_some t.pending
  then None
  else (
    let after, before =
      List.partition_tf (Map.data t.entries) ~f:(fun e -> e.key > t.cursor)
    in
    let cleanup, active =
      List.partition_tf (after @ before) ~f:(fun e ->
        e.released
        || Option.exists e.upload ~f:(fun upload ->
          match upload.stage with
          | Abort -> true
          | Begin | Chunks _ | Publish -> false))
    in
    let can_start = Map.count t.entries ~f:(fun e -> Option.is_some e.upload) < 4 in
    List.find_map (cleanup @ active) ~f:(fun entry ->
      Option.map (request entry ~can_start) ~f:(fun request ->
        t.cursor <- entry.key;
        t.pending <- Some (entry, request);
        request)))
;;

let acknowledge entry request =
  match entry.upload with
  | None -> fail entry (Native Native_failure)
  | Some upload ->
    (match request with
     | Wire.Request.Begin _ -> upload.stage <- Chunks 0
     | Chunk (_, _, offset, bytes) ->
       let next = Int64.to_int_exn offset + String.length bytes in
       upload.stage
       <- (if next = String.length upload.bytes then Publish else Chunks next)
     | Publish _ ->
       change entry (fun () ->
         entry.accepted <- Some upload.snapshot;
         entry.upload <- None);
       entry.revision <- upload.update.revision;
       entry.generation <- upload.update.generation;
       entry.handle
       <- Some (Scene.Expert.handle ~owner:entry.registry.owner upload.update.id);
       entry.error <- None;
       entry.rejected <- None;
       let notify = entry.notify in
       entry.notify <- None;
       Option.iter notify ~f:(fun f -> f (Ok entry))
     | Abort _ -> change entry (fun () -> entry.upload <- None)
     | Create | Release _ -> fail entry (Native Native_failure))
;;

let complete t response =
  check t;
  match t.pending with
  | None -> ()
  | Some (entry, request) ->
    t.pending <- None;
    if entry.released
    then (
      (match response, request with
       | Wire.Response.Created id, Create -> entry.id <- Some id
       | _ -> ());
      match request, entry.id with
      | Release _, _ | _, None -> forget entry
      | _, Some _ -> ())
    else (
      match response, request with
      | Wire.Response.Failed ((Closed | Stale_handle | Native_failure) as error), _ ->
        fail entry (Native error)
      | Failed error, (Begin _ | Chunk _ | Publish _) -> reject entry error
      | Failed error, (Create | Abort _ | Release _) -> fail entry (Native error)
      | Created id, Create -> entry.id <- Some id
      | Ack, (Begin _ | Chunk _ | Publish _ | Abort _) -> acknowledge entry request
      | Ack, Release _ -> forget entry
      | Created _, _ | Ack, Create -> fail entry (Native Native_failure));
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

let accepts_event t source ~scene_revision ~scene_generation observation =
  check t;
  let before_scene =
    Int64.(scene_revision = 0L && scene_generation = 0L)
    &&
    match observation with
    | Gpuio_protocol.Canvas_view_wire.Observation.Failed _ -> true
    | _ -> false
  in
  (not t.closed)
  && Gpuio_protocol.Canvas_view_wire.Observation.valid observation
  && (before_scene || Int64.(scene_revision > 0L && scene_generation > 0L))
  &&
  match source with
  | None -> before_scene
  | Some id ->
    Map.exists t.entries ~f:(fun entry ->
      (not entry.released)
      && Option.exists entry.id ~f:(Id.equal id)
      && Option.exists entry.desired ~f:(fun desired ->
        let matches snapshot revision generation =
          phys_equal desired.epoch snapshot.epoch
          && (before_scene
              || (Int64.equal scene_revision revision
                  && Int64.equal scene_generation generation))
        in
        Option.exists entry.accepted ~f:(fun snapshot ->
          matches snapshot entry.revision entry.generation)
        || Option.exists entry.upload ~f:(fun upload ->
          matches upload.snapshot upload.update.revision upload.update.generation
          && Option.exists t.pending ~f:(fun (pending, request) ->
            phys_equal pending entry
            &&
            match request with
            | Wire.Request.Publish (source, revision) ->
              Id.equal source id && Int64.equal revision upload.update.revision
            | Create | Begin _ | Chunk _ | Abort _ | Release _ -> false))))
;;

module Expert = struct
  let owner t = t.owner

  let counts t =
    check t;
    Map.length t.entries, t.bytes
  ;;
end
