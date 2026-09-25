open Core

(** Value contracts for native-owned inputs. A snapshot is an observation, never
    an implicit replacement command. The Eio adapter supplies Bonsai controllers. *)
module Mode : sig
  type t =
    | Single_line
    | Multiline
  [@@deriving equal, sexp_of]
end

module Config : sig
  type t [@@deriving equal, sexp_of]

  val create
    :  ?placeholder:string
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
  [@@deriving equal, sexp_of]
end

module Event : sig
  type t =
    | Changed of Snapshot.t
    | Submitted of Submission.t
  [@@deriving equal, sexp_of]
end

val max_text_bytes : int
val history_budget_bytes : int
val validate_text : mode:Mode.t -> string -> unit Or_error.t

module Expert : sig
  type config =
    { mode : Mode.t
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

  val snapshot_of_wire
    :  window:Gpuio_protocol.Window_id.t
    -> node:Gpuio_protocol.Node_id.t
    -> Gpuio_protocol.Wire.Editor.Snapshot.t
    -> Snapshot.t Or_error.t

  val command_to_wire : Command.t -> Gpuio_protocol.Wire.Editor.Command.t
  val error_of_wire : Gpuio_protocol.Wire.Editor.Error.t -> Command_error.t
end
