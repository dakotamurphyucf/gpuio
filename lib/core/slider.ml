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
  type t = W.Snapshot.t [@@deriving equal, sexp_of]

  let revision t = t.W.Snapshot.revision
  let value t = t.W.Snapshot.value
  let committed t = t.W.Snapshot.committed
  let dragging t = t.W.Snapshot.dragging
end

module Source = W.Source
module Cancel_reason = W.Cancel_reason
module Event = W.Event
module Command = W.Command
module Command_error = W.Error

module Expert = struct
  let config_to_wire t = t
  let value_to_wire t = t

  let snapshot_of_wire t =
    if W.Snapshot.valid t then Ok t else Or_error.error_string "invalid slider snapshot"
  ;;

  let event_of_wire t =
    if W.Event.valid t then Ok t else Or_error.error_string "invalid slider event"
  ;;

  let command_to_wire t = t
  let error_of_wire t = t
end
