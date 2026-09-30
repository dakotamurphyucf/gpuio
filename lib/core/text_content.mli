open Core

(** Validated ordinary text with foreground runs for [View.styled_text]. It is not
    a rich-text editor or a document registration and owns no native resources. *)
module Span : sig
  type t [@@deriving equal, sexp_of]

  (** Half-open UTF-8 byte range, 0 <= start < end <= 262144. Colors may be
      concrete or theme tokens. [create] checks numeric bounds; [Text_content.create]
      checks ordering, text bounds and Unicode scalar boundaries. *)
  val create : start_byte:int -> end_byte:int -> foreground:Color.t -> t Or_error.t

  val start_byte : t -> int
  val end_byte : t -> int
  val foreground : t -> Color.t
end

type t [@@deriving equal, sexp_of]

(** Up to 262144 valid UTF-8 bytes and 4096 sorted, nonoverlapping spans. Empty
    text/spans are allowed. Adjacent spans are allowed; gaps inherit the ordinary
    foreground. Offsets are bytes, not UTF-16 units, scalars or graphemes. *)
val create : ?spans:Span.t list -> string -> t Or_error.t

val text : t -> string
val spans : t -> Span.t list

module Expert : sig
  (** Resolve against the current theme; undefined tokens are recoverable errors.
      Native admission must independently validate the resulting wire value. *)
  val to_wire : t -> theme:Theme.t -> Gpuio_protocol.Text_content_wire.t Or_error.t

  (** Revalidate wire values before exposing an opaque public value. Returned
      spans have concrete colors; token names cannot be recovered from RGBA. *)
  val of_wire : Gpuio_protocol.Text_content_wire.t -> t Or_error.t
end
