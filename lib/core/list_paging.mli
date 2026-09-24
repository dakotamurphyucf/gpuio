open Core

module Direction : sig
  type t =
    | Before
    | After
  [@@deriving equal, sexp_of]
end

module Boundary : sig
  (** [More None] requests the initial page at an open boundary. Cursors are
      opaque application strings; the list does not interpret or serialize them. *)
  type t =
    | End
    | More of string option
  [@@deriving equal, sexp_of]
end

module Request : sig
  type t

  val direction : t -> Direction.t
  val cursor : t -> string option
  val generation : t -> int64
end

module Status : sig
  type t =
    | Ready
    | Loading
    | End
    | Failed of Error.t
  [@@deriving sexp_of]
end

module Snapshot : sig
  type ('key, 'data, 'cmp) t =
    { items : ('key, 'data, 'cmp) List_collection.t
    ; before : Status.t
    ; after : Status.t
    ; generation : int64
    }
end

module Completion : sig
  type t =
    | Applied
    | Obsolete
  [@@deriving equal, sexp_of]
end

(** UI-domain-owned paging state. At most one request per boundary is active;
    the two directions may load concurrently. Application data is independent
    of visible row computations and survives viewport changes.

    This module performs no I/O. An Eio adapter owns producer cancellation;
    token validation here also rejects already queued results after cancellation,
    retry, collection reset, or delivery to a different controller. *)
type ('key, 'data, 'cmp) t

val create
  :  ('key, 'data, 'cmp) List_collection.t
  -> before:Boundary.t
  -> after:Boundary.t
  -> ('key, 'data, 'cmp) t

val items : ('key, 'data, 'cmp) t -> ('key, 'data, 'cmp) List_collection.t
val snapshot : ('key, 'data, 'cmp) t -> ('key, 'data, 'cmp) Snapshot.t
val generation : (_, _, _) t -> int64
val status : (_, _, _) t -> Direction.t -> Status.t

(** Returns [None] for an end, an in-flight request, or a failed boundary.
    Failure requires explicit [retry]; scrolling never starts a retry loop. *)
val request : (_, _, _) t -> Direction.t -> Request.t option Or_error.t

val retry : (_, _, _) t -> Direction.t -> Request.t option Or_error.t

(** Rows arrive in display order in both directions. Keys must be unique across
    the entire loaded collection. [next] replaces only the requested boundary.
    An empty page must reach [End] or advance its cursor to prevent busy loops.
    An invalid current response leaves data unchanged and marks the edge Failed.
    An obsolete response is ignored without inspecting its contents. *)
val complete
  :  ('key, 'data, _) t
  -> Request.t
  -> rows:('key * 'data) list
  -> next:Boundary.t
  -> Completion.t Or_error.t

val fail : (_, _, _) t -> Request.t -> Error.t -> Completion.t

(** Logical cancellation immediately makes subsequent delivery obsolete and
    restores the boundary's cursor. Repeated/foreign cancellation does nothing. *)
val cancel : (_, _, _) t -> Request.t -> Completion.t

(** Replacing a conversation invalidates both boundaries' requests. The caller
    must also cancel producer fibers; this operation does not perform I/O. *)
val reset
  :  ('key, 'data, 'cmp) t
  -> ('key, 'data, 'cmp) List_collection.t
  -> before:Boundary.t
  -> after:Boundary.t
  -> unit Or_error.t

(** Streaming updates preserve collection order and do not cancel page loads. *)
val set : ('key, 'data, _) t -> key:'key -> data:'data -> unit Or_error.t
