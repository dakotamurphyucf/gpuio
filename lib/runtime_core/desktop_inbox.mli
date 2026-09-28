open Core

(** Bounded application-owned input before/after readiness. Single-domain use;
    the runner owns synchronization with the native transport. Entries are raw
    URL strings, validated against configured schemes by the eventual consumer.
    Construction and methods perform no I/O and retain no window handles. *)
type t

module Admission : sig
  type t =
    | Queued
    | Too_large
    | Full
    | Closed
  [@@deriving equal, sexp_of]
end

val max_entries : int
val max_bytes : int
val create : unit -> t

(** Reject newest on overflow, preserving earlier FIFO order. A rejected value
    is not retained. The caller must report rejection; it is not successful
    delivery. Each string is limited to [Gpuio.Deep_link.max_bytes]. *)
val push : t -> string -> Admission.t

(** Monotonic, idempotent readiness. Does nothing after [close]. *)
val ready : t -> unit

(** Returns nothing before readiness or after close. Removal precedes callback
    execution. Repeated equal URLs remain distinct OS requests. *)
val pop : t -> string option

(** Permanently reject new input and release all queued strings. *)
val close : t -> unit

val length : t -> int
val bytes : t -> int
