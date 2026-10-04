open Core

(** Bounded, application-owned history for the collection gallery. Splices add
    new stable identities; response fragments replace only the latest value. *)
type t

module Action : sig
  type t =
    | Append
    | Prepend
    | Stream_latest
    | Reset_latest
end

val initial : t
val apply : t -> Action.t -> t
val rows : t -> (int, string, Int.comparator_witness) Gpuio.List_collection.t
val first : t -> int
val last : t -> int
val streamed_lines : t -> int
val can_append : t -> bool
val can_prepend : t -> bool
