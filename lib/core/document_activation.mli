open Core

module Source : sig
  type t =
    | Mouse of Pointer.Button.t
    | Keyboard
    | Touch of { long_press : bool }
  [@@deriving equal, sexp_of]
end

(** Input metadata from a document link. Mouse modifiers are sampled at release.
    Keyboard includes synthetic accessibility activation and has no modifiers.
    Touch has no modifiers. Routing/opening remains application-owned. *)
type t = private
  { source : Source.t
  ; modifiers : Pointer.Modifiers.t
  }
[@@deriving equal, sexp_of]

module Expert : sig
  val of_wire : Gpuio_protocol.Document_wire.Activation.t -> t Or_error.t
end
