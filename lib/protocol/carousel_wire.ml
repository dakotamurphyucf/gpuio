open Core

module Axis = struct
  type t =
    | Horizontal
    | Vertical
  [@@deriving bin_io, equal, sexp_of]
end

module Direction = struct
  type t =
    | Direct
    | Previous
    | Next
  [@@deriving bin_io, equal, sexp_of]
end

module Config = struct
  type t =
    { revision : int64
    ; ids : string list
    ; selected : int64 option
    ; looping : bool
    ; disabled : bool
    ; axis : Axis.t
    ; auto_advance_ms : int64 option
    ; direction : Direction.t
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid_id id =
    (not (String.is_empty id))
    && String.length id <= 256
    && Stdlib.String.is_valid_utf_8 id
    && not (String.contains id '\000')
  ;;

  let valid t =
    Int64.(t.revision >= 0L)
    && List.length t.ids <= 128
    && List.for_all t.ids ~f:valid_id
    && (not (List.contains_dup t.ids ~compare:String.compare))
    && Option.for_all t.auto_advance_ms ~f:(fun ms ->
      Int64.(ms >= 1000L && ms <= 3_600_000L))
    &&
    match t.selected with
    | None -> List.is_empty t.ids
    | Some index -> Int64.(index >= 0L && index < of_int (List.length t.ids))
  ;;

  let can_replace t previous =
    valid t
    && (Int64.(t.revision > previous.revision)
        || (Int64.equal t.revision previous.revision
            && List.equal String.equal t.ids previous.ids
            && Option.equal Int64.equal t.selected previous.selected
            && Bool.equal t.looping previous.looping
            && Bool.equal t.disabled previous.disabled
            && Option.equal Int64.equal t.auto_advance_ms previous.auto_advance_ms
            && Direction.equal t.direction previous.direction))
  ;;
end

module Request = struct
  type t =
    | Previous
    | Next
    | First
    | Last
    | Select of string
    | Auto_next of
        { revision : int64
        ; from : string
        ; target : string
        }
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Previous | Next | First | Last -> true
    | Select id -> Config.valid_id id
    | Auto_next { revision; from; target } ->
      Int64.(revision >= 0L)
      && Config.valid_id from
      && Config.valid_id target
      && not (String.equal from target)
  ;;
end

let accepts_request (config : Config.t) request =
  Config.valid config
  && (not config.disabled)
  && (not (List.is_empty config.ids))
  && Request.valid request
  &&
  match request with
  | Request.Previous | Next | First | Last -> true
  | Select id -> List.mem config.ids id ~equal:String.equal
  | Auto_next { revision; from; target } ->
    Int64.equal revision config.revision
    && Option.is_some config.auto_advance_ms
    &&
      (match config.selected with
      | None -> false
      | Some index ->
        let index = Int64.to_int_exn index in
        let count = List.length config.ids in
        count > 1
        && (index + 1 < count || config.looping)
        && Option.exists (List.nth config.ids index) ~f:(String.equal from)
        && Option.exists
             (List.nth config.ids ((index + 1) % count))
             ~f:(String.equal target))
;;
