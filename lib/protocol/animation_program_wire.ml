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
end
