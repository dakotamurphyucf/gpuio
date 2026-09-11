open Core

(** Logical pixels or percentage points (100 means the full parent dimension).
    Values must be finite; negative lengths are useful for offsets and margins.
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
