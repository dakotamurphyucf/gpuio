open Core

(** Native focus policy for a subtree. A trapped scope claims focus and confines
    Tab/Shift-Tab, pointer focus, accessibility focus and explicit focus commands
    to its eligible descendants. Nested scopes activate in mount order.
    Opening focuses its first eligible painted control, or its non-Tab root when
    empty. Closing restores the prior eligible focus when requested; otherwise
    the enclosing scope/window supplies a fallback. No focus probe visits an
    outside control as part of traversal. *)
type t [@@deriving equal, sexp_of]

(** [auto_focus] requests entry even for an untrapped scope. [trap] always requests
    entry. Mount/unmount determines lifetime; value updates retain the scope. *)
val create : ?trap:bool -> ?auto_focus:bool -> ?restore_focus:bool -> unit -> t

module Expert : sig
  val to_wire : t -> Gpuio_protocol.Wire.Focus_scope.t
end
