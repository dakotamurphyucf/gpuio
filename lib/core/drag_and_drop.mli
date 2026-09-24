open Core

(** Immutable drag data. Constructing these values performs no filesystem I/O.
    Payloads are copied into a native gesture snapshot when dragging starts. *)
module Custom_kind : sig
  type t [@@deriving equal, compare, sexp_of]

  (** Case-sensitive application identifier, 1..128 ASCII bytes from letters,
      digits, [.], [_], [-], [/], [+]. No MIME negotiation or deserialization is
      implied. Prefer a namespaced name such as [com.example.task/v1]. *)
  val of_string : string -> t Or_error.t

  val to_string : t -> string
end

module Format : sig
  type t =
    | Text
    | Files
    | Custom of Custom_kind.t
  [@@deriving equal, compare, sexp_of]
end

module File : sig
  type t = private
    { path : File_path.t
    ; is_directory : bool option
    }
  [@@deriving equal, sexp_of]

  (** Directory metadata is caller-supplied, never obtained synchronously while
      dragging. Native incoming paths can have unknown metadata. A path is not
      an Eio capability and does not grant filesystem access. *)
  val create : ?is_directory:bool -> File_path.t -> t
end

module Payload : sig
  type t = private
    | Text of string
    | Files of File.t list
    | Custom of
        { kind : Custom_kind.t
        ; data : string
        }
  [@@deriving equal, sexp_of]

  val max_bytes : int
  val max_files : int

  (** UTF-8 without NUL, at most [max_bytes] bytes. Empty text is allowed. *)
  val text : string -> t Or_error.t

  (** Nonempty, at most [max_files], with at most [max_bytes] total path bytes.
      Preserve ordering, duplicates and exact native path bytes. *)
  val files : File.t list -> t Or_error.t

  (** Opaque bytes, including NUL and invalid UTF-8, at most [max_bytes]. No
      automatic deserialization or cross-application export. *)
  val custom : kind:Custom_kind.t -> data:string -> t Or_error.t

  val format : t -> Format.t

  (** Text/custom data bytes or the sum of file path bytes; excludes metadata
      and encoding overhead. *)
  val data_bytes : t -> int
end

module Source : sig
  type t [@@deriving equal, sexp_of]

  (** Label is nonblank UTF-8 without NUL, at most 4096 bytes; it names the
      source and supplies the default drag preview. Desktop offering is opt-in,
      files-only, and requires known directory metadata for every entry.
      Requesting it never guarantees that the OS accepts or completes a drag;
      it never authorizes moving or deleting files. *)
  val create
    :  label:string
    -> payload:Payload.t
    -> ?disabled:bool
    -> ?allow_desktop_files:bool
    -> unit
    -> t Or_error.t

  val label : t -> string
  val payload : t -> Payload.t
  val disabled : t -> bool
  val allow_desktop_files : t -> bool
end

module Target : sig
  type t [@@deriving equal, sexp_of]

  (** A nonempty allowlist of at most 16 distinct formats. Acceptance is decided
      synchronously in Rust from the current configuration; an OCaml event
      callback cannot retroactively reject a drop. Label follows Source rules.
      Disabled targets reject all payloads. *)
  val create
    :  label:string
    -> accept:Format.t list
    -> ?disabled:bool
    -> unit
    -> t Or_error.t

  val label : t -> string
  val accepted_formats : t -> Format.t list
  val disabled : t -> bool
  val accepts : t -> Payload.t -> bool
end

(** Gesture IDs are unique within one native application session. Samples use
    logical pixels. Consecutive target Moved samples may coalesce; lifecycle edges
    do not. Removed/replaced handlers receive no later callbacks. An external
    offer is not an OS copy/move acknowledgement; Unconfirmed promises no outcome. *)
module Gesture_id : sig
  type t [@@deriving equal, compare, sexp_of]
end

module Origin : sig
  type t =
    | Internal
    | Desktop
  [@@deriving equal, sexp_of]
end

module Cancel_reason : sig
  type t =
    | Escape
    | Hidden
    | Blocked
    | Disabled
    | Removed
    | Reconfigured
    | Window_closed
    | Window_inactive
  [@@deriving equal, sexp_of]
end

module Outcome : sig
  type t =
    | Internal_drop
    | Cancelled of Cancel_reason.t
    | Unconfirmed
  [@@deriving equal, sexp_of]
end

module Rejection : sig
  type t =
    | Invalid_data
    | Limit_exceeded
  [@@deriving equal, sexp_of]
end

module Modifiers : sig
  type t =
    { shift : bool
    ; control : bool
    ; alt : bool
    ; command : bool
    ; function_ : bool
    }
  [@@deriving equal, sexp_of]
end

module Offer : sig
  type t = private
    { format : Format.t
    ; data_bytes : int
    ; file_count : int
    ; origin : Origin.t
    }
  [@@deriving equal, sexp_of]
end

module Source_phase : sig
  type t =
    | Started of Payload.t
    | Desktop_offered
    | Desktop_unavailable
    | Ended of Outcome.t
  [@@deriving equal, sexp_of]
end

module Source_event : sig
  type t = private
    { gesture : Gesture_id.t
    ; phase : Source_phase.t
    }
  [@@deriving equal, sexp_of]
end

module Target_phase : sig
  type t =
    | Entered of Offer.t
    | Moved
    | Left
    | Dropped of Payload.t
    | Rejected of Rejection.t
  [@@deriving equal, sexp_of]
end

module Target_event : sig
  type t = private
    { gesture : Gesture_id.t
    ; phase : Target_phase.t
    ; window_position : Pointer.Position.t
    ; local_position : Pointer.Position.t
    ; modifiers : Modifiers.t
    }
  [@@deriving equal, sexp_of]
end

module Expert : sig
  val payload_to_wire : Payload.t -> Gpuio_protocol.Wire.Drag_and_drop.Payload.t

  val payload_of_wire
    :  Gpuio_protocol.Wire.Drag_and_drop.Payload.t
    -> Payload.t Or_error.t

  val source_to_wire : Source.t -> Gpuio_protocol.Wire.Drag_and_drop.Source.t
  val target_to_wire : Target.t -> Gpuio_protocol.Wire.Drag_and_drop.Target.t

  val source_event_of_wire
    :  Gpuio_protocol.Wire.Drag_and_drop.Source_sample.t
    -> Source_event.t Or_error.t

  val target_event_of_wire
    :  Gpuio_protocol.Wire.Drag_and_drop.Target_sample.t
    -> Target_event.t Or_error.t
end
