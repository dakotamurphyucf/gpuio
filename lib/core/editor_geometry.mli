open Core

(** Unclipped bounds from one completed native editor paint, in window-content
    logical pixels. Includes endpoint carets and rendered glyph spans. A caret
    has zero width. This is not a visibility or physical presentation receipt. *)
type t [@@deriving equal, sexp_of]

val revision : t -> Text_input.Revision.t
val x : t -> float
val y : t -> float
val width : t -> float
val height : t -> float

module Expert : sig
  val of_wire : Gpuio_protocol.Editor_geometry_wire.t -> t Or_error.t
end
