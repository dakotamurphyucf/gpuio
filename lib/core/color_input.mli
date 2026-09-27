open Core

(** Color-control descriptions and native observations. These values do not own
    GPUI resources; mounted controls and correlated commands belong to the runtime. *)
module Channel : sig
  type t =
    | Hue
    | Saturation
    | Lightness
    | Alpha
  [@@deriving equal, sexp_of]
end

module Field : sig
  type t =
    | Hex
    | Channel of Channel.t
  [@@deriving equal, sexp_of]
end

module Palette_entry : sig
  type t [@@deriving equal, sexp_of]

  val create : color:Color_value.Rgba.t -> label:string -> t Or_error.t
  val color : t -> Color_value.Rgba.t
  val label : t -> string
end

module Labels : sig
  type t [@@deriving equal, sexp_of]

  (** Nonblank UTF-8 strings, each at most 4096 bytes, without NUL. *)
  val create
    :  control:string
    -> hue:string
    -> saturation:string
    -> lightness:string
    -> alpha:string
    -> hex:string
    -> clear:string
    -> t Or_error.t

  val english : control:string -> t Or_error.t
end

module Config : sig
  type t [@@deriving equal, sexp_of]

  (** At most 256 palette entries, each with a nonblank label of at most 256
      UTF-8 bytes. Incompatible palette colors remain visible but not selectable.
      Policy changes retain historical values and expose their allowed flags. *)
  val create
    :  labels:Labels.t
    -> ?palette:Palette_entry.t list
    -> ?alpha_policy:Color_value.Alpha_policy.t
    -> ?allow_empty:bool
    -> ?disabled:bool
    -> ?read_only:bool
    -> unit
    -> t Or_error.t

  val allows : t -> Color_value.Value.t -> bool
  val is_disabled : t -> bool
  val is_read_only : t -> bool
end

module Revision : sig
  type t [@@deriving equal, compare, sexp_of]

  val of_int64 : int64 -> t Or_error.t
  val to_int64 : t -> int64
end

module Interaction_id : sig
  type t [@@deriving equal, compare, sexp_of]

  val to_int64 : t -> int64
end

module Interaction : sig
  module Kind : sig
    type t =
      | Drag of Channel.t
      | Text of Field.t
    [@@deriving equal, sexp_of]
  end

  type t [@@deriving equal, sexp_of]

  val id : t -> Interaction_id.t
  val kind : t -> Kind.t
end

module Draft : sig
  module Status : sig
    type t =
      | Empty
      | Incomplete
      | Invalid
      | Out_of_range
      | Forbidden
      | Valid
    [@@deriving equal, sexp_of]
  end

  type t [@@deriving equal, sexp_of]

  val text : t -> string
  val is_composing : t -> bool
  val status : t -> Status.t
end

module Snapshot : sig
  type t [@@deriving equal, sexp_of]

  val revision : t -> Revision.t
  val value : t -> Color_value.Value.t
  val committed : t -> Color_value.Value.t

  (** Unquantized HSLA editing values, including retained achromatic hue. *)
  val channels : t -> Color_value.Hsla.t

  val interaction : t -> Interaction.t option
  val draft : t -> Draft.t option
  val value_allowed : t -> bool
  val committed_allowed : t -> bool
end

module Source : sig
  type t =
    | Pointer
    | Keyboard
    | Accessibility
    | Text
    | Palette
    | Clear
  [@@deriving equal, sexp_of]
end

module Cancel_reason : sig
  type t =
    | Escape
    | Configuration_changed
    | Disabled
    | Read_only
    | Hidden
    | Modal
    | Window_inactive
    | Unmounted
    | Programmatic
    | Interrupted
  [@@deriving equal, sexp_of]
end

module Event : sig
  (** A preview may describe invalid/composing text while preserving the last valid
      color. Observations never write back into native child editors. *)
  type t =
    | Observed of Snapshot.t
    | Started of Snapshot.t
    | Preview of Snapshot.t
    | Committed of Source.t * Snapshot.t
    | Cancelled of Cancel_reason.t * Snapshot.t
  [@@deriving equal, sexp_of]

  val snapshot : t -> Snapshot.t
end

module Command : sig
  (** Set and Reset validate before cancelling an active edit. Reset restores the
      original mounted seed and can fail current alpha/empty policy. Programmatic
      updates remain allowed while disabled/read-only, and emit Observed.
      Cancel restores committed value and hue. Focus targets a native text field. *)
  type t =
    | Set of
        { value : Color_value.Value.t
        ; if_revision : Revision.t option
        }
    | Reset of { if_revision : Revision.t option }
    | Cancel
    | Focus of Field.t
    | Read_snapshot
  [@@deriving equal, sexp_of]
end

module Command_error : sig
  type t =
    | Not_mounted
    | Closed
    | Stale_color_input
    | Stale_revision
    | Stale_interaction
    | Disabled
    | Read_only
    | Focus_blocked
    | Busy
    | Invalid_value
    | Invalid_config
    | Invalid_draft
    | Composing
    | Limit_exceeded
    | Native_failure
  [@@deriving equal, sexp_of]
end

module Expert : sig
  val config_to_wire : Config.t -> Gpuio_protocol.Color_input_wire.Config.t
  val config_of_wire : Gpuio_protocol.Color_input_wire.Config.t -> Config.t Or_error.t
  val value_to_wire : Color_value.Value.t -> Gpuio_protocol.Color_input_wire.Value.t

  val snapshot_of_wire
    :  window:Gpuio_protocol.Window_id.t
    -> node:Gpuio_protocol.Node_id.t
    -> Gpuio_protocol.Color_input_wire.Snapshot.t
    -> Snapshot.t Or_error.t

  val event_of_wire
    :  window:Gpuio_protocol.Window_id.t
    -> node:Gpuio_protocol.Node_id.t
    -> Gpuio_protocol.Color_input_wire.Event.t
    -> Event.t Or_error.t

  val window : Snapshot.t -> Gpuio_protocol.Window_id.t
  val node : Snapshot.t -> Gpuio_protocol.Node_id.t
  val command_to_wire : Command.t -> Gpuio_protocol.Color_input_wire.Command.t
  val error_of_wire : Gpuio_protocol.Color_input_wire.Error.t -> Command_error.t
end
