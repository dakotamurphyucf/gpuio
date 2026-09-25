open Core

module Axis : sig
  type t =
    | Horizontal
    | Vertical
  [@@deriving equal, sexp_of]
end

module Scale : sig
  type t =
    | Linear
    | Logarithmic
  [@@deriving equal, sexp_of]
end

module Thumb : sig
  type t =
    | Single
    | Lower
    | Upper
  [@@deriving equal, sexp_of]
end

module Value : sig
  type t = private
    | Single of float
    | Range of
        { lower : float
        ; upper : float
        }
  [@@deriving equal, sexp_of]

  val single : float -> t Or_error.t
  val range : lower:float -> upper:float -> t Or_error.t
end

module Config : sig
  type t [@@deriving equal, sexp_of]

  (** Logarithmic mapping requires a positive minimum. Range thumb labels default
      to the main label followed by " lower" and " upper"; supply localized names
      explicitly. Values and active drag state belong to the native owner. *)
  val create
    :  domain:Numeric.Domain.t
    -> label:string
    -> ?lower_label:string
    -> ?upper_label:string
    -> ?axis:Axis.t
    -> ?scale:Scale.t
    -> ?disabled:bool
    -> ?read_only:bool
    -> unit
    -> t Or_error.t

  val domain : t -> Numeric.Domain.t
  val is_disabled : t -> bool
  val is_read_only : t -> bool
end

module Revision : sig
  type t [@@deriving compare, equal, sexp_of]

  val of_int64 : int64 -> t Or_error.t
  val to_int64 : t -> int64
end

module Snapshot : sig
  type t [@@deriving equal, sexp_of]

  val revision : t -> Revision.t

  (** Current native preview; equal to [committed] outside a drag. *)
  val value : t -> Value.t

  val committed : t -> Value.t
  val dragging : t -> Thumb.t option
end

module Source : sig
  type t =
    | Pointer
    | Keyboard
    | Accessibility
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
  (** Preview may be coalesced within its gesture. Start/commit/cancel preserve
      order and are never replaced by a newer preview. Observations are not writes. *)
  type t =
    | Observed of Snapshot.t
    | Drag_started of Snapshot.t
    | Preview of Snapshot.t
    | Committed of Source.t * Snapshot.t
    | Cancelled of Cancel_reason.t * Snapshot.t
  [@@deriving equal, sexp_of]
end

module Command : sig
  (** Replace normalizes values and cancels any drag. It cannot change single/
      range mode for a mounted owner; remount with a new key to change mode.
      Programmatic replacement is allowed while disabled/read-only. *)
  type t =
    | Replace of
        { value : Value.t
        ; if_revision : Revision.t option
        }
    | Cancel_drag
    | Focus of Thumb.t
    | Read_snapshot
  [@@deriving equal, sexp_of]
end

module Command_error : sig
  type t =
    | Not_mounted
    | Closed
    | Stale_slider
    | Stale_revision
    | Wrong_mode
    | Wrong_thumb
    | Disabled
    | Focus_blocked
    | Busy
    | Invalid_value
    | Invalid_config
    | Limit_exceeded
    | Read_only
  [@@deriving equal, sexp_of]
end

module Expert : sig
  val config_to_wire : Config.t -> Gpuio_protocol.Slider_wire.Config.t
  val value_to_wire : Value.t -> Gpuio_protocol.Slider_wire.Value.t
  val snapshot_of_wire : Gpuio_protocol.Slider_wire.Snapshot.t -> Snapshot.t Or_error.t
  val event_of_wire : Gpuio_protocol.Slider_wire.Event.t -> Event.t Or_error.t
  val command_to_wire : Command.t -> Gpuio_protocol.Slider_wire.Command.t
  val error_of_wire : Gpuio_protocol.Slider_wire.Error.t -> Command_error.t
end
