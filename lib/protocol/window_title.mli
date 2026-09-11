open! Core

(** A window title without NUL bytes. The empty title is permitted.
    This value is immutable and has no native resource ownership.
    This scaffold example is not a frozen wire representation. *)
type t [@@deriving equal, sexp_of]

(** Validates a title supplied by the application. Returns an error when the
    input contains a NUL byte; no normalization or truncation is performed. *)
val of_string : string -> t Or_error.t

val to_string : t -> string
