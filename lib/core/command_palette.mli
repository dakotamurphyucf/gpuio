(** A native searchable command chooser. Application commands and callbacks
    remain in the enclosing [Command.Registry]. *)
module Search : sig
  type t =
    | All_terms
    | Substring
    | Unfiltered
  [@@deriving equal, sexp_of]
end

module Escape : sig
  type t =
    | Dismiss
    | Clear_query_first
  [@@deriving equal, sexp_of]
end

module Config : sig
  type t [@@deriving equal, sexp_of]

  (** Commands are unique, ordered references to enclosing registries; at most
      1024 IDs and 256 KiB total metadata. Labels/placeholder are UTF-8 without
      NUL, at most 4096 bytes; the label is nonblank. Native search matches all
      whitespace-separated query terms against Unicode-lowercase label and ID,
      preserving declared order. Query text is native-owned and limited to
      4096 bytes. It is independent of application document editors.

      [All_terms] matches every whitespace-separated term against label, ID or
      keywords. [Substring] matches the whole query against the label or an
      individual keyword, preserving the pinned source's substring semantics.
      [Unfiltered] preserves declared rows regardless of query. Matching uses
      Unicode lowercase, without fuzzy ranking or Unicode normalization.
      Keywords reference known, unique command IDs; at most 64 nonblank UTF-8
      strings without NUL per command, each at most 4096 bytes. All keyword text
      and references count toward the shared 256-KiB metadata budget. Blankness
      follows [Core.String.strip], as for command IDs; non-ASCII spaces remain
      literal keyword text.

      [searchable=false] hides the query field and bypasses filtering; the private
      query is retained for a later policy change. Native navigation still works.
      [Clear_query_first] clears a nonempty visible query on Escape and dismisses
      on a subsequent Escape. IME composition consumes Escape before either
      policy. Changing policy retains the chooser's owner and captured editor. *)
  val create
    :  label:string
    -> commands:Command.Id.t list
    -> ?placeholder:string
    -> ?dismiss_on_outside_pointer:bool
    -> ?search:Search.t
    -> ?searchable:bool
    -> ?escape:Escape.t
    -> ?keywords:(Command.Id.t * string list) list
    -> unit
    -> t Core.Or_error.t

  val commands : t -> Command.Id.t list
end

module Dismissal : sig
  type t =
    | Escape
    | Outside_pointer
    | Selected of Command.Id.t
  [@@deriving equal, sexp_of]
end

module Appearance = Choice.Appearance

module Expert : sig
  val to_wire : Config.t -> Gpuio_protocol.Wire.Palette.t
  val options : Config.t -> Gpuio_protocol.Palette_options_wire.t option

  val dismissal
    :  Config.t
    -> Gpuio_protocol.Wire.Palette_dismissal.t
    -> Dismissal.t option
end
