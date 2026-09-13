open Core

(** Internal UI-domain owner of scoped encoded registrations. Rendering and
    decoding are separate. At most 1024 entries, 8 unfinished uploads and 64 MiB
    of OCaml source bytes; one request in flight, with cleanup taking priority. *)
type t

module Error : sig
  type t =
    | Closed
    | Not_ready
    | Resource_limit
    | Invalid_scope
    | Native_failure
  [@@deriving equal, sexp_of]
end

module Registration : sig
  type t

  (** Idempotent. Retires native acquisition asynchronously; existing native
      readers retain their own leases. Must run on the owning UI domain. *)
  val release : t -> unit

  val is_released : t -> bool

  module Expert : sig
    val native_id : t -> Gpuio_protocol.Resource_id.t option
  end
end

val create : scope:Scope.t -> wake:(unit -> unit) -> t

(** The callback runs on the UI loop after encoded publication or an error.
    Scope cancellation suppresses it while preserving internal late-reply cleanup.
    Source bytes are retained only until sent, failure or cancellation. *)
val register
  :  t
  -> scope:Scope.t
  -> Gpuio.Asset.Source.t
  -> on_result:((Registration.t, Error.t) Result.t -> unit)
  -> unit

(** Scheduler adapter. Call again after [complete]; never more than one request
    is outstanding. Reserve a transport lane so unrelated raw requests cannot
    prevent cleanup. Call [close] on terminal application disposal. *)
val next_request : t -> Gpuio_protocol.Wire.Asset.Request.t option

val complete : t -> Gpuio_protocol.Wire.Asset.Response.t -> unit
val close : t -> unit

module Expert : sig
  (** Entries, unfinished uploads and held source bytes. *)
  val counts : t -> int * int * int
end
