open Core
module Source = Gpuio.Asset.Source
module Wire = Gpuio_protocol.Wire.Asset
module Id = Gpuio_protocol.Resource_id

module Error = struct
  type t =
    | Closed
    | Not_ready
    | Resource_limit
    | Invalid_scope
    | Native_failure
  [@@deriving equal, sexp_of]
end

module Phase = struct
  type t =
    | Beginning of Source.t
    | Uploading of
        { id : Id.t
        ; source : Source.t
        ; offset : int
        }
    | Finishing of Id.t
    | Ready of Id.t
    | Releasing of Id.t
    | Awaiting_begin_cleanup
    | Released
end

type t =
  { scope : Scope.t
  ; owner : Gpuio.Asset.Expert.Owner.t
  ; wake : unit -> unit
  ; mutable entries : registration Int.Map.t
  ; mutable next : int
  ; mutable uploads : int
  ; mutable source_bytes : int
  ; mutable pending : (registration * Wire.Request.t) option
  ; mutable closed : bool
  }

and registration =
  { registry : t
  ; key : int
  ; format : Gpuio.Asset.Format.t
  ; mutable published : Gpuio.Asset.Handle.t option
  ; mutable phase : Phase.t
  ; mutable notify : ((registration, Error.t) Result.t -> unit) option
  ; mutable unregister : (unit -> unit) option
  }

let check t = Scope.Expert.check t.scope

let clear_source t =
  let bytes =
    match t.phase with
    | Beginning source | Uploading { source; _ } -> Source.byte_length source
    | Finishing _ | Ready _ | Releasing _ | Awaiting_begin_cleanup | Released -> 0
  in
  t.registry.source_bytes <- t.registry.source_bytes - bytes
;;

let end_upload t =
  match t.phase with
  | Beginning _ | Uploading _ | Finishing _ ->
    clear_source t;
    t.registry.uploads <- t.registry.uploads - 1
  | Ready _ | Releasing _ | Awaiting_begin_cleanup | Released -> ()
;;

let unregister t =
  let cleanup = t.unregister in
  t.unregister <- None;
  Option.iter cleanup ~f:(fun cleanup -> cleanup ())
;;

let forget t =
  t.phase <- Released;
  t.notify <- None;
  unregister t;
  t.registry.entries <- Map.remove t.registry.entries t.key
;;

let release t =
  check t.registry;
  t.notify <- None;
  unregister t;
  end_upload t;
  (match t.phase with
   | Beginning _ ->
     (match t.registry.pending with
      | Some (pending, _) when pending.key = t.key -> t.phase <- Awaiting_begin_cleanup
      | Some _ | None -> forget t)
   | Uploading { id; _ } | Finishing id | Ready id -> t.phase <- Releasing id
   | Releasing _ | Awaiting_begin_cleanup | Released -> ());
  t.registry.wake ()
;;

module Registration = struct
  type t = registration

  let release = release

  let handle t =
    check t.registry;
    (* Registrations only escape through their successful publication callback.
       Keep the immutable reference available after registration retirement. *)
    Option.value_exn t.published
  ;;

  let is_released t =
    check t.registry;
    match t.phase with
    | Releasing _ | Awaiting_begin_cleanup | Released -> true
    | Beginning _ | Uploading _ | Finishing _ | Ready _ -> false
  ;;

  module Expert = struct
    let native_id t =
      check t.registry;
      match t.phase with
      | Ready id -> Some id
      | Beginning _
      | Uploading _
      | Finishing _
      | Releasing _
      | Awaiting_begin_cleanup
      | Released -> None
    ;;
  end
end

let create ~scope ~wake =
  Scope.Expert.check scope;
  { scope
  ; owner = Gpuio.Asset.Expert.Owner.create ()
  ; wake
  ; entries = Int.Map.empty
  ; next = 0
  ; uploads = 0
  ; source_bytes = 0
  ; pending = None
  ; closed = false
  }
;;

let register t ~scope source ~(on_result : (registration, Error.t) Result.t -> unit) =
  check t;
  if t.closed || not (Scope.is_active scope)
  then on_result (Error Closed)
  else if not (Scope.Expert.same_tree t.scope scope)
  then on_result (Error Invalid_scope)
  else if
    Map.length t.entries >= 1024
    || t.uploads >= 8
    || Source.byte_length source > (64 * 1024 * 1024) - t.source_bytes
    || t.next = Int.max_value
  then on_result (Error Resource_limit)
  else (
    let entry =
      { registry = t
      ; key = t.next
      ; format = Source.format source
      ; published = None
      ; phase = Beginning source
      ; notify = Some on_result
      ; unregister = None
      }
    in
    match Scope.Expert.on_cancel scope (fun () -> release entry) with
    | Error _ -> on_result (Error Resource_limit)
    | Ok unregister ->
      entry.unregister <- Some unregister;
      t.next <- t.next + 1;
      t.uploads <- t.uploads + 1;
      t.source_bytes <- t.source_bytes + Source.byte_length source;
      t.entries <- Map.set t.entries ~key:entry.key ~data:entry;
      t.wake ())
;;

let format : Gpuio.Asset.Format.t -> Wire.Format.t = function
  | Png -> Png
  | Jpeg -> Jpeg
  | Webp -> Webp
  | Gif -> Gif
  | Svg -> Svg
  | Bmp -> Bmp
  | Tiff -> Tiff
  | Ico -> Ico
  | Pnm -> Pnm
;;

let next_request t =
  check t;
  if t.closed || Option.is_some t.pending
  then None
  else (
    let find ~cleanup =
      Map.fold_until
        t.entries
        ~init:()
        ~finish:(fun () -> None)
        ~f:(fun ~key:_ ~data:entry () ->
          let request =
            match entry.phase with
            | Releasing id when cleanup -> Some (Wire.Request.Release id)
            | Beginning source when not cleanup ->
              Some
                (Begin
                   ( format (Source.format source)
                   , Int64.of_int (Source.byte_length source) ))
            | Uploading { id; source; offset } when not cleanup ->
              let len =
                Int.min Wire.max_chunk_bytes (Source.byte_length source - offset)
              in
              Some
                (Append
                   ( id
                   , Int64.of_int offset
                   , String.sub (Source.bytes source) ~pos:offset ~len ))
            | Finishing id when not cleanup -> Some (Finish id)
            | Beginning _
            | Uploading _
            | Finishing _
            | Ready _
            | Releasing _
            | Awaiting_begin_cleanup
            | Released -> None
          in
          match request with
          | None -> Continue ()
          | Some request -> Stop (Some (entry, request)))
    in
    let next =
      match find ~cleanup:true with
      | Some _ as next -> next
      | None -> find ~cleanup:false
    in
    t.pending <- next;
    Option.map next ~f:snd)
;;

let error : Wire.Error.t -> Error.t = function
  | Closed -> Closed
  | Not_ready -> Not_ready
  | Resource_limit -> Resource_limit
  | Invalid_size
  | Stale_handle
  | Not_uploading
  | Invalid_chunk
  | Incomplete
  | Native_failure -> Native_failure
;;

let complete t (response : Wire.Response.t) =
  check t;
  match t.pending with
  | None -> ()
  | Some (entry, request) ->
    t.pending <- None;
    let fail reason =
      let notify = entry.notify in
      release entry;
      Option.iter notify ~f:(fun notify -> notify (Error reason))
    in
    (match entry.phase, request, response with
     | Awaiting_begin_cleanup, Begin _, Begun id -> entry.phase <- Releasing id
     | Awaiting_begin_cleanup, Begin _, Failed _ -> forget entry
     | Releasing _, Release _, (Ack | Failed (Closed | Stale_handle | Not_uploading)) ->
       forget entry
     | Releasing _, Release _, Failed (Resource_limit | Not_ready) -> ()
     | Releasing _, (Append _ | Finish _), (Ack | Failed _) -> ()
     | Beginning source, Begin _, Begun id ->
       entry.phase <- Uploading { id; source; offset = 0 }
     | Uploading { id; source; offset }, Append (_, _, bytes), Ack ->
       let offset = offset + String.length bytes in
       if offset = Source.byte_length source
       then (
         clear_source entry;
         entry.phase <- Finishing id)
       else entry.phase <- Uploading { id; source; offset }
     | Finishing id, Finish _, Ack ->
       end_upload entry;
       entry.phase <- Ready id;
       entry.published
       <- Some (Gpuio.Asset.Expert.handle ~owner:t.owner ~id ~format:entry.format);
       let notify = entry.notify in
       entry.notify <- None;
       Option.iter notify ~f:(fun notify -> notify (Ok entry))
     | (Beginning _ | Uploading _ | Finishing _), _, Failed reason -> fail (error reason)
     | Released, _, _ -> ()
     | _, _, _ -> failwith "invalid native asset protocol response");
    t.wake ()
;;

let close t =
  check t;
  if not t.closed
  then (
    t.closed <- true;
    Map.iter t.entries ~f:(fun entry ->
      end_upload entry;
      forget entry);
    t.entries <- Int.Map.empty;
    t.pending <- None)
;;

module Expert = struct
  let owner t =
    check t;
    t.owner
  ;;

  let counts t =
    check t;
    Map.length t.entries, t.uploads, t.source_bytes
  ;;
end
