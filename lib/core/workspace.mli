open Core

(** Immutable per-window tab order and selection. Payloads belong to the
    application: switching, reordering or removing a tab never cancels tasks.
    Share a conversation ID rather than copying its store when opening another
    window. Keep draft/scroll models in payloads or an explicit shared store. *)
module Id = Choice.Id

module Tab : sig
  type 'a t

  val create : id:Id.t -> label:string -> 'a -> 'a t Or_error.t
  val id : _ t -> Id.t
  val label : _ t -> string
  val data : 'a t -> 'a
  val with_data : 'a t -> 'a -> 'a t
  val with_label : 'a t -> string -> 'a t Or_error.t
end

type 'a t

val max_tabs : int
val empty : 'a t
val create : ?active:Id.t -> 'a Tab.t list -> 'a t Or_error.t
val tabs : 'a t -> 'a Tab.t list
val active : 'a t -> 'a Tab.t option
val find : 'a t -> Id.t -> 'a Tab.t option
val select : 'a t -> Id.t -> 'a t Or_error.t
val next : 'a t -> 'a t
val previous : 'a t -> 'a t
val add : 'a t -> ?activate:bool -> 'a Tab.t -> 'a t Or_error.t

(** Removing the active tab selects its next neighbor, or previous at the end.
    The removed payload is returned for explicit application lifecycle policy. *)
val remove : 'a t -> Id.t -> ('a t * 'a Tab.t) Or_error.t

(** Destination is a zero-based position in the resulting order. *)
val move : 'a t -> Id.t -> index:int -> 'a t Or_error.t

val update : 'a t -> Id.t -> f:('a -> 'a) -> 'a t Or_error.t
val rename : 'a t -> Id.t -> label:string -> 'a t Or_error.t
val choices : _ t -> label:string -> Choice.Config.t Or_error.t
