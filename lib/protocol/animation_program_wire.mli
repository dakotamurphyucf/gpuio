open Core
module A = Animation_wire

module Timing : sig
  type t =
    | Tween of int64 * A.Easing.t
    | Spring of A.Spring.t
  [@@deriving bin_io, equal, sexp_of]
end

module Stage : sig
  type t =
    { targets : A.Target.t list
    ; timing : Timing.t
    ; delay_ms : int64
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Clock : sig
  type t =
    | Independent
    | Application
    | Group of string
  [@@deriving bin_io, equal, sexp_of]
end

module Playback : sig
  type t =
    | Running
    | Paused
    | Cancelled
  [@@deriving bin_io, equal, sexp_of]
end

module Program : sig
  type t =
    { initial : A.Target.t list option
    ; stages : Stage.t list
    ; delay_ms : int64
    ; repeat : A.Repeat.t
    ; clock : Clock.t
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Config : sig
  type t =
    { generation : int64
    ; program : Program.t
    ; playback : Playback.t
    ; restart : int64
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Stage_result : sig
  type t =
    | Played
    | Reduced_motion
  [@@deriving bin_io, equal, sexp_of]
end

module Cancel_reason : sig
  type t =
    | Replaced
    | Removed
    | Window_closed
    | Requested
  [@@deriving bin_io, equal, sexp_of]
end

module Observation : sig
  type t =
    | Stage_completed of int64 * Stage_result.t
    | Finished
    | Cancelled of Cancel_reason.t
  [@@deriving bin_io, equal, sexp_of]
end

module Signal : sig
  type t =
    { generation : int64
    ; index : int64
    ; observation : Observation.t
    }
  [@@deriving bin_io, equal, sexp_of]
end
