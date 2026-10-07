open Core

(** Field order is part of the protocol. Native admission validates the same bounds. *)
type t =
  { scale_x : float
  ; scale_y : float
  ; rotation_degrees : float
  ; translate_x : float
  ; translate_y : float
  }
[@@deriving bin_io, equal, sexp_of]

let valid t =
  let bounded value limit = Float.is_finite value && Float.(abs value <= limit) in
  bounded t.scale_x 64.
  && bounded t.scale_y 64.
  && bounded t.rotation_degrees 360.
  && bounded t.translate_x 16384.
  && bounded t.translate_y 16384.
;;
