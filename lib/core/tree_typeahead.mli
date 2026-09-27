open Core

module Input : sig
  type t [@@deriving sexp_of]

  (** Printable UTF-8 input and its canonical Unicode caseless key are each
      bounded to 256 bytes. [reset] discards the previous prefix. [cycle] permits
      a repeated complete prefix to search after the active item; native input
      enables it for one grapheme. No clock or native handle is retained here. *)
  val create : reset:bool -> cycle:bool -> string -> t Or_error.t
end

(** One bounded search prefix, with no source snapshot, label index or payload.
    Matching uses NFD, Unicode default case folding, then NFD (Unicode 17).
    Accents remain significant; matching is locale-independent. *)
type t [@@deriving sexp_of]

val empty : t
val prefix_bytes : t -> int

(** Search the current visible loaded order once, skipping disabled nodes.
    A fresh/cycling search starts after [active] and wraps. An extension starts
    at [active]; if unmatched, its last input becomes a fresh search. Prefix
    overflow restarts with the new input. No match preserves the caller's cursor.
    Labels are not cached. Reused per-search normalizers stream one label at a
    time, stopping once the prefix decides the match; an ASCII prefix check
    avoids normalizing ordinary labels. *)
val advance
  :  t
  -> _ Tree.t
  -> visible:Tree.Id.t list
  -> active:Tree.Id.t option
  -> Input.t
  -> t * Tree.Id.t option
