open Core
module W = Gpuio_protocol.Slider_wire
module Axis = W.Axis
module Scale = W.Scale
module Thumb = W.Thumb
module Fill = Gpuio_protocol.Slider_presentation_wire.Fill

module Appearance = struct
  type t =
    { geometry : Gpuio_protocol.Slider_presentation_wire.t
    ; track_color : Color.t option
    ; fill_color : Color.t option
    ; thumb_color : Color.t option
    ; ring_color : Color.t option
    }
  [@@deriving equal, sexp_of]

  let create
        ?(fill = Fill.Selected)
        ?(track_thickness = 4.)
        ?(track_radius = 2.)
        ?(thumb_size = 12.)
        ?(target_size = 20.)
        ?(ring_width = 2.)
        ?track_color
        ?fill_color
        ?thumb_color
        ?ring_color
        ()
    =
    let geometry =
      { Gpuio_protocol.Slider_presentation_wire.default with
        fill
      ; track_thickness
      ; track_radius
      ; thumb_size
      ; target_size
      ; ring_width
      }
    in
    if Gpuio_protocol.Slider_presentation_wire.valid geometry
    then Ok { geometry; track_color; fill_color; thumb_color; ring_color }
    else Or_error.error_string "invalid slider presentation dimensions"
  ;;

  let default = create () |> Or_error.ok_exn
end

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
  let appearance_to_wire (t : Appearance.t) ~theme =
    let open Or_error.Let_syntax in
    let resolve = function
      | None -> return None
      | Some color -> Theme.resolve theme color |> Or_error.map ~f:Option.some
    in
    let%bind track_color = resolve t.track_color in
    let%bind fill_color = resolve t.fill_color in
    let%bind thumb_color = resolve t.thumb_color in
    let%map ring_color = resolve t.ring_color in
    { t.geometry with track_color; fill_color; thumb_color; ring_color }
  ;;

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
