open Core

exception Invalid_wire_handle

module type S = sig
  type t [@@deriving bin_io, compare, equal, sexp_of]

  val create : slot:int64 -> generation:int64 -> t Or_error.t
  val slot : t -> int64
  val generation : t -> int64
end

module Make () : S = struct
  type t =
    { slot : int64
    ; generation : int64
    }
  [@@deriving bin_io, compare, equal, sexp_of]

  let create ~slot ~generation =
    if
      Int64.(
        slot >= 0L && slot <= 4294967295L && generation > 0L && generation <= 4294967295L)
    then Ok { slot; generation }
    else Or_error.error_string "invalid slot or generation"
  ;;

  let slot t = t.slot
  let generation t = t.generation

  let bin_read_t buffer ~pos_ref =
    let t = bin_read_t buffer ~pos_ref in
    match create ~slot:t.slot ~generation:t.generation with
    | Ok t -> t
    | Error _ -> raise Invalid_wire_handle
  ;;

  let bin_reader_t = { bin_reader_t with read = bin_read_t }
end
