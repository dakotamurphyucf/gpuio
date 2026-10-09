open Core

(** A statically linked rich-document profile. No Rust/parser handle crosses FFI. *)
module Accent : sig
  type t =
    | Indigo
    | Amber
  [@@deriving equal, sexp_of]
end

module Event : sig
  type t =
    | Inspect_code
    | Summarize_table
    | Open_badge
    | Open_card
  [@@deriving equal, sexp_of]
end

val schema : Gpuio.Document.Profile.Schema.t

(** Property changes revoke old callbacks. A larger generation requests a reset.
    Attach the result with [Gpuio_bonsai.View.with_document_profile]. *)
val instance
  :  accent:Accent.t
  -> generation:int64
  -> Event.t Gpuio.Document.Profile.Instance.t Or_error.t
