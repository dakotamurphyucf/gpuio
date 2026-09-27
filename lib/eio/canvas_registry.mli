open Core

(** Internal UI-domain scheduler. At most 256 registrations, one request in flight,
    at most four staged uploads, round-robin progress, cleanup first, and 64 MiB
    of charged snapshots/buffers.
    A scene's conservative charge includes its OCaml representation and one
    encoded buffer. This is retention accounting, not a process RSS limit. *)
type t

module Error : sig
  type t =
    | Native of Gpuio_protocol.Wire.Canvas.Error.t
    | Wrong_application
    | Wrong_scope
  [@@deriving equal, sexp_of]
end

module Registration : sig
  type t

  val handle : t -> Gpuio.Canvas_scene.Handle.t
  val scene : t -> Gpuio.Canvas_scene.t option
  val is_published : t -> bool
  val set : t -> Gpuio.Canvas_scene.t -> (unit, Error.t) Result.t
  val reset : t -> Gpuio.Canvas_scene.t -> (unit, Error.t) Result.t
  val error : t -> Error.t option
  val release : t -> unit
  val is_released : t -> bool
end

val create
  :  scope:Scope.t
  -> asset_owner:Gpuio.Asset.Expert.Owner.t
  -> wake:(unit -> unit)
  -> t

val register
  :  t
  -> scope:Scope.t
  -> Gpuio.Canvas_scene.t
  -> on_result:((Registration.t, Error.t) Result.t -> unit)
  -> unit

val next_request : t -> Gpuio_protocol.Wire.Canvas.Request.t option
val complete : t -> Gpuio_protocol.Wire.Canvas.Response.t -> unit

(** Fences released registrations, scene resets and unpublished revisions. A
    Publish acknowledgement may follow an event from that exact in-flight Publish. *)
val accepts_event
  :  t
  -> Gpuio_protocol.Resource_id.t option
  -> scene_revision:int64
  -> scene_generation:int64
  -> Gpuio_protocol.Canvas_view_wire.Observation.t
  -> bool

val close : t -> unit

module Expert : sig
  val owner : t -> Gpuio.Canvas_scene.Expert.Owner.t
  val counts : t -> int * int
end
