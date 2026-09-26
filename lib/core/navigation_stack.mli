open Core

(** Bounded, immutable application navigation history, independent of native
    transitions and workspace tabs. Payloads and their task lifetimes belong to
    the application. Removing/hiding an entry does not cancel an Eio task. *)
module Id : sig
  (** One navigation instance, not a route name. Multiple visits to the same
      route use distinct IDs. Nonempty UTF-8 without NUL, at most 256 bytes. *)
  type t [@@deriving compare, equal, sexp_of]

  val of_string : string -> t Or_error.t
  val to_string : t -> string
end

module Entry : sig
  type 'a t

  (** Labels are nonempty UTF-8 without NUL, at most 4096 bytes. *)
  val create : id:Id.t -> label:string -> 'a -> 'a t Or_error.t

  val id : _ t -> Id.t
  val label : _ t -> string
  val data : 'a t -> 'a
  val with_data : 'a t -> 'a -> 'a t
  val with_label : 'a t -> string -> 'a t Or_error.t
end

type 'a t

(** Total bound including forward entries. Operations/enumeration are O(n) in
    this bounded history. No payload is copied, serialized or implicitly closed. *)
val max_entries : int

val empty : 'a t
val singleton : 'a Entry.t -> 'a t

(** Root first; defaults to the last entry as current. [current] must be a
    member; entries following it become forward history. Reject duplicates. *)
val create : ?current:Id.t -> 'a Entry.t list -> 'a t Or_error.t

val entries : 'a t -> 'a Entry.t list
val back_entries : 'a t -> 'a Entry.t list
val current : 'a t -> 'a Entry.t option
val forward_entries : 'a t -> 'a Entry.t list
val find : 'a t -> Id.t -> 'a Entry.t option
val can_pop : _ t -> bool
val can_forward : _ t -> bool

(** Discard the forward branch; [entry]'s ID must be absent from the whole
    existing history, including that branch. Limits apply to resulting history. *)
val push : 'a t -> 'a Entry.t -> 'a t Or_error.t

(** Pop protects the root. Forward at the end and pop at the root/empty stack
    are no-ops. Popped entries remain available to [forward]. *)
val pop : 'a t -> 'a t

val pop_to_root : 'a t -> 'a t
val forward : 'a t -> 'a t

(** Preserve forward entries. An empty stack becomes a singleton. The new ID
    may equal the current ID, but must not name any other existing entry. *)
val replace : 'a t -> 'a Entry.t -> 'a t Or_error.t

(** Return to an entry on the back/current path; later entries move into forward
    history. Reject absent/forward-only IDs, including stale breadcrumb requests. *)
val pop_to : 'a t -> Id.t -> 'a t Or_error.t

val update : 'a t -> Id.t -> f:('a -> 'a) -> 'a t Or_error.t
val clear : 'a t -> 'a t

module Motion : sig
  (** Native presentation policy. Default is a 200 ms horizontal slide; replacement
      at the same history position fades. First placement is immediate. Reduced
      motion settles immediately. Timing never runs an OCaml callback per frame.
      This policy is for the navigation presenter under implementation. *)
  type t [@@deriving equal, sexp_of]

  val default : t
  val immediate : t

  (** Finite duration in [0,10] seconds, rounded up to whole milliseconds.
      Zero duration settles immediately. *)
  val slide : Time_ns.Span.t -> t Or_error.t

  val fade : Time_ns.Span.t -> t Or_error.t
end

module Expert : sig
  val motion_config
    :  Motion.t
    -> selected:int64 option
    -> hidden:Content_policy.t
    -> Gpuio_protocol.Navigation_stack_wire.Config.t

  val presentation_config
    :  _ t
    -> hidden:Content_policy.t
    -> motion:Motion.t
    -> Gpuio_protocol.Navigation_stack_wire.Config.t
end
