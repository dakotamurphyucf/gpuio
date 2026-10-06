(** A native searchable command chooser. Application commands and callbacks
    remain in the enclosing [Command.Registry]. *)
module Search : sig
  type t =
    | All_terms
    | Substring
    | Unfiltered
    | External
  [@@deriving equal, sexp_of]
end

module Escape : sig
  type t =
    | Dismiss
    | Clear_query_first
  [@@deriving equal, sexp_of]
end

module Group : sig
  module Id : sig
    type t [@@deriving equal, compare, sexp_of]

    (** Nonblank UTF-8 without NUL, at most 256 bytes. *)
    val of_string : string -> t Core.Or_error.t

    val to_string : t -> string
  end

  type t [@@deriving equal, sexp_of]

  (** A stable group with an optional passive heading. Empty groups are allowed;
      a group with no matching commands has no visible heading. Labels are
      nonblank UTF-8 without NUL, at most 4096 bytes. *)
  val create
    :  id:Id.t
    -> ?label:string
    -> commands:Command.Id.t list
    -> unit
    -> t Core.Or_error.t

  val id : t -> Id.t
  val label : t -> string option
  val commands : t -> Command.Id.t list
end

module Entry : sig
  type t =
    | Command of Command.Id.t
    | Group of Group.t
    | Separator
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
      keywords. [Substring] trims surrounding Unicode whitespace and matches the remaining
      whole query against the label or an individual keyword, preserving the
      pinned source's substring semantics without changing the stored query.
      [Unfiltered] preserves declared rows regardless of query. Matching uses
      Unicode lowercase, without fuzzy ranking or Unicode normalization.
      Keywords reference known, unique command IDs; at most 64 nonblank UTF-8
      strings without NUL per command, each at most 4096 bytes. All keyword text
      and references count toward the shared 256-KiB metadata budget. Blankness
      follows [Core.String.strip], as for command IDs; non-ASCII spaces remain
      literal keyword text.

      [External] keeps the native query but shows only explicitly published
      results. Before publication and after a native query edit, no result is
      eligible. Results bypass local matching and supply their own ordered groups.
      Changing the configured command IDs invalidates the published result set.
      Registry labels/content remain ordinary View updates; publication does not
      turn those separate updates into one atomic transaction.

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

  (** Grouped presentation over the same native command registry. Commands and
      group IDs must be unique; at most 1024 top-level entries and 1024 commands.
      Group IDs/labels share the 256-KiB metadata budget with search policies.
      Leading, trailing and consecutive separators are suppressed after filtering.
      Headings/separators never participate in keyboard selection. *)
  val create_entries
    :  label:string
    -> entries:Entry.t list
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

module Results : sig
  type t [@@deriving equal, sexp_of]

  (** Ordered references to commands staged in this palette's config and enclosing
      registry. Native publication validates membership atomically. At most 1024
      unique IDs and 256 KiB of metadata, using the same group rules as [Config].
      Row content stays keyed to the original configured command position. *)
  val create : commands:Command.Id.t list -> unit -> t Core.Or_error.t

  val create_entries : entries:Entry.t list -> unit -> t Core.Or_error.t
end

module Dismissal : sig
  type t =
    | Escape
    | Outside_pointer
    | Selected of Command.Id.t
  [@@deriving equal, sexp_of]
end

module Appearance = Choice.Appearance

(** Immutable native state, delivered initially and after observable changes.
    Native typing/navigation never waits for an application callback. *)
module Snapshot : sig
  type t [@@deriving equal, sexp_of]

  val query : t -> string
  val composing : t -> bool

  (** Native loading presentation; independent of query identity and eligibility. *)
  val loading : t -> bool

  val selected : t -> Command.Id.t option

  (** Includes disabled matching commands; excludes passive headings/separators. *)
  val matched_count : t -> int

  (** Same observer lifetime and query revision, including accepted edits and composition transitions.
      Selection/loading-only changes preserve this identity; typing away and back does not.
      Check [composing] before starting application search. This comparison is an
      observation fence, not an atomic native result-publication command. *)
  val same_query : t -> t -> bool
end

module Command : sig
  type t =
    | Read_snapshot
    | Focus
    | Set_query of string
    | Highlight of Command.Id.t option
    | Set_loading of bool
    | Publish_results of Results.t
  [@@deriving equal, sexp_of]
end

(** [Publish_results] requires [Search.External] and a current query fence; the
    App/controller always attaches one. It atomically installs result order and
    clears loading, preserves query/undo, retains an enabled surviving highlight
    or chooses the first enabled result. It rejects composition. Use an accepted
    View lifecycle effect when staging new registry/config commands first. *)
module Command_error = Gpuio_protocol.Palette_command_wire.Error

module Expert : sig
  val window : Snapshot.t -> Gpuio_protocol.Window_id.t
  val node : Snapshot.t -> Gpuio_protocol.Node_id.t
  val observer : Snapshot.t -> Gpuio_protocol.Handler_id.t
  val sequence : Snapshot.t -> int64
  val query_revision : Snapshot.t -> int64
  val same_owner : Snapshot.t -> Snapshot.t -> bool
  val command_to_wire : Command.t -> Gpuio_protocol.Palette_command_wire.Command.t

  val snapshot_of_wire
    :  window:Gpuio_protocol.Window_id.t
    -> node:Gpuio_protocol.Node_id.t
    -> observer:Gpuio_protocol.Handler_id.t
    -> Gpuio_protocol.Palette_state_wire.t
    -> Snapshot.t Core.Or_error.t

  val to_wire : Config.t -> Gpuio_protocol.Wire.Palette.t
  val options : Config.t -> Gpuio_protocol.Palette_options_wire.t option
  val layout : Config.t -> Gpuio_protocol.Palette_layout_wire.t option

  val dismissal
    :  Config.t
    -> Gpuio_protocol.Wire.Palette_dismissal.t
    -> Dismissal.t option
end
