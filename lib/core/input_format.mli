open Core

(** Formatting of ordinary single-line drafts. Pass a value through
    [Text_input.Config.create]'s [format] argument to configure native editing.
    The conversion helpers are pure and never edit a mounted input. *)
module Pattern : sig
  type t [@@deriving equal, sexp_of]

  (** Nonempty UTF-8 source, at most 1024 bytes / 256 scalars, without control
      characters. [9] is an ASCII digit, [A] an ASCII letter, [#] ASCII
      alphanumeric and [*] one Unicode scalar; other scalars are literals.
      Slots count scalars, independently of grapheme-aware native editing. *)
  val create : string -> t Or_error.t

  val source : t -> string
end

module Number : sig
  type t [@@deriving equal, sexp_of]

  (** Optional one-scalar grouping separator and maximum fraction length.
      Separators cannot be controls, digits, signs or decimal punctuation,
      including their supported full-width equivalents. Fraction length is in
      0..262144. Omitted means no additional fraction limit. No float conversion
      occurs; leading/fractional zeros and incomplete signed decimal drafts remain.
      A zero fraction limit forbids the decimal point. Exponent notation is not
      part of this format. *)
  val create : ?separator:string -> ?fraction_digits:int -> unit -> t Or_error.t

  val separator : t -> string option
  val fraction_digits : t -> int option
end

module Error : sig
  type t =
    | Invalid_text
    | Does_not_fit
    | Limit_exceeded
  [@@deriving equal, sexp_of]
end

type t [@@deriving equal, sexp_of]

val pattern : Pattern.t -> t
val number : Number.t -> t

(** Consume raw slot values and insert literals/grouping. Raw pattern input
    never contains implicit formatting literals: under [-*], raw [-] becomes
    [--]. Empty input stays empty; trailing pattern literals are not invented
    after the last supplied slot. Reject leftover/mismatched input rather than
    dropping it. Number formatting normalizes full-width digits/signs/dot/comma
    before checking its raw, ungrouped decimal grammar. Both input and output are
    bounded to 256 KiB. No operation changes an editor or its undo history. *)
val format_raw : t -> string -> (string, Error.t) Result.t

(** Extract slots from an already formatted draft. Pattern drafts must be exact
    prefixes of the pattern, including literals; a trailing literal prefix is
    allowed even if no following slot has been entered yet. Numeric grouping must
    already be canonical. Preserve precision and trailing zeros, and reject
    noncanonical numeric text rather than silently removing arbitrary separators.
    Formatting a raw value and extracting it is lossless except the documented
    numeric full-width normalization. The converse need not preserve optional
    trailing pattern literals. *)
val raw_of_formatted : t -> string -> (string, Error.t) Result.t

val accepts_formatted : t -> string -> bool

module Expert : sig
  val to_wire : t -> Gpuio_protocol.Input_format_wire.t
  val of_wire : Gpuio_protocol.Input_format_wire.t -> t Or_error.t
end
