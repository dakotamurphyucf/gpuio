open Core

let max_properties = 65536
let max_message = 16384
let max_components = 64

module Schema = struct
  type t =
    { name : string
    ; version : int64
    ; fingerprint : string
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    let lower c = Char.(c >= 'a' && c <= 'z') in
    let digit c = Char.(c >= '0' && c <= '9') in
    let segments = String.split t.name ~on:'.' in
    String.length t.name <= 128
    && List.length segments >= 2
    && List.for_all segments ~f:(fun segment ->
      (not (String.is_empty segment))
      && lower segment.[0]
      && String.for_all segment ~f:(fun c -> lower c || digit c || Char.equal c '_'))
    && Int64.(t.version >= 1L && t.version <= 65535L)
    && String.length t.fingerprint = 64
    && String.for_all t.fingerprint ~f:(fun c -> digit c || Char.(c >= 'a' && c <= 'f'))
  ;;
end

module Command = struct
  type t =
    { sequence : int64
    ; payload : string
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t = Int64.(t.sequence > 0L) && String.length t.payload <= max_message
end

module Config = struct
  type t =
    { schema : Schema.t
    ; generation : int64
    ; label : string
    ; disabled : bool
    ; properties : string
    ; command : Command.t option
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Schema.valid t.schema
    && Int64.(t.generation > 0L)
    && (not (String.is_empty (String.strip t.label)))
    && String.length t.label <= 1024
    && Stdlib.String.is_valid_utf_8 t.label
    && (not (String.contains t.label '\000'))
    && String.length t.properties <= max_properties
    && Option.for_all t.command ~f:Command.valid
  ;;
end

module Error = struct
  type t =
    | Invalid_schema
    | Incompatible_sdk
    | Duplicate_component
    | Limit_exceeded
    | Unknown_component
    | Incompatible_schema
    | Invalid_properties
    | Invalid_command
    | Closed
    | Hidden
    | Stale
    | Overloaded
    | Panicked
    | Invalid_event
  [@@deriving bin_io, equal, sexp_of]
end

module Signal = struct
  type t =
    | Data of string
    | Mounted
    | Command_completed of int64
    | Failed of Error.t
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Data bytes -> String.length bytes <= max_message
    | Command_completed sequence -> Int64.(sequence > 0L)
    | Mounted | Failed _ -> true
  ;;
end
