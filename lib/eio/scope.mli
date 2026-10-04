open Core

(** Logical task lifetimes, owned by the UI domain. Scopes are independent of
    component visibility. Canceling a scope cancels descendants and suppresses
    their queued completions. External Eio cancellation remains cancellation. *)
type t

module Task : sig
  type t

  val cancel : t -> unit

  (** Whether the producer fiber has finished. Its result may still be queued;
      [cancel] also suppresses queued delivery after this becomes true. *)
  val is_finished : t -> bool
end

(** Names contain 1..256 bytes. At most 1024 active scopes, including the root,
    share one application tree. A closed parent or exhausted quota returns Error. *)
val child : t -> name:string -> t Or_error.t

val cancel : t -> unit
val is_active : t -> bool

module Stats : sig
  (** Counts shared by the entire application scope tree. Tasks remain counted
      until their producer fiber unwinds, including cancellation; queued result
      delivery is measured by the scheduler inbox instead. *)
  type t =
    { scopes : int
    ; tasks : int
    ; cleanups : int
    }
  [@@deriving sexp_of]
end

val stats : t -> Stats.t

(** Register window/application resource cleanup on the owning UI domain.
    It runs once, synchronously after the scope becomes inactive and its children
    are cancelled. The callback must not raise, block or perform I/O. Returns an
    idempotent unregister function; a closed scope or exhausted shared cleanup
    limit (4096 registrations) returns Error without registering the callback. *)
val on_cancel : t -> (unit -> unit) -> (unit -> unit) Or_error.t

(** [f] runs as an Eio fiber. Pass I/O capabilities in its closure. CPU work may
    use Eio's domain manager, but it must not access Bonsai from another domain.
    Ordinary producer exceptions become Error; external Eio cancellation is
    re-raised. [on_result] and its returned effect execute on the UI loop; their
    exceptions propagate to the runner rather than becoming producer errors.
    Scope/task cancellation suppresses results still waiting for delivery. *)
val start
  :  t
  -> f:(unit -> 'a)
  -> on_result:('a Or_error.t -> unit Bonsai.Effect.t)
  -> Task.t Or_error.t

module Expert : sig
  val create : sw:Eio.Switch.t -> inbox:Inbox.t -> max_tasks:int -> t

  (** Internal non-raising cleanup, called on scope cancellation. The returned
     function unregisters it. Registrations are bounded. *)
  val on_cancel : t -> (unit -> unit) -> (unit -> unit) Or_error.t

  val try_enqueue : t -> (unit -> unit) -> bool
  val enqueue : t -> (unit -> unit) -> unit
  val check : t -> unit

  (** Whether scopes share the same application scheduler and cancellation root. *)
  val same_tree : t -> t -> bool
end
