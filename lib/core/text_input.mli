open Core

(** Value contracts for native-owned inputs. A snapshot is an observation, never
    an implicit replacement command. The Eio adapter supplies Bonsai controllers. *)
module Mode : sig
  type t =
    | Single_line
    | Multiline
  [@@deriving equal, sexp_of]
end

module Password_display : sig
  type t =
    | Hidden
    | Revealed
  [@@deriving equal, sexp_of]
end

module Privacy : sig
  (** Password mode is single-line only. Both display states suppress the native
      accessibility value; Hidden also suppresses native Copy/Cut. Revealed allows
      ordinary copying. This controls presentation, not encrypted storage: native
      snapshots, submissions and their sexps still contain the application value. *)
  type t =
    | Plain
    | Password of Password_display.t
  [@@deriving equal, sexp_of]
end

module Content_hint = Input_content_hint
module Search = Editor_search

module Config : sig
  type t [@@deriving equal, sexp_of]

  val create
    :  ?privacy:Privacy.t
    -> ?content_hint:Content_hint.t
    -> ?format:Input_format.t
    -> ?edit_filter:Input_validation.t
    -> ?layout:Text_area_layout.t
    -> ?clear_on_escape:bool
    -> ?searchable:bool
    -> ?placeholder:string
    -> ?read_only:bool
    -> ?disabled:bool
    -> ?submit_on_enter:bool
    -> ?auto_focus:bool
    -> ?min_rows:int
    -> ?max_rows:int
    -> mode:Mode.t
    -> label:string
    -> unit
    -> t Or_error.t

  val mode : t -> Mode.t
  val privacy : t -> Privacy.t
  val content_hint : t -> Content_hint.t option

  (** Ordinary single-line inputs only. Policy changes retain an incompatible
      native draft rather than replacing it. Explicit replacement commands must
      supply formatted text; use [Input_format.format_raw] before constructing a
      command from raw slot/numeric values. Selection offsets refer to that
      formatted text. *)
  val format : t -> Input_format.t option

  (** Native single-line edit filtering. This does not certify submitted values;
      see [Input_validation]. Changing the filter preserves native draft/history. *)
  val edit_filter : t -> Input_validation.t option

  (** Ordinary multiline inputs only. Removing the option restores native
      defaults. Layout changes retain text, selection, composition and history. *)
  val layout : t -> Text_area_layout.t option

  (** Defaults to false. On a focused editable nonempty field, Escape attempts
      one undoable clear, subject to the current format/filter, and consumes the
      action. Composition takes precedence: the first Escape only ends the mark.
      Empty, read-only and disabled fields retain ordinary Escape propagation.
      Both input modes are supported; picker query slots own Escape separately. *)
  val clear_on_escape : t -> bool

  (** Opt-in ordinary multiline search, default false. Disabling closes the
      native session and preserves the query, draft, selection and history. *)
  val searchable : t -> bool
end

module Revision : sig
  type t [@@deriving compare, equal, sexp_of]

  val of_int64 : int64 -> t Or_error.t
  val to_int64 : t -> int64
end

module Selection : sig
  (** UTF-8 byte offsets. Anchor/head preserve direction. A collapsed range is
      a caret. Text-dependent character boundaries are checked when applied. *)
  type t [@@deriving equal, sexp_of]

  val create : anchor:int -> head:int -> t Or_error.t
  val anchor : t -> int
  val head : t -> int
  val validate : t -> text:string -> unit Or_error.t
end

module Selection_policy : sig
  type t =
    | Start
    | End
    | Preserve
    | Select of Selection.t
  [@@deriving equal, sexp_of]
end

module Undo_policy : sig
  type t =
    | Record
    | Reset
  [@@deriving equal, sexp_of]
end

module Snapshot : sig
  type t [@@deriving equal, sexp_of]

  val text : t -> string
  val revision : t -> Revision.t
  val selection : t -> Selection.t
  val composition : t -> Selection.t option
  val focused : t -> bool
end

module Submission : sig
  (** The exact native text/revision at a submit event, outside composition. *)
  type t [@@deriving equal, sexp_of]

  val text : t -> string
  val revision : t -> Revision.t
end

module Command : sig
  (** [Submit] captures the current native snapshot outside composition, without
      editing or emitting a second native submit event. Prefer the Eio
      controller's [submit] to invoke its configured application handler. *)
  type t =
    | Replace of
        { text : string
        ; selection : Selection_policy.t
        ; undo : Undo_policy.t
        ; if_revision : Revision.t option
        }
    | Select of Selection.t
    | Focus
    | Undo
    | Redo
    | Submit
    | Read_snapshot
  [@@deriving equal, sexp_of]
end

module Command_error : sig
  type t =
    | Not_mounted
    | Closed
    | Stale_editor
    | Stale_revision
    | Composing
    | Invalid_selection
    | Limit_exceeded
    | Busy
    | Native_failure
    | Invalid_text
    | Focus_blocked
    | Search_unavailable
    | Stale_search
    | Not_editable
  [@@deriving equal, sexp_of]
end

module Event : sig
  type t =
    | Changed of Snapshot.t
    | Submitted of Submission.t
    | Search_changed of Search.Snapshot.t
    (** Bounded metadata from an opted-in multiline editor, including native
        Find actions. Independent of text changes; may coalesce before delivery.
        Disable sends closed state. This event never changes the text draft. *)
  [@@deriving equal, sexp_of]
end

val max_text_bytes : int
val history_budget_bytes : int
val validate_text : mode:Mode.t -> string -> unit Or_error.t

module Expert : sig
  type config =
    { mode : Mode.t
    ; privacy : Privacy.t
    ; content_hint : Content_hint.t option
    ; format : Input_format.t option
    ; edit_filter : Input_validation.t option
    ; layout : Text_area_layout.t option
    ; clear_on_escape : bool
    ; searchable : bool
    ; label : string
    ; placeholder : string
    ; read_only : bool
    ; disabled : bool
    ; submit_on_enter : bool
    ; auto_focus : bool
    ; min_rows : int
    ; max_rows : int
    }

  val config : Config.t -> config

  val snapshot
    :  window:Gpuio_protocol.Window_id.t
    -> node:Gpuio_protocol.Node_id.t
    -> revision:Revision.t
    -> text:string
    -> selection:Selection.t
    -> composition:Selection.t option
    -> focused:bool
    -> Snapshot.t Or_error.t

  val submission : Snapshot.t -> Submission.t Or_error.t
  val submission_snapshot : Submission.t -> Snapshot.t
  val window : Snapshot.t -> Gpuio_protocol.Window_id.t
  val node : Snapshot.t -> Gpuio_protocol.Node_id.t
  val config_to_wire : Config.t -> Gpuio_protocol.Wire.Editor.Config.t
  val privacy_to_wire : Privacy.t -> Gpuio_protocol.Wire.Editor.Privacy.t

  val snapshot_of_wire
    :  window:Gpuio_protocol.Window_id.t
    -> node:Gpuio_protocol.Node_id.t
    -> Gpuio_protocol.Wire.Editor.Snapshot.t
    -> Snapshot.t Or_error.t

  val command_to_wire : Command.t -> Gpuio_protocol.Wire.Editor.Command.t
  val error_of_wire : Gpuio_protocol.Wire.Editor.Error.t -> Command_error.t
end
