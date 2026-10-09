open Core

(** A validated display label for [Presentation.styled_label]. It owns no native
    resources. Matching is computed once at construction, not during painting. *)
module Match : sig
  type t

  (** Queries are valid UTF-8, at most 4096 bytes. Empty queries select no ranges.
      Matching lowercases each Unicode scalar independently: no locale, final-sigma
      context, normalization or full case folding. Thus [SS] does not match [ß]. *)
  val prefix : string -> t Or_error.t

  (** All occurrences, including overlaps. Overlapping/adjacent results coalesce.
      A match inside an expanded lowercase scalar (e.g. [i] in [İ]) colors the
      whole original scalar; transformed offsets never index the original string. *)
  val all : string -> t Or_error.t
end

type t [@@deriving equal, sexp_of]

(** Joins primary and optional secondary with one ASCII space (also when either
    is empty). Combined source and displayed output must each fit 262144 UTF-8
    bytes. Secondary uses a muted foreground; matched ranges take precedence.
    More than 4096 final foreground runs returns an error, never silent truncation.

    [masked=true] replaces each source Unicode scalar, including the separator,
    with one bullet U+2022. The result retains only bullets and no source-dependent
    formatting or query. Copy/default accessibility therefore receive only the
    replacement text. This is display masking, not a secret-entry or memory-erasure
    contract. Caller-owned source values remain the caller's responsibility.

    Matching is linear in the lowered source/query plus the number of occurrences.
    Temporary lowered source is bounded to 1048576 bytes; query to 16384 bytes.
    No case conversion or matching happens for masked values. *)
val create
  :  ?secondary:string
  -> ?highlight:Match.t
  -> ?masked:bool
  -> string
  -> t Or_error.t

val display_text : t -> string

module Expert : sig
  (** Assign colors to already validated semantic runs. Gaps inherit the view's
      foreground. Tokens resolve during normal View reconciliation. *)
  val to_text_content : t -> secondary:Color.t -> highlight:Color.t -> Text_content.t
end
