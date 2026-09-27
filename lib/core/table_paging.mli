open Core
module Direction = List_paging.Direction
module Boundary = List_paging.Boundary
module Status = List_paging.Status
module Completion = List_paging.Completion

module Request : sig
  (** Captures the query at admission, never a mutable reference to the latest
      filter/sort settings. Opaque ownership also distinguishes independent
      pagers with the same numeric generation. No row data is captured. *)
  type 'query t

  val query : 'query t -> 'query
  val direction : _ t -> Direction.t
  val cursor : _ t -> string option
  val generation : _ t -> int64
end

module Snapshot : sig
  type ('query, 'data) t =
    { query : 'query
    ; data : 'data Table_data.t
    ; generation : int64
    ; before : Status.t
    ; after : Status.t
    }
end

(** UI-domain-owned directional paging. One request per boundary, two in total.
    Data is application-owned and survives viewport eviction. Sorting/filtering
    belongs to [query] and the application/server; this module never reorders
    the loaded subset. All rows and cursors admitted by [reset] must belong to
    the new query. Prefer immutable query values; mutating a query captured by
    a request violates its snapshot contract.

    No I/O or fiber cancellation occurs here. The Eio adapter must cancel
    producers on reset/cancel/close; opaque request validation additionally
    rejects completions already queued before cancellation. Mutate on the UI
    domain, outside Incremental graph evaluation. *)
type ('query, 'data) t

val max_page_rows : int
val max_cursor_bytes : int
val max_error_bytes : int

(** Cursors are opaque bytes, at most 4096 bytes; seed data uses Table_data's
    logical limits and may exceed the per-page limit. *)
val create
  :  query:'query
  -> 'data Table_data.t
  -> before:Boundary.t
  -> after:Boundary.t
  -> ('query, 'data) t Or_error.t

val snapshot : ('query, 'data) t -> ('query, 'data) Snapshot.t
val data : (_, 'data) t -> 'data Table_data.t
val query : ('query, _) t -> 'query
val generation : (_, _) t -> int64
val status : (_, _) t -> Direction.t -> Status.t

(** Ready requests are admitted once; end/loading/failed boundaries return None.
    Failed boundaries require [retry], never an automatic render/scroll loop. *)
val request : ('query, _) t -> Direction.t -> 'query Request.t option Or_error.t

val retry : ('query, _) t -> Direction.t -> 'query Request.t option Or_error.t

(** At most 2048 rows per response, in display order. Empty pages must advance
    their cursor or reach End. Duplicates, oversized pages/cursors or source
    limits reject atomically and mark this current boundary Failed. Obsolete
    responses are ignored before inspecting any row/cursor payload. Stored
    failure text is capped to 4096 UTF-8 bytes without NUL, even when the returned
    error is larger. Invalid UTF-8 diagnostics use a fixed fallback message. *)
val complete
  :  ('query, 'data) t
  -> 'query Request.t
  -> rows:(Table_data.Id.t * 'data) list
  -> next:Boundary.t
  -> Completion.t Or_error.t

val fail : ('query, _) t -> 'query Request.t -> Error.t -> Completion.t
val cancel : ('query, _) t -> 'query Request.t -> Completion.t

(** Advance query generation and invalidate both requests atomically, including
    when the same query value is supplied again. Invalid boundaries leave the
    old query, rows and pending requests unchanged. Supplying a reordered source
    from the same Table_data lineage preserves surviving row references/anchors;
    a fresh source deliberately resets identity. No assumption that an absent
    row will return in later pages is made here; the widget owns selection repair. *)
val reset
  :  ('query, 'data) t
  -> query:'query
  -> 'data Table_data.t
  -> before:Boundary.t
  -> after:Boundary.t
  -> unit Or_error.t

(** Current-query payload updates preserve order and pending requests. *)
val set : (_, 'data) t -> key:Table_data.Id.t -> data:'data -> unit Or_error.t

(** Append up to [max_page_rows] new records at a known latest boundary
    ([After = End]), without canceling older-history requests. *)
val append : (_, 'data) t -> (Table_data.Id.t * 'data) list -> unit Or_error.t

module Expert : sig
  (** Producer adapters use this before entering a queued load and when deciding
      which cancellation context to retire. Does not compare query payloads. *)
  val is_current : ('query, _) t -> 'query Request.t -> bool
end
