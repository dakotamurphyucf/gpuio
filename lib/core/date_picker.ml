open Core

module Error = struct
  type t =
    | Not_open
    | Not_ready
    | Stale_session
    | Stale_draft
    | Disabled
    | Read_only
    | Wrong_mode
    | Incomplete_range
    | Disallowed_selection
    | Limit_exceeded
    | Native of Calendar.Command_error.t
  [@@deriving equal, sexp_of]
end

module Session = struct
  module Id = struct
    type t = int64 [@@deriving compare, equal, sexp_of]

    let to_int64 t = t
  end

  type t =
    { id : Id.t
    ; mode : Calendar.Mode.t
    ; original : Calendar.Selection.t
    ; initial : Calendar.Selection.t
    ; initial_month : Calendar.Month.t
    }
  [@@deriving equal, sexp_of]

  let id t = t.id
  let original t = t.original
  let initial t = t.initial
  let initial_month t = t.initial_month
end

type t =
  { next_id : int64
  ; session : Session.t option
  ; draft : Calendar.Snapshot.t option
  ; error : Error.t option
  }
[@@deriving equal, sexp_of]

let empty = { next_id = 0L; session = None; draft = None; error = None }
let error t = t.error
let draft t = t.draft

let session t ~config ~value =
  Option.filter t.session ~f:(fun session ->
    (not (Calendar.Config.is_disabled config))
    && Calendar.Mode.equal session.mode (Calendar.Config.mode config)
    && Calendar.Selection.equal session.original value)
;;

let close t = { t with session = None; draft = None; error = None }

let sync t ~config ~value =
  match session t ~config ~value with
  | Some _ -> t
  | None -> if Option.is_some t.session then close t else t
;;

let open_popup t ~config ~value ~initial_month =
  let t = sync t ~config ~value in
  if Calendar.Config.is_disabled config
  then { t with error = Some Disabled }
  else if not (Calendar.Selection.fits value ~mode:(Calendar.Config.mode config))
  then { t with error = Some Wrong_mode }
  else (
    match t.session with
    | Some _ -> t
    | None ->
      if Int64.equal t.next_id Int64.max_value
      then { t with error = Some Limit_exceeded }
      else (
        let id = Int64.succ t.next_id in
        let initial =
          if
            Calendar.Constraints.allows_selection
              (Calendar.Config.constraints config)
              value
              ~mode:(Calendar.Config.mode config)
          then value
          else Calendar.Selection.empty
        in
        { next_id = id
        ; session =
            Some
              { Session.id
              ; mode = Calendar.Config.mode config
              ; original = value
              ; initial
              ; initial_month
              }
        ; draft = None
        ; error = None
        }))
;;

let matches t id = Option.exists t.session ~f:(fun s -> Session.Id.equal s.id id)
let cancel t ~session = if matches t session then close t else t

let same_lease left right =
  Gpuio_protocol.Window_id.equal
    (Calendar.Expert.window left)
    (Calendar.Expert.window right)
  && Gpuio_protocol.Node_id.equal (Calendar.Expert.node left) (Calendar.Expert.node right)
;;

let accepts t snapshot =
  Option.exists t.session ~f:(fun s ->
    Calendar.Mode.equal s.mode (Calendar.Snapshot.mode snapshot))
  && Option.for_all t.draft ~f:(fun previous ->
    same_lease previous snapshot
    && Calendar.Revision.compare
         (Calendar.Snapshot.revision snapshot)
         (Calendar.Snapshot.revision previous)
       >= 0)
;;

let observe t ~session snapshot =
  if matches t session && accepts t snapshot
  then { t with draft = Some snapshot; error = None }
  else t
;;

let observe_native t ~session snapshot =
  if
    (not (matches t session))
    || not
         (Option.exists t.session ~f:(fun s ->
            Calendar.Mode.equal s.mode (Calendar.Snapshot.mode snapshot)))
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

let confirm t ~config ~value ~session:id snapshot =
  let t = sync t ~config ~value in
  let result =
    if not (matches t id)
    then Error Error.Stale_session
    else if Calendar.Config.is_read_only config
    then Error Read_only
    else if Option.is_none t.draft
    then Error Not_ready
    else if not (accepts t snapshot)
    then Error Stale_draft
    else (
      let selection = Calendar.Snapshot.selection snapshot in
      if not (Calendar.Selection.fits selection ~mode:(Calendar.Config.mode config))
      then Error Wrong_mode
      else if
        not
          (Calendar.Constraints.allows_selection
             (Calendar.Config.constraints config)
             selection
             ~mode:(Calendar.Config.mode config))
      then Error Disallowed_selection
      else (
        match selection with
        | Range_start _ -> Error Incomplete_range
        | Empty | Single _ | Range _ -> Ok selection))
  in
  match result with
  | Ok _ -> close t, result
  | Error Stale_draft
    when Option.exists t.draft ~f:(fun current -> not (same_lease current snapshot)) ->
    t, result
  | Error error -> failed t ~session:id error, result
;;
