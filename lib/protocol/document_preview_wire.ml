open Core

module Config = struct
  type t =
    { epoch : int64
    ; max_lines : int64 option
    ; observe : bool
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Int64.(t.epoch > 0L)
    && Option.for_all t.max_lines ~f:(fun n -> Int64.(n >= 1L && n <= 4096L))
  ;;
end

module State = struct
  type t =
    | Pending
    | Collapsed
    | Source_view
    | Rich of bool
  [@@deriving bin_io, equal, sexp_of]
end

module Event = struct
  type t =
    { config_epoch : int64
    ; source_revision : int64
    ; source_generation : int64
    ; state : State.t
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Int64.(t.config_epoch > 0L && t.source_revision >= 0L && t.source_generation >= 0L)
  ;;
end
