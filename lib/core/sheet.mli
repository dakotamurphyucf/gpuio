open Core

module Edge : sig
  type t =
    | Left
    | Right
    | Top
    | Bottom
  [@@deriving equal, sexp_of]
end

module Insets : sig
  (** Application-reserved space inside the window content viewport. Each edge
      is finite and in 0..16384 logical pixels. These are not inferred OS safe
      areas. Zero is the default. *)
  type t [@@deriving equal, sexp_of]

  val zero : t

  val create
    :  ?top:float
    -> ?right:float
    -> ?bottom:float
    -> ?left:float
    -> unit
    -> t Or_error.t
end

module Config : sig
  type t [@@deriving equal, sexp_of]

  (** Modal drawer attached to a window edge. Defaults: [Right], extent 360,
      Escape and outside-pointer dismissal enabled. [extent] is width for left/
      right and height for top/bottom, in logical pixels (finite, 1..16384).
      Native layout clamps the extent to the inset viewport and fills the other
      axis. Insets move the panel boundaries only; the modal backdrop still
      blocks the whole content viewport. On a very small window, opposing insets
      shrink proportionally to leave one logical pixel (or the whole viewport
      dimension if smaller). Resizing does not require an application update.
      Sheet geometry takes precedence over panel dimensions and margins; style
      still controls its colors, border, padding and content layout. Padding and
      borders are proportionally compressed across configured visual states if
      needed to fit the panel while leaving one logical pixel for content. *)
  val create
    :  label:string
    -> ?edge:Edge.t
    -> ?extent:float
    -> ?insets:Insets.t
    -> ?dismiss_on_escape:bool
    -> ?dismiss_on_outside_pointer:bool
    -> unit
    -> t Or_error.t
end

module Expert : sig
  val overlay : Config.t -> Overlay.Config.t
  val kind : Config.t -> Gpuio_protocol.Wire.Overlay_kind.t
  val insets : Config.t -> Insets.t option
  val insets_to_wire : Insets.t -> Gpuio_protocol.Sheet_insets_wire.t
end
