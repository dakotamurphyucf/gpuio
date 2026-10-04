open Core

(** Bounded, literal native text-area search. These value types describe search
    observations; they do not implement a search bar or execute commands. *)
module Query : sig
  type t [@@deriving equal, sexp_of]

  (** At most 2,048 UTF-8 bytes without NUL. Empty means no matches. *)
  val create : string -> t Or_error.t

  val empty : t
  val to_string : t -> string
end

module Case : sig
  type t =
    | Sensitive
    | Ascii_insensitive
  [@@deriving equal, sexp_of]
end

module Mode : sig
  type t =
    | Closed
    | Find
    | Replace
  [@@deriving equal, sexp_of]
end

module Stamp : sig
  (** Exact window/node lease plus native editor and search revisions. A query
      or navigation round trip invalidates an older stamp. *)
  type t [@@deriving equal, sexp_of]
end

module Occurrence : sig
  type t [@@deriving equal, sexp_of]

  (** Zero-based occurrence index. Byte offsets are UTF-8, end exclusive. *)
  val index : t -> int

  val byte_start : t -> int
  val byte_end : t -> int
end

module Snapshot : sig
  type t [@@deriving equal, sexp_of]

  val stamp : t -> Stamp.t
  val mode : t -> Mode.t
  val query : t -> Query.t
  val case : t -> Case.t
  val match_count : t -> int
  val current : t -> Occurrence.t option

  (** Search is open and the native editor currently allows replacement outside
      composition. A subsequent command must recheck this; it is an observation. *)
  val can_replace : t -> bool

  (** Changes on each accepted open/reopen. Presentation can use this to focus
      and select the query once, without resetting focus during ordinary renders. *)
  val activation_revision : t -> int64
end

module Command : sig
  type t =
    | Read
    | Close_and_focus of Snapshot.t
    (** Close this particular opening and restore focus atomically. Checks the
        observed owner and activation, so an older bar cannot close a reopened
        search. Text/query/navigation changes within that opening are allowed.
        Disabled or hidden targets close without restoring focus. Composed or
        already closed targets fail without focus changes. Ordinary [Close]
        remains metadata-only. *)
    | Open of { replace : bool }
    | Close
    | Set_query of
        { query : Query.t
        ; case : Case.t
        }
    | Set_query_text of Query.t
    (** Change query text while preserving the current native case policy. *)
    | Set_case of Case.t
    (** Change case policy while preserving the current native query text. *)
    | Toggle_case
    (** Toggle the current native policy; rapid activations do not reuse a stale
        rendered checkbox value. *)
    | Next
    | Previous
    | Replace_current of
        { if_stamp : Stamp.t
        ; replacement : string
        }
    | Replace_all of
        { if_stamp : Stamp.t
        ; replacement : string
        }
  [@@deriving equal, sexp_of]
end

module Response : sig
  type t =
    | Observed of Snapshot.t
    | Replaced of
        { snapshot : Snapshot.t
        ; count : int
        }
  [@@deriving equal, sexp_of]
end

module Expert : sig
  (** Validates bounded metadata. The native matcher guarantees UTF-8 character
      boundaries; without the full text this conversion can check byte bounds,
      query length and match consistency, not the underlying characters. *)
  val snapshot_of_wire
    :  window:Gpuio_protocol.Window_id.t
    -> node:Gpuio_protocol.Node_id.t
    -> Gpuio_protocol.Editor_search_wire.Snapshot.t
    -> Snapshot.t Or_error.t

  val stamp_to_wire : Stamp.t -> Gpuio_protocol.Editor_search_wire.Stamp.t
  val stamp_window : Stamp.t -> Gpuio_protocol.Window_id.t
  val stamp_node : Stamp.t -> Gpuio_protocol.Node_id.t
  val command_to_wire : Command.t -> Gpuio_protocol.Editor_search_wire.Command.t
  val expected_stamp : Command.t -> Stamp.t option
end
