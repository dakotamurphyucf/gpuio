open Core

(** Pure popup lifetime and confirmation policy. The application owns the confirmed
    value; one retained native color input owns the draft for each opening. *)
module Error : sig
  type t =
    | Not_open
    | Not_ready
    | Stale_session
    | Stale_draft
    | Disabled
    | Read_only
    | Composing
    | Invalid_draft
    | Drag_in_progress
    | Disallowed_value
    | Limit_exceeded
    | Native of Color_input.Command_error.t
  [@@deriving equal, sexp_of]
end

module Session : sig
  module Id : sig
    type t [@@deriving compare, equal, sexp_of]

    val to_int64 : t -> int64
  end

  type t [@@deriving equal, sexp_of]

  val id : t -> Id.t
  val original : t -> Color_value.Value.t
  val initial : t -> Color_value.Value.t
end

type t [@@deriving equal, sexp_of]

val empty : t
val error : t -> Error.t option
val draft : t -> Color_input.Snapshot.t option

(** A changed application value or disabling invalidates the opening. Other policy
    changes retain the native draft; Apply always checks current policy again. *)
val session
  :  t
  -> config:Color_input.Config.t
  -> value:Color_value.Value.t
  -> Session.t option

val sync : t -> config:Color_input.Config.t -> value:Color_value.Value.t -> t

(** Repeated open preserves the current session. If current policy disallows the
    application's historical value, seed Empty when allowed; otherwise use the
    original RGB with opaque alpha, or opaque black for a disallowed Empty value.
    This draft fallback never mutates the confirmed application value. *)
val open_popup : t -> config:Color_input.Config.t -> value:Color_value.Value.t -> t

val cancel : t -> session:Session.Id.t -> t

(** Command observations require the same opening, native lease and a revision
    no older than the last accepted observation. *)
val observe : t -> session:Session.Id.t -> Color_input.Snapshot.t -> t

(** Only for reconciler-validated native observations. A new native lease denotes
    remounting the open view and may replace the old draft; replies may not. *)
val observe_native : t -> session:Session.Id.t -> Color_input.Snapshot.t -> t

val failed : t -> session:Session.Id.t -> Error.t -> t

(** Validate one draft against current policy. Reject marked text, invalid raw
    drafts and unfinished pointer drags. A valid noncomposing text preview may be
    applied without separately pressing Enter. This does not commit native state. *)
val candidate
  :  Color_input.Snapshot.t
  -> config:Color_input.Config.t
  -> (Color_value.Value.t, Error.t) Result.t

(** Confirm a freshly read native draft for the current opening and application
    value. Success closes the session and returns the new application value.
    Old replies cannot confirm or close a later opening or a remounted draft. *)
val confirm
  :  t
  -> config:Color_input.Config.t
  -> value:Color_value.Value.t
  -> session:Session.Id.t
  -> Color_input.Snapshot.t
  -> t * (Color_value.Value.t, Error.t) Result.t
