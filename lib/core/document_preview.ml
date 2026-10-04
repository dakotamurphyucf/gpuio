open Core

module State = struct
  type t =
    | Pending
    | Collapsed
    | Source_view
    | Rich of { clamped : bool }
  [@@deriving equal, sexp_of]
end

module Event = struct
  type t =
    { source_revision : int64
    ; source_generation : int64
    ; state : State.t
    }
  [@@deriving equal, sexp_of]
end

module Expert = struct
  let event_of_wire (event : Gpuio_protocol.Document_preview_wire.Event.t) =
    if not (Gpuio_protocol.Document_preview_wire.Event.valid event)
    then Or_error.error_string "invalid document preview observation"
    else (
      let state : State.t =
        match event.state with
        | Pending -> Pending
        | Collapsed -> Collapsed
        | Source_view -> Source_view
        | Rich clamped -> Rich { clamped }
      in
      Ok
        { Event.source_revision = event.source_revision
        ; source_generation = event.source_generation
        ; state
        })
  ;;
end
