(** Bounded, single-domain scheduler inbox. Producers may suspend for capacity. *)
type t

val create : capacity:int -> unit -> t
val try_push : t -> (unit -> unit) -> bool
val push : t -> (unit -> unit) -> unit
val wake : t -> unit
val take_turn : t -> (unit -> unit) list
val await : t -> unit
val close : t -> unit
