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
