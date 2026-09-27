open Core

(** A scoped, retained native scene. All operations run on the application's UI
    domain. Scope cancellation releases the registration; borrowed handles do not
    extend its lifetime. Registering a scene does not itself mount a widget. *)
type t

module Error = Canvas_registry.Error

(** Completes after the first native publication. Cancellation suppresses late
    delivery and cleans up an allocation even when its reply arrives afterward.
    Image handles must belong to [app] and be available during publication. *)
val create
  :  App.t
  -> scope:Scope.t
  -> Gpuio.Canvas_scene.t
  -> (t, Error.t) Result.t Bonsai.Effect.t

val handle : t -> Gpuio.Canvas_scene.Handle.t
val scene : t -> Gpuio.Canvas_scene.t option

(** The latest desired scene has been accepted natively, not necessarily painted.
    False after release or while an update/reset awaits acceptance, including
    reverting to an accepted snapshot while an older upload is still in flight. *)
val is_published : t -> bool

(** Coalesces updates that have not started uploading. An in-flight publication
    finishes before uploading the latest desired snapshot. Acceptance here is
    local; native rejection is reported by [error] and keeps the prior published
    scene. There is no automatic retry of a rejected snapshot: call [set] or
    [reset] again to retry. A successful publication clears the error. *)
val set : t -> Gpuio.Canvas_scene.t -> (unit, Error.t) Result.t

(** Starts a new resource generation, clearing native resource history on
    successful publication. Coalesced resets never skip native generations.
    The registration's handle remains stable. *)
val reset : t -> Gpuio.Canvas_scene.t -> (unit, Error.t) Result.t

val error : t -> Error.t option
val release : t -> unit
val is_released : t -> bool
