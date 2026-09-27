open Core

(** Contracts for a native numeric editor. Draft text and committed application
    value are distinct; observing a snapshot never replaces native text. *)
module Value : sig
  (** [Empty] also represents an unfilled required field. [of_float] rejects
      non-finite numbers and canonicalizes zero; domain normalization happens
      when mounting, replacing a value or committing a draft. *)
  type t = private
    | Empty
    | Number of float
  [@@deriving equal, sexp_of]

  val empty : t
  val of_float : float -> t Or_error.t
end

module Step_controls : sig
  type t =
    | Hidden
    | Sides
    | Stacked
  [@@deriving equal, sexp_of]
end

module Config : sig
  type t [@@deriving equal, sexp_of]

  (** Labels must be nonblank single-line UTF-8 without NUL. Each label and
      placeholder is bounded to [max_draft_bytes] bytes. Step controls default
      to [Sides]; Boolean options default to [false]. The two step labels
      default to [label ^ " increase"] and [label ^ " decrease"].

      [allow_empty] governs committing, not initial or explicit empty values.
      Updating a domain preserves the draft and normalizes the committed value. *)
  val create
    :  domain:Numeric.Domain.t
    -> label:string
    -> ?placeholder:string
    -> ?increment_label:string
    -> ?decrement_label:string
    -> ?step_controls:Step_controls.t
    -> ?allow_empty:bool
    -> ?disabled:bool
    -> ?read_only:bool
    -> ?auto_focus:bool
    -> unit
    -> t Or_error.t

  val domain : t -> Numeric.Domain.t
  val allows_empty : t -> bool
  val is_disabled : t -> bool
  val is_read_only : t -> bool
end

module Revision : sig
  (** Numeric observation sequence, including semantic commit/rejection events;
      distinct from the inner editor's text-edit revision. *)
  type t [@@deriving compare, equal, sexp_of]

  val of_int64 : int64 -> t Or_error.t
  val to_int64 : t -> int64
end

module Snapshot : sig
  (** Immutable observation bound to one window/node lifetime. [domain] is the
      domain at observation time; [classification] uses that domain even after
      a configuration update. Selection and composition use UTF-8 byte offsets;
      selection preserves direction, composition is an ordered range. *)
  type t [@@deriving equal, sexp_of]

  val revision : t -> Revision.t
  val domain : t -> Numeric.Domain.t
  val draft : t -> string
  val classification : t -> Numeric.Draft.t
  val committed : t -> Value.t
  val selection : t -> Text_input.Selection.t
  val composition : t -> Text_input.Selection.t option
  val focused : t -> bool
end

module Source : sig
  type t =
    | Keyboard
    | Stepper
    | Accessibility
    | Programmatic
  [@@deriving equal, sexp_of]
end

module Rejection : sig
  type t =
    | Empty_required
    | Incomplete
    | Syntax
    | Non_finite
    | Composing
  [@@deriving equal, sexp_of]
end

module Cancel_reason : sig
  type t =
    | Escape
    | Programmatic
  [@@deriving equal, sexp_of]
end

module Event : sig
  (** [Observed] includes mounting and configuration changes. [Changed] reports
      native draft, selection, composition or focus changes. Commit/cancel and
      rejection are ordered semantic boundaries, even without a text change.
      A rejected commit preserves draft and selection. Losing focus does not
      implicitly commit. *)
  type t =
    | Observed of Snapshot.t
    | Changed of Snapshot.t
    | Committed of Source.t * Snapshot.t
    | Rejected of Rejection.t * Snapshot.t
    | Cancelled of Cancel_reason.t * Snapshot.t
  [@@deriving equal, sexp_of]
end

module Command : sig
  (** Replacements are explicit, including while disabled/read-only. Guards
      compare numeric revisions; the Eio controller also fences native identity.
      All mutating commands reject active composition. Undo/redo restore drafts,
      leaving the last committed value unchanged until another commit.

      [Commit] normalizes a finite draft, including clamping out-of-range values;
      empty acceptance follows the configuration. [Cancel] restores committed
      text. [Step] rejects disabled/read-only or invalid/incomplete drafts;
      an empty draft seeds normalized zero without an additional step.
      [Replace_value] may set [Empty] even for a required field. These value
      replacements produce observations, not user-commit events. *)
  type t =
    | Replace_draft of
        { text : string
        ; selection : Text_input.Selection_policy.t
        ; undo : Text_input.Undo_policy.t
        ; if_revision : Revision.t option
        }
    | Replace_value of
        { value : Value.t
        ; selection : Text_input.Selection_policy.t
        ; undo : Text_input.Undo_policy.t
        ; if_revision : Revision.t option
        }
    | Select of Text_input.Selection.t
    | Focus
    | Undo
    | Redo
    | Commit
    | Cancel
    | Step of Numeric.Direction.t
    | Read_snapshot
  [@@deriving equal, sexp_of]
end

module Command_error : sig
  type t =
    | Not_mounted
    | Closed
    | Stale_input
    | Stale_revision
    | Composing
    | Invalid_selection
    | Limit_exceeded
    | Busy
    | Native_failure
    | Invalid_text
    | Invalid_value
    | Focus_blocked
    | Disabled
    | Read_only
    | Invalid_config
    | Rejected of Rejection.t
  [@@deriving equal, sexp_of]
end

val max_draft_bytes : int
val validate_draft : string -> unit Or_error.t

module Expert : sig
  val config_to_wire : Config.t -> Gpuio_protocol.Number_input_wire.Config.t
  val value_to_wire : Value.t -> Gpuio_protocol.Number_input_wire.Value.t

  val snapshot_of_wire
    :  window:Gpuio_protocol.Window_id.t
    -> node:Gpuio_protocol.Node_id.t
    -> Gpuio_protocol.Number_input_wire.Snapshot.t
    -> Snapshot.t Or_error.t

  val event_of_wire
    :  window:Gpuio_protocol.Window_id.t
    -> node:Gpuio_protocol.Node_id.t
    -> Gpuio_protocol.Number_input_wire.Event.t
    -> Event.t Or_error.t

  val command_to_wire : Command.t -> Gpuio_protocol.Number_input_wire.Command.t
  val error_of_wire : Gpuio_protocol.Number_input_wire.Error.t -> Command_error.t
  val window : Snapshot.t -> Gpuio_protocol.Window_id.t
  val node : Snapshot.t -> Gpuio_protocol.Node_id.t
end
