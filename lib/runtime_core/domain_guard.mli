(** Enforces ownership of graph and scheduler state. *)
type t

val create : unit -> t
val check : t -> unit
