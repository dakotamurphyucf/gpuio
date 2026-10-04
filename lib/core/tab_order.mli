open Core

(** Native sequential-focus ordering for checkbox, switch, standalone radio and
    managed radio-group Views. Omit the View argument to restore default order. *)
type t [@@deriving equal, sexp_of]

(** Default [tab_stop=true], [index=0]. Index is -1,000,000..1,000,000.
    Native traversal sorts ascending with stable tree order for ties. Negative
    values do not disable traversal; [tab_stop=false] does. This does not remove
    pointer/programmatic/accessibility focus or override disabled/modal policy. *)
val create : ?tab_stop:bool -> ?index:int -> unit -> t Or_error.t

val default : t
val tab_stop : t -> bool
val index : t -> int

module Expert : sig
  val to_wire : t -> Gpuio_protocol.Checkable_wire.Tab_order.t
  val of_wire : Gpuio_protocol.Checkable_wire.Tab_order.t -> t Or_error.t
end
