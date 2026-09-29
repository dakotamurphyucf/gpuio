open Core

(** Per-file collapse, row previews and source-provenance observations.
    Pass [Config.t] to [Document.Config.create ~diff] and receive queued events
    through [View.document ~on_diff]. No synchronous native callback is exposed. *)
module File_key : sig
  type t =
    | Path of string
    | Unnamed
  [@@deriving equal, compare, sexp_of]
end

module Collapse : sig
  (** Path labels match every section with that name; [Unnamed] matches sections
      without a parsed name. Managed seeds initialize on mount/generation reset
      or ownership transition, not on each configuration update. Controlled
      activation reports intent and awaits an application update. *)
  type t =
    | Managed of { initially_collapsed : File_key.t list }
    | Controlled of File_key.t list
  [@@deriving equal, sexp_of]
end

module Line_limit : sig
  (** Counts expanded context/added/removed/annotation rows, not headers.
      [None] is unlimited; a limit is 0..8192, a step is 1..8192. Collapsed bodies
      do not contribute to the hidden count used by Show more. *)
  type t =
    | Managed of
        { initial : int option
        ; step : int
        }
    | Controlled of int option
  [@@deriving equal, sexp_of]
end

module Config : sig
  type t [@@deriving equal, sexp_of]

  (** Defaults: managed expanded files, unlimited rows, step 200, word emphasis.
      Keys must be unique (at most 8192); named paths are 1..4096 UTF-8 bytes,
      NUL-free labels with no filesystem behavior. Encoded config <=256 KiB.
      Unknown keys are valid for future streamed file headers. *)
  val create
    :  ?collapse:Collapse.t
    -> ?line_limit:Line_limit.t
    -> ?word_diff:bool
    -> unit
    -> t Or_error.t

  val default : t
  val collapse : t -> Collapse.t
  val line_limit : t -> Line_limit.t
  val word_diff : t -> bool
end

module File : sig
  (** [index] is zero-based in the event's installed snapshot, not a durable ID.
      New paths identify renamed/added files; deletions use the old path. *)
  type t = private
    { index : int
    ; key : File_key.t
    ; before_path : string option
    ; after_path : string option
    }
  [@@deriving equal, sexp_of]
end

module Line : sig
  (** [start_byte,end_byte) addresses original UTF-8 payload bytes excluding the
      diff marker and one line ending. Empty payloads are valid. Context carries
      both one-based numbers; additions/removals carry one; annotations neither. *)
  type t = private
    { file : File.t
    ; before : int option
    ; after : int option
    ; start_byte : int
    ; end_byte : int
    ; text : string
    }
  [@@deriving equal, sexp_of]
end

module Observation : sig
  type t = private
    | Toggle_file of
        { file : File.t
        ; collapsed : bool
        ; applied : bool
        }
    | Show_more of
        { visible : int
        ; hidden : int
        ; applied_limit : int option
        }
    | Line of Line.t
  [@@deriving equal, sexp_of]
end

module Event : sig
  (** Source provenance is independent of the view-tree revision. Values refer
      to the installed picture, which can precede a newer same-generation parse.
      Do not apply its offsets/index to newer content without checking/rebasing. *)
  type t = private
    { source_revision : int64
    ; source_generation : int64
    ; observation : Observation.t
    }
  [@@deriving equal, sexp_of]
end

module Expert : sig
  val to_wire : Config.t -> Gpuio_protocol.Document_diff_wire.Config.t
  val of_wire : Gpuio_protocol.Document_diff_wire.Config.t -> Config.t Or_error.t

  (** Checks payload and ownership semantics. The dispatcher must independently
      fence the config epoch, handler and registered source generation. *)
  val event_of_wire
    :  config:Config.t
    -> Gpuio_protocol.Document_diff_wire.Event.t
    -> Event.t Or_error.t
end
