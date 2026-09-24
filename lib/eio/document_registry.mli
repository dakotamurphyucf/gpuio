open Core

(** Internal UI-domain owner of document registrations. One transport request
    in flight, round-robin progress between sources, cleanup before uploads.
    At most 1024 entries and 64 MiB conservatively charged canonical snapshots.
    Setters coalesce to the latest desired snapshot without a chunk queue. *)
type t

module Registration : sig
  type t

  val handle : t -> Gpuio.Text_source.Handle.t
  val source : t -> Gpuio.Text_source.t option
  val is_published : t -> bool

  (** Same source identity required. Generation reset is accepted. Expected
      failures return a typed native error; a released registration is closed.
      Accepted locally does not mean published natively. *)
  val set
    :  t
    -> Gpuio.Text_source.t
    -> (unit, Gpuio_protocol.Wire.Document.Error.t) Result.t

  val error : t -> Gpuio_protocol.Wire.Document.Error.t option
  val release : t -> unit
  val is_released : t -> bool
end

val create : scope:Scope.t -> wake:(unit -> unit) -> t

val register
  :  t
  -> scope:Scope.t
  -> Gpuio.Text_source.t
  -> on_result:((Registration.t, Gpuio_protocol.Wire.Document.Error.t) Result.t -> unit)
  -> unit

val next_request : t -> Gpuio_protocol.Wire.Document.Request.t option
val complete : t -> Gpuio_protocol.Wire.Document.Response.t -> unit
val close : t -> unit

module Expert : sig
  val owner : t -> Gpuio.Text_source.Expert.Owner.t
  val counts : t -> int * int
end

(** Suppress queued native navigation as soon as the owning scope is released
    or OCaml resets the source, including while that reset awaits upload. *)
val accepts_navigation : t -> Gpuio_protocol.Resource_id.t -> generation:int64 -> bool
