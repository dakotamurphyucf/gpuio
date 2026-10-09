open Core

(** Ordinary View content associated with stable radar-axis IDs. The collection
    never calls into OCaml from native layout, paint or measurement. *)
module Entry : sig
  type 'view t

  val create : axis:Chart_data.Datum_id.t -> 'view -> 'view t
  val axis : _ t -> Chart_data.Datum_id.t
  val content : 'view t -> 'view
end

type 'view t

(** At most 64 distinct axis IDs. IDs may be absent from the current dataset:
    such content is retained but hidden until its axis appears. Collection order
    is not content identity. Removing an entry unmounts its native content. *)
val create : 'view Entry.t list -> 'view t Or_error.t

val empty : 'view t

module Expert : sig
  val entries : 'view t -> 'view Entry.t list
  val axes : _ t -> int64 list
end
