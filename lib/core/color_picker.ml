open Core

module Error = struct
  type t =
    | Not_open
    | Not_ready
    | Stale_session
    | Stale_draft
    | Disabled
    | Read_only
    | Composing
    | Invalid_draft
    | Drag_in_progress
    | Disallowed_value
    | Limit_exceeded
    | Native of Color_input.Command_error.t
  [@@deriving equal, sexp_of]
end

module Session = struct
  module Id = struct
    type t = int64 [@@deriving compare, equal, sexp_of]

    let to_int64 t = t
  end

  type t =
    { id : Id.t
    ; original : Color_value.Value.t
    ; initial : Color_value.Value.t
    }
  [@@deriving equal, sexp_of]

  let id t = t.id
  let original t = t.original
  let initial t = t.initial
end

type t =
  { next_id : int64
  ; session : Session.t option
  ; draft : Color_input.Snapshot.t option
  ; error : Error.t option
  }
[@@deriving equal, sexp_of]

let empty = { next_id = 0L; session = None; draft = None; error = None }
let error t = t.error
let draft t = t.draft

let session t ~config ~value =
  Option.filter t.session ~f:(fun session ->
    (not (Color_input.Config.is_disabled config))
    && Color_value.Value.equal session.original value)
;;

let close t = { t with session = None; draft = None; error = None }

let sync t ~config ~value =
  match session t ~config ~value with
  | Some _ -> t
  | None -> if Option.is_some t.session then close t else t
;;

let seed config value =
  if Color_input.Config.allows config value
  then value
  else if Color_input.Config.allows config Color_value.Value.Empty
  then Color_value.Value.Empty
  else (
    let red, green, blue =
      match value with
      | Color_value.Value.Empty -> 0, 0, 0
      | Color color ->
        ( Color_value.Rgba.red color
        , Color_value.Rgba.green color
        , Color_value.Rgba.blue color )
    in
    Color_value.Value.Color
      (Color_value.Rgba.create ~red ~green ~blue ~alpha:255 |> Or_error.ok_exn))
;;

let open_popup t ~config ~value =
  let t = sync t ~config ~value in
  if Color_input.Config.is_disabled config
  then { t with error = Some Disabled }
  else (
    match t.session with
    | Some _ -> t
    | None ->
      if Int64.equal t.next_id Int64.max_value
      then { t with error = Some Limit_exceeded }
      else (
        let id = Int64.succ t.next_id in
        { next_id = id
        ; session = Some { Session.id; original = value; initial = seed config value }
        ; draft = None
        ; error = None
        }))
;;

let matches t id = Option.exists t.session ~f:(fun s -> Session.Id.equal s.id id)
let cancel t ~session = if matches t session then close t else t

let same_lease left right =
  Gpuio_protocol.Window_id.equal
    (Color_input.Expert.window left)
    (Color_input.Expert.window right)
  && Gpuio_protocol.Node_id.equal
       (Color_input.Expert.node left)
       (Color_input.Expert.node right)
;;

let accepts t snapshot =
  Option.for_all t.draft ~f:(fun previous ->
    same_lease previous snapshot
    && Color_input.Revision.compare
         (Color_input.Snapshot.revision snapshot)
         (Color_input.Snapshot.revision previous)
       >= 0)
;;

let observe t ~session snapshot =
  if matches t session && accepts t snapshot
  then { t with draft = Some snapshot; error = None }
  else t
;;

let observe_native t ~session snapshot =
  if not (matches t session)
  then t
  else (
    let t =
      match t.draft with
      | Some previous when not (same_lease previous snapshot) -> { t with draft = None }
      | Some _ | None -> t
    in
    observe t ~session snapshot)
;;

let failed t ~session error =
  if matches t session then { t with error = Some error } else t
;;

let candidate snapshot ~config =
  if Color_input.Config.is_disabled config
  then Error Error.Disabled
  else if Color_input.Config.is_read_only config
  then Error Read_only
  else if
    Option.exists (Color_input.Snapshot.draft snapshot) ~f:Color_input.Draft.is_composing
  then Error Composing
  else if
    Option.exists (Color_input.Snapshot.interaction snapshot) ~f:(fun interaction ->
      match Color_input.Interaction.kind interaction with
      | Drag _ -> true
      | Text _ -> false)
  then Error Drag_in_progress
  else if
    Option.exists (Color_input.Snapshot.draft snapshot) ~f:(fun draft ->
      not (Color_input.Draft.Status.equal (Color_input.Draft.status draft) Valid))
  then Error Invalid_draft
  else if
    not
      (Color_input.Snapshot.value_allowed snapshot
       && Color_input.Config.allows config (Color_input.Snapshot.value snapshot))
  then Error Disallowed_value
  else Ok (Color_input.Snapshot.value snapshot)
;;

let confirm t ~config ~value ~session:id snapshot =
  let t = sync t ~config ~value in
  let result =
    if not (matches t id)
    then Error Error.Stale_session
    else if Option.is_none t.draft
    then Error Not_ready
    else if not (accepts t snapshot)
    then Error Stale_draft
    else candidate snapshot ~config
  in
  match result with
  | Ok _ -> close t, result
  | Error Stale_draft
    when Option.exists t.draft ~f:(fun current -> not (same_lease current snapshot)) ->
    t, result
  | Error error -> failed t ~session:id error, result
;;
