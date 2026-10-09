open Core

(** Native editor geometry in logical pixels. It describes the most recent
    completed layout, which can precede pending text/layout changes. It is not a
    physical presentation acknowledgement. *)
module Offset : sig
  type t [@@deriving equal, sexp_of]

  (** Nonnegative distances from the document's left/top edges, each at most
      1e9 logical pixels. Native scrolling clamps these to the available extent. *)
  val create : x:float -> y:float -> t Or_error.t

  val origin : t
  val x : t -> float
  val y : t -> float
end

type t [@@deriving equal, sexp_of]

val offset : t -> Offset.t
val width : t -> float
val height : t -> float
val line_height : t -> float

(** Zero-based logical buffer lines, with an exclusive upper limit. Wrapped
    display rows are not counted separately. This is the native laid-out range:
    it includes overscan lines beyond the visible viewport. *)
val first_buffer_line : t -> int

val buffer_line_limit : t -> int

module Expert : sig
  val offset_to_wire : Offset.t -> Gpuio_protocol.Editor_viewport_wire.Offset.t
  val of_wire : Gpuio_protocol.Editor_viewport_wire.t -> t Or_error.t
end
