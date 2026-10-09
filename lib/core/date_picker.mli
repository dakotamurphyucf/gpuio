open Core

(** Pure popup lifetime and confirmation policy. The application owns [value];
    the native calendar owns the draft during one opening. No time-zone or I/O
    operation occurs here. [Gpuio_eio.Date_picker] supplies the UI controller. *)
module Error : sig
  type t =
    | Not_open
    | Not_ready
    | Stale_session
    | Stale_draft
    | Disabled
    | Read_only
    | Wrong_mode
    | Incomplete_range
    | Disallowed_selection
    | Limit_exceeded
    | Native of Calendar.Command_error.t
  [@@deriving equal, sexp_of]
end

module Preset : sig
  (** A labeled, complete civil selection. Dates are supplied by the application;
      this module never reads the clock. Empty is useful for a clear preset.
      Partial ranges are rejected. Labels follow [Choice]'s UTF-8/4096-byte
      contract; IDs follow [Choice.Id]'s contract. *)
  type t [@@deriving equal, sexp_of]

  val create
    :  id:Choice.Id.t
    -> label:string
    -> selection:Calendar.Selection.t
    -> t Or_error.t

  val id : t -> Choice.Id.t
  val label : t -> string
  val selection : t -> Calendar.Selection.t

  (** Revalidate against the current mode, constraints and interaction policy.
      A preset may remain visible but unavailable when configuration changes. *)
  val validate : t -> config:Calendar.Config.t -> (unit, Error.t) Result.t

  module Collection : sig
    type preset := t
    type t [@@deriving equal, sexp_of]

    val empty : t
    val max_presets : int

    (** At most 32 presets with distinct IDs, preserving caller order. Duplicate
        labels or selections are allowed. *)
    val create : preset list -> t Or_error.t

    val to_list : t -> preset list
  end
end

module Session : sig
  module Id : sig
    type t [@@deriving compare, equal, sexp_of]

    val to_int64 : t -> int64
  end

  type t [@@deriving equal, sexp_of]

  val id : t -> Id.t
  val original : t -> Calendar.Selection.t
  val initial : t -> Calendar.Selection.t
  val initial_month : t -> Calendar.Month.t
end

type t [@@deriving equal, sexp_of]

val empty : t
val error : t -> Error.t option
val draft : t -> Calendar.Snapshot.t option

(** Changing the application value or calendar mode invalidates an open session.
    Disabling also closes it. Other configuration changes retain its draft;
    confirmation revalidates against the current constraints. *)
val session
  :  t
  -> config:Calendar.Config.t
  -> value:Calendar.Selection.t
  -> Session.t option

val sync : t -> config:Calendar.Config.t -> value:Calendar.Selection.t -> t

(** Opens once; repeated open requests preserve the current session. A historical
    value disallowed by current constraints remains the application value, but
    the new native draft starts empty. Cancelling never clears that value.
    [initial_month] is explicit; no ambient clock is used. *)
val open_popup
  :  t
  -> config:Calendar.Config.t
  -> value:Calendar.Selection.t
  -> initial_month:Calendar.Month.t
  -> t

val cancel : t -> session:Session.Id.t -> t

(** Rejects old session IDs, different native leases and regressing revisions. *)
val observe : t -> session:Session.Id.t -> Calendar.Snapshot.t -> t

(** Only for an event already validated by the retained-view reconciler. A new
    native lease means the same open view was removed and mounted again: accept
    its freshly seeded observation, while old command replies remain fenced by
    [observe]/[confirm]. Never use this entry point for a command reply. *)
val observe_native : t -> session:Session.Id.t -> Calendar.Snapshot.t -> t

val failed : t -> session:Session.Id.t -> Error.t -> t

(** Confirm a freshly read native snapshot. Empty or complete allowed selections
    succeed and close the session. Partial ranges, stale replies and read-only or
    disallowed values fail without applying a value. A new application value
    invalidates confirmation even if the native read was already in flight. *)
val confirm
  :  t
  -> config:Calendar.Config.t
  -> value:Calendar.Selection.t
  -> session:Session.Id.t
  -> Calendar.Snapshot.t
  -> t * (Calendar.Selection.t, Error.t) Result.t
