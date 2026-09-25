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

module Config : sig
  type t [@@deriving equal, sexp_of]

  (** Policy is immutable for one native placement: remount to change it.
      The label is nonblank single-line UTF-8 without NUL, at most 4096 bytes.
      Flags default to false; ordinary updates preserve code and selection.
      Masking hides visual/accessibility text and disables copy/cut; snapshots
      still contain actual text. This is not a secure-storage abstraction. *)
  val create
    :  policy:Policy.t
    -> label:string
    -> ?masked:bool
    -> ?disabled:bool
    -> ?read_only:bool
    -> ?auto_focus:bool
    -> unit
    -> t Or_error.t

  val policy : t -> Policy.t
  val is_masked : t -> bool
  val is_disabled : t -> bool
  val is_read_only : t -> bool
end

module Revision : sig
  type t [@@deriving compare, equal, sexp_of]

  val of_int64 : int64 -> t Or_error.t
  val to_int64 : t -> int64
end

module Snapshot : sig
  (** Immutable, bound to one window/node lifetime. [value] is canonical accepted
      text. During IME composition, [draft] is a separate Unicode preedit draft;
      otherwise [draft] equals [Value.to_string value]. Selection and composition
      use UTF-8 byte offsets into [draft]; selection preserves direction.
      [is_complete] requires no active composition and exactly the policy length.
      Undo/redo availability describes retained history, not edit permission. *)
  type t [@@deriving equal, sexp_of]

  val revision : t -> Revision.t
  val policy : t -> Policy.t
  val value : t -> Value.t
  val draft : t -> string
  val selection : t -> Text_input.Selection.t
  val composition : t -> Text_input.Selection.t option
  val focused : t -> bool
  val can_undo : t -> bool
  val can_redo : t -> bool
  val is_complete : t -> bool
end

module Event : sig
  (** [Observed] covers mounting, configuration and explicit commands. [Changed]
      covers native value/selection/preedit/focus/history changes. A native edit
      that changes the accepted value to a full code emits [Changed], then
      [Complete] with the next revision; neither can be coalesced away across that
      boundary. Same-value edits, initial state, preedit and programmatic commands
      never emit [Complete]. Native undo/redo may complete a changed value.

      [Rejected] is an ordered boundary and carries the resulting snapshot:
      invalid committed IME text has already restored the original checkpoint.
      Its unexpected-character offset refers to the candidate draft for IME and
      the insertion for ordinary typing/paste. Completion is not authentication. *)
  type t =
    | Observed of Snapshot.t
    | Changed of Snapshot.t
    | Complete of Snapshot.t
    | Rejected of Input_error.t * Snapshot.t
  [@@deriving equal, sexp_of]
end

module Command : sig
  (** Replace/Clear validate and check optional revisions before mutation. They
      remain available when disabled/read-only. Preserve clamps each old selection
      endpoint to the new canonical length; explicit Select must fit exactly.
      A value constructed under another policy must still fit the mounted policy.
      Reset clears undo/redo even when the value is unchanged.

      Replace/Clear/Select/Undo/Redo reject active composition; Cancel_composition
      explicitly restores its checkpoint. Undo/Redo reject disabled/read-only.
      Focus follows native visibility/modal/disabled gates. Commands produce
      observations, never user Complete events. Native leases are also checked
      by the controller; a revision alone does not identify an editor lifetime. *)
  type t =
    | Replace of
        { value : Value.t
        ; selection : Text_input.Selection_policy.t
        ; undo : Text_input.Undo_policy.t
        ; if_revision : Revision.t option
        }
    | Clear of
        { undo : Text_input.Undo_policy.t
        ; if_revision : Revision.t option
        }
    | Select of Text_input.Selection.t
    | Focus
    | Undo
    | Redo
    | Cancel_composition
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
    | Invalid_value
    | Focus_blocked
    | Disabled
    | Read_only
    | Invalid_config
  [@@deriving equal, sexp_of]
end

module Expert : sig
  val config_to_wire : Config.t -> Gpuio_protocol.Otp_wire.Config.t
  val value_to_wire : Value.t -> string
  val policy_to_wire : Policy.t -> Gpuio_protocol.Otp_wire.Policy.t

  val snapshot_of_wire
    :  window:Gpuio_protocol.Window_id.t
    -> node:Gpuio_protocol.Node_id.t
    -> Gpuio_protocol.Otp_wire.Snapshot.t
    -> Snapshot.t Or_error.t

  val event_of_wire
    :  window:Gpuio_protocol.Window_id.t
    -> node:Gpuio_protocol.Node_id.t
    -> Gpuio_protocol.Otp_wire.Event.t
    -> Event.t Or_error.t

  val command_to_wire : Command.t -> Gpuio_protocol.Otp_wire.Command.t
  val error_of_wire : Gpuio_protocol.Otp_wire.Error.t -> Command_error.t
  val window : Snapshot.t -> Gpuio_protocol.Window_id.t
  val node : Snapshot.t -> Gpuio_protocol.Node_id.t
end
