open Core
module Geometry = Canvas_wire

module Viewport = struct
  type t =
    { origin : Geometry.Point.t
    ; zoom : float
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Geometry.Point.valid t.origin
    && Float.is_finite t.zoom
    && Float.(t.zoom >= 0.05 && t.zoom <= 64.)
  ;;
end

module Command = struct
  module Action = struct
    type t =
      | Select of int64 option
      | Set_viewport of Viewport.t
      | Reset_viewport
      | Reset_positions
    [@@deriving bin_io, equal, sexp_of]

    let valid = function
      | Select id -> Option.for_all id ~f:(fun id -> Int64.(id > 0L))
      | Set_viewport viewport -> Viewport.valid viewport
      | Reset_viewport | Reset_positions -> true
    ;;
  end

  type t =
    { sequence : int64
    ; action : Action.t
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t = Int64.(t.sequence > 0L) && Action.valid t.action
end

module Config = struct
  type t =
    { source : Resource_id.t option
    ; label : string
    ; initial_viewport : Viewport.t
    ; minimum_zoom : float
    ; maximum_zoom : float
    ; selectable : bool
    ; draggable : bool
    ; pan_zoom : bool
    ; disabled : bool
    ; selection_color : int64
    ; command : Command.t option
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    (not (String.is_empty (String.strip t.label)))
    && String.length t.label <= 1024
    && Stdlib.String.is_valid_utf_8 t.label
    && (not (String.contains t.label '\000'))
    && Viewport.valid t.initial_viewport
    && Float.is_finite t.minimum_zoom
    && Float.is_finite t.maximum_zoom
    && Float.(
         t.minimum_zoom >= 0.05
         && t.maximum_zoom <= 64.
         && t.minimum_zoom <= t.initial_viewport.zoom
         && t.initial_viewport.zoom <= t.maximum_zoom)
    && Int64.(t.selection_color >= 0L && t.selection_color <= 0xffff_ffffL)
    && Option.for_all t.command ~f:Command.valid
  ;;
end

module Error = struct
  type t =
    | Wrong_application
    | Unavailable_scene
    | Render_limit
    | Unavailable_image
    | Invalid_command
    | Native_failure
  [@@deriving bin_io, equal, sexp_of]
end

module Observation = struct
  type t =
    | Selection_changed of int64 option
    | Activated of int64
    | Moved of int64 * Geometry.Transform.t
    | Viewport_changed of Viewport.t
    | Command_completed of int64
    | Failed of Error.t
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Selection_changed id -> Option.for_all id ~f:(fun id -> Int64.(id > 0L))
    | Activated id | Command_completed id -> Int64.(id > 0L)
    | Moved (id, transform) -> Int64.(id > 0L) && Geometry.Transform.valid transform
    | Viewport_changed viewport -> Viewport.valid viewport
    | Failed _ -> true
  ;;
end
