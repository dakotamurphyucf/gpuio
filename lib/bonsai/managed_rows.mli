open Core
module B = Bonsai.Cont

module Lifetime : sig
  (** One activation of a transient row. Re-visiting the same key creates a new
      lifetime; effects captured during its previous visit remain invalid. *)
  type t

  (** Check at execution time. For asynchronous work, guard its completion (the
      effect that injects the result), not just the effect starting the work.
      Cancel row-owned producers with the row's deactivation lifecycle too. *)
  val guard : t -> unit Bonsai.Effect.t -> unit Bonsai.Effect.t
end

(** The managed-list row primitive, also usable by custom native list adapters.
    Membership is the desired active set, including overscan and pinned rows.
    A removed row runs its deactivation hooks and resets its subtree model.

    The bounded-transient contract requires child state to use default-reset
    semantics. Custom reset handlers that keep non-default state violate this
    contract; store persistent state outside [assoc]. Delayed/external injects
    must use [Lifetime.guard] at delivery, and row-owned producers must be
    cancelled on deactivation. Ordinary accepted native callbacks are already
    generation checked by the view reconciler. Deletion and eviction both reset
    transient state; neither changes the application's source collection.

    Like any Bonsai computation, this does not bound application data or payloads
    retained by application-owned effects. Its memory guarantee applies to
    compliant row models, not arbitrary user retention outside this component. *)
val assoc
  :  ('key, 'cmp) B.comparator
  -> ('key, 'data, 'cmp) Map.t B.t
  -> f:('key B.t -> 'data B.t -> Lifetime.t B.t -> B.graph -> 'result B.t)
  -> B.graph
  -> ('key, 'result, 'cmp) Map.t B.t
