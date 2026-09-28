open Core

(** UI-domain dataset scheduler. At most 256 registrations, four staged uploads,
    one correlated request in flight and 128 MiB conservatively charged data and
    encode buffers. Cleanup precedes round-robin progress. The native host owns
    decoding/publication and must return exactly one response for each request. *)
type t

module Error : sig
  type t =
    | Native of Gpuio_protocol.Chart_resource_wire.Error.t
    | Wrong_scope
  [@@deriving equal, sexp_of]
end

module Registration : sig
  type t

  (** Available after the initial publication callback succeeds. *)
  val handle : t -> Gpuio.Chart_resource.t

  val data : t -> Gpuio.Chart_data.t option
  val is_published : t -> bool

  (** Coalesces unstarted updates. In-flight updates finish before the latest
      desired value uploads. Local acceptance does not imply native publication.
      Rejection preserves the last published data and requires explicit retry. *)
  val set : t -> Gpuio.Chart_data.t -> (unit, Error.t) Result.t

  (** Retires selection identity immediately; native generation advances only on
      successful publication. Coalesced resets cannot skip generations. *)
  val reset : t -> Gpuio.Chart_data.t -> (unit, Error.t) Result.t

  val error : t -> Error.t option
  val release : t -> unit
  val is_released : t -> bool
end

val create : scope:Scope.t -> wake:(unit -> unit) -> t

(** Callback after initial publication or failure. Cancelling the scope suppresses
    late delivery and schedules release even if Create's response arrives late. *)
val register
  :  t
  -> scope:Scope.t
  -> Gpuio.Chart_data.t
  -> on_result:((Registration.t, Error.t) Result.t -> unit)
  -> unit

val next_request : t -> Gpuio_protocol.Chart_resource_wire.Request.t option
val complete : t -> Gpuio_protocol.Chart_resource_wire.Response.t -> unit

(** Application shutdown: the host must also close its native resource store.
    Registration release while the application is alive uses Release requests. *)
val close : t -> unit

(** Fences semantic events against the accepted/currently publishing revision,
    logical reset and scope release. Payload validation belongs to the widget. *)
val accepts_revision
  :  t
  -> Gpuio_protocol.Resource_id.t
  -> revision:int64
  -> generation:int64
  -> bool

(** Validates view observations; pre-publication failures still require a live
    registration. Released scopes and logical resets suppress stale events. *)
val accepts_event
  :  t
  -> Gpuio_protocol.Resource_id.t option
  -> data_revision:int64
  -> data_generation:int64
  -> Gpuio_protocol.Chart_view_wire.Observation.t
  -> bool

module Expert : sig
  val owner : t -> Gpuio.Chart_resource.Expert.Owner.t
  val counts : t -> int * int
end
