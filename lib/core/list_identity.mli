open Core

type t

val prepare : t option -> Virtual_list.Order.t -> t Or_error.t
val order : t -> Gpuio_protocol.List_wire.Order.t
val id : t -> Key.t -> int64 option
val key : t -> int64 -> Key.t option
