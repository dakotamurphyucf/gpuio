open Core

(** Scoped registration of immutable encoded assets. This module does not decode
    pixels. Acquire file/network bytes with explicit Eio capabilities, construct
    a [Source], then evaluate [register] on the UI loop. *)
module Source = Gpuio.Asset.Source

module Format = Gpuio.Asset.Format
module Error = Asset_registry.Error

type t = Asset_registry.Registration.t

(** Completes after the entire encoded source is published natively. At most
    eight uploads and 64 MiB of source bytes can be pending per application.
    [scope] must belong to [app]. Before application negotiation, returns
    [Not_ready]. Cancellation suppresses the user completion and releases any
    late allocation; a successful registration lives until release or scope end.
    Encoded publication is not successful image decoding. *)
val register : App.t -> scope:Scope.t -> Source.t -> (t, Error.t) Result.t Bonsai.Effect.t

(** Idempotent asynchronous retirement. Existing native readers keep their
    leases; new bindings must not use a released registration. *)
val release : t -> unit

val is_released : t -> bool

(** Immutable reference for pure image configurations. Available after successful
    registration, including after release; it does not extend the registration's
    lifetime. New bindings to a retired reference must fail locally. *)
val handle : t -> Gpuio.Asset.Handle.t

module Expert : sig
  (** Encoded identity for the owning app only. Not a portable or persistent ID.
      Returns [None] immediately upon release, before its acknowledgement. *)
  val native_id : t -> Gpuio_protocol.Resource_id.t option
end
