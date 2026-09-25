open Core

(** A single native one-time-code editor will use these bounded text contracts.
    Filling the code is not an authentication assertion. *)
module Alphabet : sig
  type t =
    | Digits
    | Ascii_alphanumeric
  [@@deriving equal, sexp_of]
end

module Policy : sig
  type t [@@deriving equal, sexp_of]

  (** [length] is in [1..32]. Letters are case-sensitive and preserved. *)
  val create : length:int -> ?alphabet:Alphabet.t -> unit -> t Or_error.t

  val length : t -> int
  val alphabet : t -> Alphabet.t
end

module Input_error : sig
  (** Offsets count bytes of the original input. Errors contain no code text.
      [Invalid_policy] guards raw protocol data; validated Core policies cannot
      produce it. [Invalid_value] rejects a code from an incompatible policy. *)
  type t =
    | Invalid_policy
    | Input_too_large
    | Invalid_utf8
    | Unexpected_character of { byte_offset : int }
    | Too_long
    | Invalid_value
    | Invalid_selection
  [@@deriving equal, sexp_of]
end

module Value : sig
  (** Canonical ASCII prefix, at most 32 characters. Compatibility with a specific
      policy is checked by constructors and operations, not encoded in its type. *)
  type t [@@deriving equal, sexp_of]

  val empty : t
  val to_string : t -> string
  val length : t -> int
  val fits : t -> policy:Policy.t -> bool
  val is_complete : t -> policy:Policy.t -> bool

  (** Map full-width digits/Latin letters to ASCII, preserve case, reject any
      other character or overlength code. Empty prefixes are valid. *)
  val of_string : Policy.t -> string -> (t, Input_error.t) Result.t

  (** As [of_string], additionally removing ASCII whitespace and ASCII hyphens.
      This does not accept other Unicode spaces or dash characters. *)
  val of_paste : Policy.t -> string -> (t, Input_error.t) Result.t

  (** Replace a directional selection atomically. In a canonical ASCII code,
      UTF-8 byte offsets equal cell indices. The result's caret follows the
      normalized insertion; neither truncation nor partial acceptance occurs. *)
  val replace
    :  t
    -> policy:Policy.t
    -> selection:Text_input.Selection.t
    -> text:string
    -> (t * Text_input.Selection.t, Input_error.t) Result.t

  (** Empty or separator-only paste preserves both value and selection. *)
  val paste
    :  t
    -> policy:Policy.t
    -> selection:Text_input.Selection.t
    -> text:string
    -> (t * Text_input.Selection.t, Input_error.t) Result.t
end

(** Bound raw insertion/paste bytes before normalization, including separators. *)
val max_input_bytes : int
