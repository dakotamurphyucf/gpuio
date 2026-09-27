open Core

(** Accepted local simulation parameters. Changes apply to future sends, never
    rewrite an active stream. Chunk sizes are bytes, not tokens or characters. *)
type t [@@deriving equal, sexp_of]

val default : t
val chunk_bytes : t -> int
val interval_ms : t -> int
val with_chunk_bytes : t -> int -> t Or_error.t
val with_interval_ms : t -> int -> t Or_error.t
val backend : t -> Conversation.Backend.Config.t
