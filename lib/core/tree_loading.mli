open Core
module Boundary = List_paging.Boundary

module Request : sig
  type t

  val parent : t -> Tree.Id.t
  val cursor : t -> string option
  val generation : t -> int64
  val same : t -> t -> bool
end

module Status : sig
  type t =
    | Ready
    | Queued
    | Loading
    | End
    | Failed of Error.t
  [@@deriving sexp_of]
end

module Completion : sig
  type t =
    | Applied
    | Obsolete
  [@@deriving equal, sexp_of]
end

module Page : sig
  (** New immediate children and a closed forest of their new descendants.
      Existing IDs cannot be reused. [next] replaces the parent's boundary. *)
  type 'data t =
    { roots : Tree.Id.t list
    ; nodes : (Tree.Id.t * 'data Tree.Node.t) list
    ; next : Boundary.t
    }
end

module Snapshot : sig
  (** Immutable loaded data and status. Retaining historical snapshots is an
      application lifetime decision, outside the controller's cache bounds. *)
  type 'data t

  val tree : 'data t -> 'data Tree.t
  val generation : _ t -> int64

  (** Both controller identity and reset generation must match. Two separately
      created controllers may expose the same numeric generation. *)
  val same_generation : _ t -> _ t -> bool

  val status : _ t -> Tree.Id.t -> Status.t option
  val queued_count : _ t -> int
  val running_count : _ t -> int
  val failed_count : _ t -> int
  val error_detail_count : _ t -> int

  (** Conservative invalidation of branch status, including error-detail eviction
      and source boundary changes. Visits each candidate ID once using shared
      maps and the bounded request/error sets; payloads are never compared.
      Generation changes require the consumer to reset its projection. *)
  val fold_changed_statuses
    :  'data t
    -> previous:'data t
    -> init:'acc
    -> f:('acc -> Tree.Id.t -> 'acc)
    -> 'acc
end

(** UI-domain-owned model; no I/O, fibers, polling or native callbacks. An Eio
    adapter calls [take] to start producers and cancels producers whose request
    fails [is_current]. Each branch has at most one queued or running request.

    Failure markers are bounded by loaded node count, independent of a bounded
    cache of error messages. Evicted details produce a generic Failed status,
    never Ready: failure always requires an explicit retry. *)
type 'data t

val max_queued : int
val max_running : int
val max_page_nodes : int
val max_error_details : int
val max_error_bytes : int
val create : 'data Tree.t -> 'data t
val snapshot : 'data t -> 'data Snapshot.t

(** [true] means enqueued. Duplicate requests, End, and failures without retry
    return [false]. Absent/leaf targets, closed controllers and admission overflow
    return errors without altering state. Retry only enqueues a failed branch. *)
val request : _ t -> Tree.Id.t -> bool Or_error.t

val retry : _ t -> Tree.Id.t -> bool Or_error.t

(** FIFO promotion, bounded by [max_running]. Only a promoted token may complete.
    No active request is started implicitly by requesting or completing another. *)
val take : _ t -> Request.t option

val running : _ t -> Request.t list
val is_current : _ t -> Request.t -> bool

(** Obsolete results are ignored before inspecting page contents. Current invalid
    pages fail atomically and set Failed. Pages append new children in display
    order. Empty pages must reach End or advance the cursor. *)
val complete : 'data t -> Request.t -> 'data Page.t -> Completion.t Or_error.t

val fail : _ t -> Request.t -> Error.t -> Completion.t

(** Cancel queued/running work and restore its source boundary. Failures and
    accepted child data are preserved. Late completions become obsolete. *)
val cancel : _ t -> Tree.Id.t -> unit

val cancel_subtree : _ t -> Tree.Id.t -> unit

(** Default collapse policy: cancel work for parents that are no longer visible
    and expanded in the reconciled UI state. Explicit prefetch can omit this call. *)
val cancel_hidden : _ t -> Tree_state.t -> unit

(** Explicitly discard failure and work when application load parameters change.
    A payload-only edit does not implicitly invalidate its branch's loader. *)
val invalidate : _ t -> Tree.Id.t -> unit

(** Adopt an update from the latest source lineage, preserving unrelated work.
    Revisions must advance (or the identical tree value is accepted). Changed
    parent incarnation/children revisions retire requests and failure markers.
    Use [reset] for a separately constructed lineage, regardless of matching IDs. *)
val update : 'data t -> 'data Tree.t -> unit Or_error.t

val reset : 'data t -> 'data Tree.t -> unit Or_error.t

(** Idempotent logical shutdown. The latest application tree stays available;
    work, failure details and producer eligibility are released immediately. *)
val close : _ t -> unit
