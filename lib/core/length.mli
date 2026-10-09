open Core

(** Logical pixels or percentage points. The reference depends on the property:
    layout dimensions generally use the parent dimension; line height uses the
    effective font size (162.5 means 1.625 times that size).
    Values must be finite with absolute value at most 1,000,000; negative lengths
    are useful for offsets and margins.
    Properties such as width validate nonnegative values separately. *)
type t [@@deriving equal, sexp_of]

val px : float -> t Or_error.t
val px_exn : float -> t
val percent : float -> t Or_error.t
val percent_exn : float -> t
val auto : t

module Expert : sig
  val to_wire : t -> Gpuio_protocol.Wire.Length.t
end
