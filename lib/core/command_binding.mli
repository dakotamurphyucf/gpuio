(** Native binding-inspection data. The mounted observer is not wired yet; these
    constructors do not query a window or register shortcuts. *)
module Context : sig
  type t [@@deriving equal, sexp_of]

  val focused : t
  val here : t

  (** Preserve only the exact window/node lease, not the editor text or selection.
      A removed editor produces Context_gone; it never targets a remount. *)
  val editor : Text_input.Snapshot.t -> t

  (** Native GPUI context facts, for example ["Input mode=visible"], not a
      keybinding predicate. Nonblank UTF-8 without NUL, <=1024 bytes. Bounded
      native parsing reports Invalid_context for malformed facts. This context
      supports native-action targets only. *)
  val native_context : string -> t Core.Or_error.t
end

module Target : sig
  type t =
    | Command of Command.Id.t
    | Native_action of Command.Native.t
  [@@deriving equal, sexp_of]
end

module Config : sig
  type t [@@deriving equal, sexp_of]

  (** 1..64 unique ordered targets; default context Focused. Here accepts only
      registry commands, Native_context only native actions. Focused and Editor
      permit both. Non-focused contexts describe hypothetical bindings and do not
      apply gates from an unrelated focused editor. *)
  val create : ?context:Context.t -> Target.t list -> t Core.Or_error.t

  val context : t -> Context.t
  val targets : t -> Target.t list
end

module Suppression : sig
  type t =
    | Disabled
    | Scope_blocked
    | Native_unavailable
    | Composition
    | Text_input
    | Native_navigation
    | Conflict of Command.Id.t
  [@@deriving equal, sexp_of]
end

module Disposition : sig
  type t =
    | Declared
    | Override
    | Native_first
    | Widget
    | Unavailable of Suppression.t
  [@@deriving equal, sexp_of]
end

module Stroke : sig
  module Modifier : sig
    type t =
      | Control
      | Alt
      | Shift
      | Super
      | Function
    [@@deriving equal, sexp_of]
  end

  (** Native display data, including keys/modifiers outside Shortcut's input
      domain. This type cannot be registered as a single-chord shortcut. *)
  type t [@@deriving equal, sexp_of]

  val key : t -> string
  val modifiers : t -> Modifier.t list
  val format : t -> platform:Shortcut.Platform.t -> string
  val accessible_label : t -> platform:Shortcut.Platform.t -> string
end

module Candidate : sig
  type t = private
    { shortcut : Shortcut.t
    ; disposition : Disposition.t
    }
  [@@deriving equal, sexp_of]
end

module Unsupported : sig
  type t =
    | Sequence_too_long
    | Invalid_stroke
  [@@deriving equal, sexp_of]
end

module Entry : sig
  type t = private
    | Missing_command
    | Registry of
        { enabled : bool
        ; candidates : Candidate.t list
        }
    | Native_unbound
    | Native_binding of
        { strokes : Stroke.t list
        ; disposition : Disposition.t
        }
    | Native_unsupported of Unsupported.t
  [@@deriving equal, sexp_of]
end

(** [Capacity] means sampling exceeded the native work budget; it must not be
    interpreted as a missing command or an unbound action. [Suspended] is a
    retained, currently inactive observer. [Epoch_exhausted] is terminal until
    the observer configuration is replaced or remounted. *)
module State : sig
  type t = private
    | Ready of (Target.t * Entry.t) list
    | Suspended
    | Context_gone
    | Invalid_context
    | Epoch_exhausted
    | Capacity
  [@@deriving equal, sexp_of]
end

module Observation : sig
  type t [@@deriving equal, sexp_of]

  (** Positive, monotonically increasing within one mounted configuration.
      A snapshot is not a guarantee about future focus or key delivery. *)
  val epoch : t -> int64

  val state : t -> State.t
end

module Expert : sig
  val to_wire : Config.t -> Gpuio_protocol.Command_binding_wire.Config.t
  val valid_window : Config.t -> Gpuio_protocol.Window_id.t -> bool

  val of_wire
    :  Config.t
    -> Gpuio_protocol.Command_binding_wire.Observation.t
    -> Observation.t option
end
