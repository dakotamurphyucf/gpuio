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

module Draft : sig
  (** A mount seed, independent of the committed value. Single-line UTF-8 without
      NUL, at most 4096 bytes. Empty, incomplete and invalid numeric expressions
      are permitted; classification and committing use the current domain.
      This contains no native selection, undo history or IME composition. *)
  type t [@@deriving equal, sexp_of]

  val of_string : string -> t Or_error.t
  val to_string : t -> string
end

module Step_controls : sig
  type t =
    | Hidden
    | Sides
    | Stacked
  [@@deriving equal, sexp_of]
end

module Appearance : sig
  type t [@@deriving equal, sexp_of]

  (** Presentation for [View.number_frame], independent of editing policy.
      Geometry is finite: [gap]/[editor_padding] in 0..256, [button_width] and
      both minimum heights in 1..256, [border_width] in 0..64. Defaults are
      4, 0, 24, 20 and 16 respectively; omitted [border_width] preserves
      the outer View border widths.

      Frame/editor styles accept Base, Focused and Disabled. Button styles
      accept Base, Hovered, Pressed and Disabled. Supported properties are
      background, foreground, opacity, border color, shadows, corner radii and
      typography (font size/family/weight, alignment, line height, whitespace,
      overflow, line clamp, decoration). Geometry and interaction fields are
      rejected; at most 128 declarations are allowed across all parts.
      Frame appearance refines outer View style; focus follows the actual
      editor, not focus in an auxiliary slot. Theme tokens resolve atomically. *)
  val create
    :  ?gap:float
    -> ?button_width:float
    -> ?button_min_height:float
    -> ?stacked_button_min_height:float
    -> ?editor_padding:float
    -> ?border_width:float
    -> ?frame_style:Style.t
    -> ?editor_style:Style.t
    -> ?decrement_style:Style.t
    -> ?increment_style:Style.t
    -> unit
    -> t Or_error.t

  val default : t
end

module Step_mode : sig
  (** [Native] uses the fixed domain step. [Application] emits a guarded request
      before user stepping; the application must resolve or decline it.
      Programmatic [Step] commands continue to use the native fixed step. *)
  type t =
    | Native
    | Application
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
    -> ?step_mode:Step_mode.t
    -> ?step_controls:Step_controls.t
    -> ?allow_empty:bool
    -> ?disabled:bool
    -> ?read_only:bool
    -> ?auto_focus:bool
    -> unit
    -> t Or_error.t

  val step_mode : t -> Step_mode.t
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

module Step_request : sig
  (** One native intent, bound to its original editor lifetime and revision.
      Requests are obtained only from [Event.Step_requested]. *)
  type t [@@deriving equal, sexp_of]

  val snapshot : t -> Snapshot.t
  val direction : t -> Numeric.Direction.t
  val source : t -> Source.t
end

module Step_resolution : sig
  type t =
    | Apply of Value.t
    | Decline
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
    | Step_requested of Step_request.t
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
    (** Consumes one still-current request. Applying records an undoable commit
        with the original source; declining leaves the value unchanged. A stale
        reply cannot consume a newer request. Disabled/read-only and native
        interaction gates apply, unlike ordinary explicit replacements. *)
    | Resolve_step of Step_request.t * Step_resolution.t
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
  val appearance_to_wire
    :  Appearance.t
    -> theme:Theme.t
    -> Gpuio_protocol.Wire.Number_presentation.t Or_error.t

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
  val command_snapshot : Command.t -> Snapshot.t option
  val error_of_wire : Gpuio_protocol.Number_input_wire.Error.t -> Command_error.t
  val window : Snapshot.t -> Gpuio_protocol.Window_id.t
  val node : Snapshot.t -> Gpuio_protocol.Node_id.t
end
