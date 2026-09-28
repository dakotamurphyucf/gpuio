open Core

(** A scoped, retained native dataset. All operations run on the application's UI
    domain. Scope cancellation releases the registration; borrowed handles do not
    extend its lifetime. Registering a dataset does not itself mount a widget. *)
type t

module Error = Chart_registry.Error

(** Completes after the first native publication. Cancellation suppresses late
    delivery and cleans up an allocation even when its reply arrives afterward.
    Large datasets are decoded by bounded native background workers. *)
val create
  :  App.t
  -> scope:Scope.t
  -> Gpuio.Chart_data.t
  -> (t, Error.t) Result.t Bonsai.Effect.t

val handle : t -> Gpuio.Chart_resource.t
val data : t -> Gpuio.Chart_data.t option

(** The latest desired dataset has been accepted natively, not necessarily painted.
    False after release or while an update/reset awaits acceptance, including
    reverting to an accepted snapshot while an older upload is still in flight. *)
val is_published : t -> bool

(** Coalesces updates that have not started uploading. An in-flight publication
    finishes before uploading the latest desired snapshot. Acceptance here is
    local; native rejection is reported by [error] and keeps the prior published
    dataset. There is no automatic retry of a rejected snapshot: call [set] or
    [reset] again to retry. A successful publication clears the error. *)
val set : t -> Gpuio.Chart_data.t -> (unit, Error.t) Result.t

(** Retires the old selection epoch immediately. Native generation advances on
    successful publication. Coalesced resets never skip native generations.
    The registration's handle remains stable. *)
val reset : t -> Gpuio.Chart_data.t -> (unit, Error.t) Result.t

val error : t -> Error.t option
val release : t -> unit
val is_released : t -> bool
