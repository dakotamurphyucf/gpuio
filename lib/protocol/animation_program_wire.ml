open Core
module A = Animation_wire

module Timing = struct
  type t =
    | Tween of int64 * A.Easing.t
    | Spring of A.Spring.t
  [@@deriving bin_io, equal, sexp_of]
end

module Stage = struct
  type t =
    { targets : A.Target.t list
    ; timing : Timing.t
    ; delay_ms : int64
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Clock = struct
  type t =
    | Independent
    | Application
    | Group of string
  [@@deriving bin_io, equal, sexp_of]
end

module Playback = struct
  type t =
    | Running
    | Paused
    | Cancelled
  [@@deriving bin_io, equal, sexp_of]
end

module Program = struct
  type t =
    { initial : A.Target.t list option
    ; stages : Stage.t list
    ; delay_ms : int64
    ; repeat : A.Repeat.t
    ; clock : Clock.t
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Config = struct
  type t =
    { generation : int64
    ; program : Program.t
    ; playback : Playback.t
    ; restart : int64
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Stage_result = struct
  type t =
    | Played
    | Reduced_motion
  [@@deriving bin_io, equal, sexp_of]
end

module Cancel_reason = struct
  type t =
    | Replaced
    | Removed
    | Window_closed
    | Requested
  [@@deriving bin_io, equal, sexp_of]
end

module Observation = struct
  type t =
    | Stage_completed of int64 * Stage_result.t
    | Finished
    | Cancelled of Cancel_reason.t
  [@@deriving bin_io, equal, sexp_of]
end

module Signal = struct
  type t =
    { generation : int64
    ; index : int64
    ; observation : Observation.t
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Int64.(t.generation > 0L)
    &&
    match t.observation with
    | Stage_completed (stage, _) ->
      Int64.(stage >= 0L && stage < 32L && t.index = stage + 1L)
    | Finished | Cancelled _ -> Int64.equal t.index 33L
  ;;

  let valid_batch signals =
    match signals with
    | [] -> false
    | first :: _ ->
      List.length signals <= 33
      && List.for_all signals ~f:(fun t ->
        valid t && Int64.equal t.generation first.generation)
      && List.is_sorted_strictly signals ~compare:(fun a b ->
        Int64.compare a.index b.index)
  ;;
end

exception Invalid_wire_batch

module Batch = struct
  type t = Signal.t list [@@deriving bin_io, equal, sexp_of]

  let bin_read_t buffer ~pos_ref =
    let count = (Bin_prot.Read.bin_read_nat0 buffer ~pos_ref :> int) in
    if count <= 0 || count > 33 then raise Invalid_wire_batch;
    let rec read remaining reversed =
      if remaining = 0
      then List.rev reversed
      else (
        let signal = Signal.bin_read_t buffer ~pos_ref in
        read (remaining - 1) (signal :: reversed))
    in
    let signals = read count [] in
    if not (Signal.valid_batch signals) then raise Invalid_wire_batch;
    signals
  ;;

  let bin_reader_t = { bin_reader_t with read = bin_read_t }
  let bin_t = { bin_t with reader = bin_reader_t }
end
