open Core

(** Deterministic fixtures; no network, credentials, wall clock or randomness. *)
module Config : sig
  type t [@@deriving equal, sexp_of]

  val create
    :  ?chunk_bytes:int
    -> ?delay_seconds:float
    -> ?accept_delay_seconds:float
    -> ?fail_after_chunks:int
    -> unit
    -> t Or_error.t

  val delay_seconds : t -> float
  val accept_delay_seconds : t -> float
end

module Step : sig
  (** Chunks are bytes and may split UTF-8 scalars, exercising the document
      decoder. A failure is terminal and replaces Finish. *)
  type t =
    | Chunk of string
    | Fail of string
    | Finish
  [@@deriving equal, sexp_of]
end

val response : prompt:string -> string
val plan : Config.t -> prompt:string -> Step.t list
val markdown_fixture : string
val code_fixture : string
val diff_fixture : string
