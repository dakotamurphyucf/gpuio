type file_descr = Unix.file_descr

open Core

(** Low-level bridge handle. Create on the OS main thread with a dedicated
    nonblocking wake pipe. The native side duplicates its write descriptor.
    Keep the read end alive until [run] returns and all worker activity ends.
    [submit]/[drain] may run on the single OCaml UI domain while [run] owns the
    OS main thread. [dispose] is idempotent and requires [run] to have returned.
    Rust panics become OCaml exceptions at the export boundary. *)
type t

val create : file_descr -> t

(** Creation policy; false keeps the host alive without windows until shutdown. *)
val create_with_options : exit_on_last_window:bool -> file_descr -> t

val run : t -> unit

val submit
  :  t
  -> Gpuio_protocol.Wire.Message.t
  -> (unit, Gpuio_protocol.Wire.Error_code.t) Result.t

val drain : t -> Gpuio_protocol.Wire.Event.t list Or_error.t

(** Emergency wake-and-stop, independent of command queue capacity. Cancels all
    outstanding requests. Used when the OCaml worker fails. *)
val abort : t -> unit

val dispose : t -> unit
