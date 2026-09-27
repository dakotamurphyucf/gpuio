open Core

module Row : sig
  type t

  val number : t -> int
  val tool : t -> string
  val score : t -> int
  val summary : t -> string
end

module Query : sig
  module Size : sig
    type t =
      | Sample
      | Large
    [@@deriving equal, sexp_of]
  end

  module Filter : sig
    type t =
      | All
      | High_score
      | Between of Score_range.t
      | Empty
    [@@deriving equal, sexp_of]
  end

  module Loading : sig
    type t =
      | Normal
      | Slow
      | Fail_once
    [@@deriving equal, sexp_of]
  end

  type t

  val create
    :  ?size:Size.t
    -> ?filter:Filter.t
    -> ?sort:Gpuio.Table.Sort.t
    -> ?loading:Loading.t
    -> unit
    -> t Or_error.t

  val size : t -> Size.t
  val filter : t -> Filter.t
  val sort : t -> Gpuio.Table.Sort.t option
  val loading : t -> Loading.t
  val with_sort : t -> Gpuio.Table.Sort.t option -> t Or_error.t
  val with_filter : t -> Filter.t -> t
  val with_size : t -> Size.t -> t
end

val id : int -> Gpuio.Table_data.Id.t
val column : string -> Gpuio.Table_column.Id.t
val columns : unit -> Gpuio.Table_column.Collection.t
val schema : Gpuio.Table_column.t list -> Gpuio.Table_column.Collection.t Or_error.t

(** Full query ordering, including a stable number tie-breaker, precedes paging.
    Suitable for a worker domain; no mutable runtime state or I/O. *)
val rows : Query.t -> (Gpuio.Table_data.Id.t * Row.t) list

(** Preserve membership for surviving keys when accepting a complete result set. *)
val replace : Row.t Gpuio.Table_data.t -> Query.t -> Row.t Gpuio.Table_data.t Or_error.t

(** A page of at most 24 rows from the globally ordered query. Invalid cursors
    return Error; [Before] is unsupported by this forward-only sample. *)
val page
  :  Query.t Gpuio.Table_paging.Request.t
  -> Row.t Gpuio_eio.Table_paging.Page.t Or_error.t

val cell : Row.t -> Gpuio.Table_column.Id.t -> string Or_error.t
val describe : Row.t -> string
