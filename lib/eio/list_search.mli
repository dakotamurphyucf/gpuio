open Core

module Status : sig
  type t =
    | Ready
    | Debouncing
    | Loading
    | Failed of Error.t
    | Cancelled
    | Closed
  [@@deriving sexp_of]
end

module Request : sig
  (** One immutable source/query snapshot. Producers may perform Eio I/O using
      explicit capabilities captured by [search]; never access Bonsai off-domain. *)
  type ('key, 'data, 'cmp) t

  val items : ('key, 'data, 'cmp) t -> ('key, 'data, 'cmp) Gpuio.List_collection.t
  val query : (_, _, _) t -> string
  val epoch : (_, _, _) t -> Gpuio.Key.t
end

module Page : sig
  (** [upsert] may update loaded records or append fetched records. Omitted loaded
      records remain available for hidden selection. [visible] is the exact result
      order, including any section decorations. Both lists must have unique keys;
      each visible key must exist after the merge. An invalid page applies nothing. *)
  type ('key, 'data) t =
    { upsert : ('key * 'data) list
    ; visible : 'key list
    }
end

module Snapshot : sig
  type ('key, 'data, 'cmp) t

  val items : ('key, 'data, 'cmp) t -> ('key, 'data, 'cmp) Gpuio.List_collection.t
  val source_id : (_, _, _) t -> Gpuio.List_collection.Source_id.t
  val query : (_, _, _) t -> string
  val epoch : (_, _, _) t -> Gpuio.Key.t
  val status : (_, _, _) t -> Status.t
  val visible : ('key, _, _) t -> 'key list
  val is_busy : (_, _, _) t -> bool

  (** Prior results remain visible while a query is pending/failed/cancelled.
      They are explicitly stale. Disable list input while stale unless the app
      deliberately permits interaction with results for the previous query. *)
  val is_stale : (_, _, _) t -> bool
end

(** One search owner outside transient row lifetimes. Debouncing and production
    use the supplied monotonic Eio clock and Scope. Superseding work is cancelled;
    query/source epochs also fence queued completions. No work starts at creation:
    the empty query initially exposes all loaded records with Ready status.

    Fetched records merge into the original collection, preserving surviving
    membership references and hidden preferences. There is no implicit eviction.
    [max_loaded] bounds both retained records and result metadata (default 100000,
    maximum 1000000). Applications can evict with [update_source]; removal/reinsert
    retires old targets. Payload byte size is application-owned, not bounded here.

    At most one current producer; cancelled fibers remain charged to Scope's task
    quota until they unwind. Admission failures become retryable Failed snapshots.
    [on_change] runs on the UI loop, must not block, and is additional to [value].
    Do not create/mutate controllers during Incremental graph evaluation. *)
type ('key, 'data, 'cmp) t

val create
  :  scope:Scope.t
  -> clock:_ Eio.Time.Mono.t
  -> ?debounce:Time_ns.Span.t
  -> ?max_loaded:int
  -> ?on_change:(('key, 'data, 'cmp) Snapshot.t -> unit Bonsai.Effect.t)
  -> ('key, 'data, 'cmp) Gpuio.List_collection.t
  -> search:(('key, 'data, 'cmp) Request.t -> ('key, 'data) Page.t Or_error.t)
  -> ('key, 'data, 'cmp) t Or_error.t

val snapshot : ('key, 'data, 'cmp) t -> ('key, 'data, 'cmp) Snapshot.t
val value : ('key, 'data, 'cmp) t -> ('key, 'data, 'cmp) Snapshot.t Bonsai.Cont.t

(** Stale source tokens ignore queued editor events. The same query is a no-op;
    it does not retry failures. UTF-8 queries are at most 4096 bytes without NUL.
    Feed committed editor observations; composing text must not start a search. *)
val set_query
  :  (_, _, _) t
  -> source:Gpuio.List_collection.Source_id.t
  -> string
  -> unit Or_error.t

(** Explicitly rerun the current query, even after a successful result. *)
val refresh : (_, _, _) t -> unit Or_error.t

(** Retry only a current Failed/Cancelled epoch; stale controls are ignored. *)
val retry : (_, _, _) t -> epoch:Gpuio.Key.t -> unit Or_error.t

val cancel : (_, _, _) t -> epoch:Gpuio.Key.t -> unit

(** Install an application-edited snapshot. A structural/independent source
    change always fences old work and starts a fresh search. A point-only update
    with [refresh=false] declares that search matching/order is unaffected, keeps
    its visible order/epoch and does not restart the producer. Such newer values
    win over upserts from the pending search's older snapshot. Defaults to true.
    Independent sources clear old results; same-source edits keep only surviving
    visible membership references until the new query completes. *)
val update_source
  :  ('key, 'data, 'cmp) t
  -> ?refresh:bool
  -> ('key, 'data, 'cmp) Gpuio.List_collection.t
  -> unit Or_error.t

(** Cancels owned work and queued delivery, unregisters cleanup and publishes
    Closed to [value]. Idempotent; parent-scope cancellation closes it too. *)
val close : (_, _, _) t -> unit
