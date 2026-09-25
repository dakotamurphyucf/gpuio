open Core
module Geometry = Canvas_wire

let max_bytes = 4 * 1024 * 1024
let max_items = 20_000
let max_resources = 4096
let max_path_commands = 65_536
let max_text_bytes = 1024 * 1024
let max_interactive_items = 2048
let max_clips = 8

module Resource_key = struct
  type t =
    { id : int64
    ; generation : int64
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Text = struct
  type t =
    { value : string
    ; font_family : string
    ; font_size : float
    ; font_weight : int64
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Resource = struct
  module Data = struct
    type t =
      | Path of Geometry.Path.t
      | Text of Text.t
      | Image of Resource_id.t
    [@@deriving bin_io, equal, sexp_of]
  end

  type t =
    { key : Resource_key.t
    ; data : Data.t
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Stroke = struct
  type t =
    { color : int64
    ; width : float
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Paint = struct
  type t =
    { fill : int64 option
    ; stroke : Stroke.t option
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Shape = struct
  type t =
    | Rectangle of Geometry.Rect.t
    | Ellipse of Geometry.Rect.t
    | Path of Resource_key.t
  [@@deriving bin_io, equal, sexp_of]
end

module Drawing = struct
  type t =
    | Shape of Shape.t * Paint.t
    | Text of Resource_key.t * Geometry.Point.t * int64
    | Image of Resource_key.t * Geometry.Rect.t
  [@@deriving bin_io, equal, sexp_of]
end

module Interaction = struct
  type t =
    { label : string
    ; hit_region : Geometry.Hit_region.t
    ; draggable : bool
    ; activatable : bool
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Item = struct
  type t =
    { id : int64
    ; transform : Geometry.Transform.t
    ; clips : Geometry.Rect.t list
    ; drawing : Drawing.t
    ; interaction : Interaction.t option
    }
  [@@deriving bin_io, equal, sexp_of]
end

(** Versioned immutable publication; item order is back-to-front. Resource IDs
    are unique within a snapshot; references require the exact generation. *)
type t =
  { version : int64
  ; description : string
  ; resources : Resource.t list
  ; items : Item.t list
  }
[@@deriving bin_io, equal, sexp_of]
