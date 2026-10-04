open Core

module Instance = struct
  type t =
    { schema : Extension_wire.Schema.t
    ; generation : int64
    ; properties : string
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Extension_wire.Schema.valid t.schema
    && Int64.(t.generation > 0L)
    && String.length t.properties <= Extension_wire.max_properties
  ;;
end

module Config = struct
  type t =
    { epoch : int64
    ; instance : Instance.t option
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t = Int64.(t.epoch > 0L) && Option.for_all t.instance ~f:Instance.valid
end

module Stage = struct
  type t =
    | Configure
    | Parse
    | Highlight
    | Render
    | Input
  [@@deriving bin_io, equal, sexp_of]
end

module Error = struct
  type t =
    | Invalid_schema
    | Incompatible_sdk
    | Duplicate_profile
    | Unknown_profile
    | Incompatible_schema
    | Invalid_properties
    | Invalid_plugin
    | Duplicate_plugin
    | Invalid_highlight
    | Invalid_source
    | Parse
    | Render
    | Highlight
    | Limit_exceeded
    | Cancelled
    | Panicked
    | Closed
    | Invalid_event
  [@@deriving bin_io, equal, sexp_of]
end

module Signal = struct
  type t =
    | Data of string
    | Failed of Stage.t * Error.t
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Data bytes -> String.length bytes <= Extension_wire.max_message
    | Failed _ -> true
  ;;
end

module Event = struct
  type t =
    { config_epoch : int64
    ; instance_generation : int64
    ; source_revision : int64
    ; source_generation : int64
    ; signal : Signal.t
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Int64.(
      t.config_epoch > 0L
      && t.instance_generation > 0L
      && t.source_revision > 0L
      && t.source_generation > 0L)
    && Signal.valid t.signal
  ;;
end
