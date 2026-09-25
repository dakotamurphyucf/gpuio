open Core
module Wire = Gpuio_protocol.Canvas_view_wire
module Geometry = Canvas_geometry

module Viewport = struct
  type t =
    { origin : Geometry.Point.t
    ; zoom : float
    }
  [@@deriving equal, sexp_of]

  let to_wire t : Wire.Viewport.t =
    { origin = Geometry.Expert.point_to_wire t.origin; zoom = t.zoom }
  ;;

  let create ~origin ~zoom =
    let t = { origin; zoom } in
    if Wire.Viewport.valid (to_wire t)
    then Ok t
    else Or_error.error_string "canvas viewport zoom must be finite and in [0.05,64]"
  ;;

  let default =
    { origin = Geometry.Point.create ~x:0. ~y:0. |> Or_error.ok_exn; zoom = 1. }
  ;;

  let origin t = t.origin
  let zoom t = t.zoom

  let of_wire (t : Wire.Viewport.t) =
    let%bind.Or_error origin = Geometry.Point.create ~x:t.origin.x ~y:t.origin.y in
    create ~origin ~zoom:t.zoom
  ;;
end

module Command = struct
  module Action = struct
    type t =
      | Select of Canvas_scene.Item_id.t option
      | Set_viewport of Viewport.t
      | Reset_viewport
      | Reset_positions
    [@@deriving equal, sexp_of]

    let to_wire : t -> Wire.Command.Action.t = function
      | Select id -> Select (Option.map id ~f:Canvas_scene.Item_id.to_int64)
      | Set_viewport viewport -> Set_viewport (Viewport.to_wire viewport)
      | Reset_viewport -> Reset_viewport
      | Reset_positions -> Reset_positions
    ;;
  end

  type t =
    { sequence : int64
    ; action : Action.t
    }
  [@@deriving equal, sexp_of]

  let create ~sequence action =
    if Int64.(sequence > 0L)
    then Ok { sequence; action }
    else Or_error.error_string "canvas command sequence must be positive"
  ;;

  let to_wire t : Wire.Command.t =
    { sequence = t.sequence; action = Action.to_wire t.action }
  ;;
end

module Error = Wire.Error

module Observation = struct
  type t =
    | Selection_changed of Canvas_scene.Item_id.t option
    | Activated of Canvas_scene.Item_id.t
    | Moved of Canvas_scene.Item_id.t * Geometry.Transform.t
    | Viewport_changed of Viewport.t
    | Command_completed of int64
    | Failed of Error.t
  [@@deriving equal, sexp_of]

  let of_wire wire =
    let open Or_error.Let_syntax in
    if not (Wire.Observation.valid wire)
    then Or_error.error_string "invalid canvas observation"
    else (
      match wire with
      | Selection_changed None -> Ok (Selection_changed None)
      | Selection_changed (Some id) ->
        let%map id = Canvas_scene.Item_id.of_int64 id in
        Selection_changed (Some id)
      | Activated id ->
        let%map id = Canvas_scene.Item_id.of_int64 id in
        Activated id
      | Moved (id, t) ->
        let%bind id = Canvas_scene.Item_id.of_int64 id in
        let%map transform =
          Geometry.Transform.create ~a:t.a ~b:t.b ~c:t.c ~d:t.d ~tx:t.tx ~ty:t.ty
        in
        Moved (id, transform)
      | Viewport_changed viewport ->
        let%map viewport = Viewport.of_wire viewport in
        Viewport_changed viewport
      | Command_completed sequence -> Ok (Command_completed sequence)
      | Failed error -> Ok (Failed error))
  ;;
end

module Event = struct
  type t =
    { scene_revision : int64
    ; scene_generation : int64
    ; observation : Observation.t
    }
  [@@deriving equal, sexp_of]
end

module Config = struct
  type t =
    { scene : Canvas_scene.Handle.t
    ; wire : Wire.Config.t
    }
  [@@deriving equal, sexp_of]

  let create
        ~scene
        ?(label = "Canvas")
        ?(initial_viewport = Viewport.default)
        ?(minimum_zoom = 0.05)
        ?(maximum_zoom = 64.)
        ?(selectable = true)
        ?(draggable = true)
        ?(pan_zoom = true)
        ?(disabled = false)
        ?(selection_color = Color.rgb_exn 0x8b5cf6)
        ?(theme = Theme.default)
        ?command
        ()
    =
    let%bind.Or_error selection_color = Theme.resolve theme selection_color in
    let wire : Wire.Config.t =
      { source = None
      ; label
      ; initial_viewport = Viewport.to_wire initial_viewport
      ; minimum_zoom
      ; maximum_zoom
      ; selectable
      ; draggable
      ; pan_zoom
      ; disabled
      ; selection_color
      ; command = Option.map command ~f:Command.to_wire
      }
    in
    if Wire.Config.valid wire
    then Ok { scene; wire }
    else Or_error.error_string "invalid canvas label or viewport limits"
  ;;

  let scene t = t.scene
end

module Expert = struct
  let to_wire (t : Config.t) ~owner =
    let source =
      Option.bind owner ~f:(fun owner ->
        if Canvas_scene.Expert.belongs_to t.scene ~owner
        then Some (Canvas_scene.Expert.native_id t.scene)
        else None)
    in
    { t.wire with source }
  ;;

  let event ~scene_revision ~scene_generation observation =
    let identity_valid =
      Int64.(scene_revision > 0L && scene_generation > 0L)
      || (Int64.(scene_revision = 0L && scene_generation = 0L)
          &&
          match observation with
          | Wire.Observation.Failed _ -> true
          | _ -> false)
    in
    if not identity_valid
    then Or_error.error_string "canvas event requires a published scene identity"
    else (
      let%map.Or_error observation = Observation.of_wire observation in
      { Event.scene_revision; scene_generation; observation })
  ;;
end
