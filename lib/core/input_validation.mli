open Core

(** Native ordinary-input edit filtering. Prepare a regex once, wrap it with
    [regex], then pass the filter to [Text_input.Config.create]. Preparing a regex
    alone does not attach a policy or edit an input. *)
module Regex : sig
  module Matching : sig
    type t =
      | Whole_value
      | Substring
    [@@deriving equal, sexp_of]
  end

  module Source : sig
    type t [@@deriving equal, sexp_of]

    (** A bounded Rust-regex source, not a syntax-checked expression. At most
        2048 UTF-8 bytes without NUL. Empty source is permitted. Whole-value
        matching is the default and cannot be weakened by an alternative or
        embedded multiline flag. Backreferences and lookaround are unsupported.
        Use [Gpuio_eio.Input_validation.prepare_regex] to obtain a checked value. *)
    val create : ?matching:Matching.t -> ?case_sensitive:bool -> string -> t Or_error.t

    val pattern : t -> string
    val matching : t -> Matching.t
    val case_sensitive : t -> bool
  end

  (** Immutable checked source data. Contains no Rust handle, callback, native
      allocation or window lifetime. Native ingress still revalidates it. *)
  type t [@@deriving equal, sexp_of]

  val source : t -> Source.t
end

(** A native edit filter, not form/submission validity. Use a prefix-compatible
    expression if a value must be typed incrementally. Mounting or changing a
    filter retains the draft, and undo may restore incompatible history. An
    incompatible draft remains editable until repaired; submissions still carry
    the exact draft and require application business validation. *)
type t [@@deriving equal, sexp_of]

(** Empty text is accepted by default, independently of the expression, so an
    ordinary field can be cleared. With [allow_empty=false], the expression itself
    decides whether empty text is accepted. Filtering runs on formatted text when a format
    is configured. No synchronous OCaml predicate is invoked by native editing. *)
val regex : ?allow_empty:bool -> Regex.t -> t

module Error : sig
  type t =
    | Invalid_source
    | Invalid_regex of string
    | Too_complex
    | Native_failure
  [@@deriving equal, sexp_of]
end

module Expert : sig
  val to_wire : t -> Gpuio_protocol.Input_validation_wire.Rule.t
  val source_to_wire : Regex.Source.t -> Gpuio_protocol.Input_validation_wire.Source.t
  val checked_source : Regex.Source.t -> Regex.t
  val error_of_wire : Gpuio_protocol.Input_validation_wire.Error.t -> Error.t
end
