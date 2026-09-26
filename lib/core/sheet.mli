open Core

module Edge : sig
  type t =
    | Left
    | Right
    | Top
    | Bottom
  [@@deriving equal, sexp_of]
end

module Config : sig
  type t [@@deriving equal, sexp_of]

  (** Modal drawer attached to a window edge. Defaults: [Right], extent 360,
      Escape and outside-pointer dismissal enabled. [extent] is width for left/
      right and height for top/bottom, in logical pixels (finite, 1..16384).
      Native layout clamps the extent to the viewport and fills the other axis.
      Sheet geometry takes precedence over panel dimensions and margins; style
      still controls its colors, border, padding and content layout. *)
  val create
    :  label:string
    -> ?edge:Edge.t
    -> ?extent:float
    -> ?dismiss_on_escape:bool
    -> ?dismiss_on_outside_pointer:bool
    -> unit
    -> t Or_error.t
end

module Expert : sig
  val overlay : Config.t -> Overlay.Config.t
  val kind : Config.t -> Gpuio_protocol.Wire.Overlay_kind.t
end
