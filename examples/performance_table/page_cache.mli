open Core

(** Deterministic application-owned cache for the qualification workload.
    Logical keys and membership metadata are O(rows); only four adjacent pages
    retain payloads. Point updates preserve the Table_data source lineage. This
    is not the boundary-append Table_paging adapter or a remote I/O benchmark. *)
module Row : sig
  type t

  val number : t -> int
  val cell : t -> column:int -> string
end

type t

val page_size : int
val max_pages : int
val columns : int
val id : int -> Gpuio.Table_data.Id.t
val create : rows:int -> t Or_error.t
val data : t -> Row.t option Gpuio.Table_data.t

(** Replace the cache window with pages surrounding the target. Evict old
    payloads before loading new ones, including during a disjoint jump. *)
val prepare : t -> target:int -> t Or_error.t

val loaded_rows : t -> int
val peak_loaded_rows : t -> int
val loads : t -> int
val evictions : t -> int
