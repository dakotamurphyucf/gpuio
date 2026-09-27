open Core
module W = Gpuio_protocol.Slider_wire
module Axis = W.Axis
module Scale = W.Scale
module Thumb = W.Thumb

module Value = struct
  include W.Value

  let checked v =
    if valid v
    then Ok v
    else Or_error.error_string "slider values must be finite and ordered"
  ;;

  let single value = checked (Single value)
  let range ~lower ~upper = checked (Range { lower; upper })
end

module Config = struct
  type t = W.Config.t [@@deriving equal, sexp_of]

  let create
        ~domain
        ~label
        ?(lower_label = label ^ " lower")
        ?(upper_label = label ^ " upper")
        ?(axis = Axis.Horizontal)
        ?(scale = Scale.Linear)
        ?(disabled = false)
        ?(read_only = false)
        ()
    =
    let t =
      { W.Config.domain = Numeric.Expert.to_wire domain
      ; label
      ; lower_label
      ; upper_label
      ; axis
      ; scale
      ; disabled
      ; read_only
      }
    in
    if W.Config.valid t
    then Ok t
    else Or_error.error_string "invalid slider labels, domain or scale"
  ;;

  let domain t = Numeric.Expert.of_wire t.W.Config.domain |> Or_error.ok_exn
  let is_disabled t = t.W.Config.disabled
  let is_read_only t = t.W.Config.read_only
end

module Revision = struct
  type t = int64 [@@deriving compare, equal, sexp_of]

  let of_int64 t =
    if Int64.(t >= 0L) then Ok t else Or_error.error_string "negative slider revision"
  ;;

  let to_int64 t = t
end

module Snapshot = struct
  type t =
    { window : Gpuio_protocol.Window_id.t
    ; node : Gpuio_protocol.Node_id.t
    ; data : W.Snapshot.t
    }
  [@@deriving equal, sexp_of]

  let revision t = t.data.revision
  let value t = t.data.value
  let committed t = t.data.committed
  let dragging t = t.data.dragging
end

module Source = W.Source
module Cancel_reason = W.Cancel_reason

module Event = struct
  type t =
    | Observed of Snapshot.t
    | Drag_started of Snapshot.t
    | Preview of Snapshot.t
    | Committed of Source.t * Snapshot.t
    | Cancelled of Cancel_reason.t * Snapshot.t
  [@@deriving equal, sexp_of]
end

module Command = W.Command
module Command_error = W.Error

module Expert = struct
  let config_to_wire t = t
  let value_to_wire t = t

  let snapshot_of_wire ~window ~node data =
    if W.Snapshot.valid data
    then Ok { Snapshot.window; node; data }
    else Or_error.error_string "invalid slider snapshot"
  ;;

  let window t = t.Snapshot.window
  let node t = t.Snapshot.node

  let event_of_wire ~window ~node event =
    if not (W.Event.valid event)
    then Or_error.error_string "invalid slider event"
    else (
      let%map.Or_error s = snapshot_of_wire ~window ~node (W.Event.snapshot event) in
      match event with
      | Observed _ -> Event.Observed s
      | Drag_started _ -> Drag_started s
      | Preview _ -> Preview s
      | Committed (source, _) -> Committed (source, s)
      | Cancelled (reason, _) -> Cancelled (reason, s))
  ;;

  let command_to_wire t = t
  let error_of_wire t = t
end
