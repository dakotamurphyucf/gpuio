open Core

(** Scoped display resource and streaming controller. All operations belong to
    the application's UI domain; Eio producer fibers on that domain may append.
    The conversation scope, not visibility, determines this resource's lifetime. *)
type t

module Error = Gpuio_protocol.Document_wire.Error

(** Completes after the initial snapshot is published. Scope cancellation
    suppresses late delivery and retires native resources. *)
val create
  :  App.t
  -> scope:Scope.t
  -> Gpuio.Text_source.t
  -> (t, Error.t) Result.t Bonsai.Effect.t

val handle : t -> Gpuio.Text_source.Handle.t
val source : t -> Gpuio.Text_source.t option

(** True once the latest desired snapshot is accepted natively. This is not
    parser completion or physical presentation. False after release. *)
val is_published : t -> bool

val error : t -> Error.t option
val release : t -> unit

(** Setters accept a coalesced desired snapshot; native publication is async.
    The latest terminal state is always delivered with its exact content. *)
val append : t -> string -> unit Or_error.t

(** Byte chunks may split Unicode scalars; at most three bytes are buffered.
    An error accepts neither the chunk nor a partial prefix of that chunk. *)
val push_bytes : t -> string -> unit Or_error.t

val pending_bytes : t -> int
val finish : t -> unit Or_error.t

(** Cancellation drops an incomplete trailing scalar, if any, and publishes
    [Cancelled] with all previously accepted complete Unicode content. *)
val cancel : t -> unit Or_error.t

val reset : t -> string -> unit Or_error.t
val edit : t -> first:int -> last:int -> text:string -> unit Or_error.t
