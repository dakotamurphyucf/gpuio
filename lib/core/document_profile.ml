open Core
module Wire = Gpuio_protocol.Document_profile_wire
module Schema = Extension.Schema
module Codec = Extension.Codec
module Stage = Wire.Stage
module Error = Wire.Error

module Definition = struct
  type ('properties, 'event) t =
    { schema : Schema.t
    ; properties : 'properties Codec.t
    ; events : 'event Codec.t
    }

  let create ~schema ~properties ~events =
    if Codec.max_bytes properties > 65536 || Codec.max_bytes events > 16384
    then Or_error.error_string "document profile codec exceeds its byte limit"
    else Ok { schema; properties; events }
  ;;

  let schema t = t.schema
end

module Instance = struct
  type 'event t =
    { schema : Schema.t
    ; wire : Wire.Instance.t
    ; events : 'event Codec.t
    }

  let create (definition : (_, _) Definition.t) ~generation properties =
    if Int64.(generation <= 0L)
    then Or_error.error_string "document profile generation must be positive"
    else (
      let%map.Or_error properties = Codec.encode definition.properties properties in
      { schema = definition.schema
      ; wire =
          { schema = Schema.Expert.to_wire definition.schema; generation; properties }
      ; events = definition.events
      })
  ;;

  let generation t = t.wire.generation
  let schema t = t.schema
end

module Signal = struct
  type 'a t =
    | Data of 'a
    | Failed of
        { stage : Stage.t
        ; error : Error.t
        }
  [@@deriving sexp_of]
end

module Event = struct
  type 'a t =
    { source_revision : int64
    ; source_generation : int64
    ; signal : 'a Signal.t
    }
  [@@deriving sexp_of]
end

module Expert = struct
  let to_wire (t : _ Instance.t) = t.wire

  let event (t : _ Instance.t) (event : Wire.Event.t) =
    if
      not
        (Wire.Event.valid event && Int64.equal t.wire.generation event.instance_generation)
    then Or_error.error_string "invalid or obsolete document profile event"
    else (
      let signal =
        match event.signal with
        | Failed (stage, error) -> Signal.Failed { stage; error }
        | Data bytes ->
          (match Codec.decode t.events bytes with
           | Ok value -> Signal.Data value
           | Error _ -> Signal.Failed { stage = Input; error = Invalid_event })
      in
      Ok
        { Event.source_revision = event.source_revision
        ; source_generation = event.source_generation
        ; signal
        })
  ;;
end
